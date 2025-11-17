#ifndef ASHMAIZE_VM_CUH
#define ASHMAIZE_VM_CUH

#include "blake2b.cuh"
#include "argon2.cuh"

#define INSTR_SIZE 20
#define NB_REGS 32
#define REGS_BITS 5
#define REGS_INDEX_MASK 31
#define REGISTER_SIZE 8

#define DIGEST_INIT_SIZE 64
#define REGS_CONTENT_SIZE 256

struct VMState {
    uint64_t regs[NB_REGS];
    uint32_t ip;
    uint8_t prog_seed[64];
    uint32_t memory_counter;
    uint32_t loop_counter;
    Blake2bState prog_digest_state;
    Blake2bState mem_digest_state;
};

enum Op3Type {
    Add = 0,
    Mul = 1,
    MulH = 2,
    Xor = 3,
    Div = 4,
    Mod = 5,
    And = 6,
    Hash = 7
};

enum Op2Type {
    ISqrt = 0,
    Neg = 1,
    BitRev = 2,
    RotL = 3,
    RotR = 4
};

struct Instruction {
    uint8_t opcode;
    uint8_t op1, op2;
    uint8_t r1, r2, r3;
    uint64_t lit1, lit2;
};

__device__ void vm_init(VMState &vm, const uint8_t *rom_digest, uint32_t rom_digest_len,
                       const uint8_t *salt, uint32_t salt_len) {
    uint8_t init_buffer[REGS_CONTENT_SIZE + 3 * DIGEST_INIT_SIZE];

    uint8_t init_buffer_input[DIGEST_INIT_SIZE + 512];
    for (uint32_t i = 0; i < rom_digest_len; ++i) {
        init_buffer_input[i] = rom_digest[i];
    }
    for (uint32_t i = 0; i < salt_len; ++i) {
        init_buffer_input[rom_digest_len + i] = salt[i];
    }

    hprime(init_buffer, REGS_CONTENT_SIZE + 3 * DIGEST_INIT_SIZE,
           init_buffer_input, rom_digest_len + salt_len);

    const uint8_t *init_buffer_regs = init_buffer;
    for (uint32_t i = 0; i < NB_REGS; ++i) {
        vm.regs[i] = ((uint64_t)init_buffer_regs[i * 8 + 0] << 0) |
                     ((uint64_t)init_buffer_regs[i * 8 + 1] << 8) |
                     ((uint64_t)init_buffer_regs[i * 8 + 2] << 16) |
                     ((uint64_t)init_buffer_regs[i * 8 + 3] << 24) |
                     ((uint64_t)init_buffer_regs[i * 8 + 4] << 32) |
                     ((uint64_t)init_buffer_regs[i * 8 + 5] << 40) |
                     ((uint64_t)init_buffer_regs[i * 8 + 6] << 48) |
                     ((uint64_t)init_buffer_regs[i * 8 + 7] << 56);
    }

    const uint8_t *digests_data = init_buffer + REGS_CONTENT_SIZE;

    blake2b_init(vm.prog_digest_state, 64);
    blake2b_update(vm.prog_digest_state, digests_data, 64);

    blake2b_init(vm.mem_digest_state, 64);
    blake2b_update(vm.mem_digest_state, &digests_data[64], 64);

    for (int i = 0; i < 64; ++i) {
        vm.prog_seed[i] = digests_data[128 + i];
    }

    vm.ip = 0;
    vm.loop_counter = 0;
    vm.memory_counter = 0;
}

__device__ uint64_t special1_value64(Blake2bState &prog_digest_state) {
    Blake2bState temp_state = prog_digest_state;
    uint8_t out[BLAKE2B_OUTBYTES];
    blake2b_final(temp_state, out, BLAKE2B_OUTBYTES);
    return ((uint64_t)out[0] << 0) |
           ((uint64_t)out[1] << 8) |
           ((uint64_t)out[2] << 16) |
           ((uint64_t)out[3] << 24) |
           ((uint64_t)out[4] << 32) |
           ((uint64_t)out[5] << 40) |
           ((uint64_t)out[6] << 48) |
           ((uint64_t)out[7] << 56);
}

__device__ uint64_t special2_value64(Blake2bState &mem_digest_state) {
    Blake2bState temp_state = mem_digest_state;
    uint8_t out[BLAKE2B_OUTBYTES];
    blake2b_final(temp_state, out, BLAKE2B_OUTBYTES);
    return ((uint64_t)out[0] << 0) |
           ((uint64_t)out[1] << 8) |
           ((uint64_t)out[2] << 16) |
           ((uint64_t)out[3] << 24) |
           ((uint64_t)out[4] << 32) |
           ((uint64_t)out[5] << 40) |
           ((uint64_t)out[6] << 48) |
           ((uint64_t)out[7] << 56);
}

__device__ uint64_t mem_access64(VMState &vm, const uint8_t *rom, uint64_t addr, uint32_t rom_size) {
    uint32_t rom_addr = addr % rom_size;
    uint32_t chunk_start_idx = (rom_addr / 64) * 64;
    uint8_t mem_chunk[64];

    for (uint32_t i = 0; i < 64; ++i) {
        mem_chunk[i] = rom[(chunk_start_idx + i) % rom_size];
    }

    blake2b_update(vm.mem_digest_state, mem_chunk, 64);
    vm.memory_counter++;

    uint32_t idx_in_chunk = ((vm.memory_counter % 8)) * 8;
    uint64_t result = ((uint64_t)mem_chunk[idx_in_chunk + 0] << 0) |
                      ((uint64_t)mem_chunk[idx_in_chunk + 1] << 8) |
                      ((uint64_t)mem_chunk[idx_in_chunk + 2] << 16) |
                      ((uint64_t)mem_chunk[idx_in_chunk + 3] << 24) |
                      ((uint64_t)mem_chunk[idx_in_chunk + 4] << 32) |
                      ((uint64_t)mem_chunk[idx_in_chunk + 5] << 40) |
                      ((uint64_t)mem_chunk[idx_in_chunk + 6] << 48) |
                      ((uint64_t)mem_chunk[idx_in_chunk + 7] << 56);

    return result;
}

__device__ Instruction decode_instruction(const uint8_t *instruction) {
    Instruction instr;
    instr.opcode = instruction[0];
    instr.op1 = instruction[1] >> 4;
    instr.op2 = instruction[1] & 0x0F;

    uint16_t rs = ((uint16_t)instruction[2] << 8) | instruction[3];
    instr.r1 = (rs >> (2 * REGS_BITS)) & REGS_INDEX_MASK;
    instr.r2 = (rs >> REGS_BITS) & REGS_INDEX_MASK;
    instr.r3 = rs & REGS_INDEX_MASK;

    instr.lit1 = ((uint64_t)instruction[4] << 0) |
                 ((uint64_t)instruction[5] << 8) |
                 ((uint64_t)instruction[6] << 16) |
                 ((uint64_t)instruction[7] << 24) |
                 ((uint64_t)instruction[8] << 32) |
                 ((uint64_t)instruction[9] << 40) |
                 ((uint64_t)instruction[10] << 48) |
                 ((uint64_t)instruction[11] << 56);

    instr.lit2 = ((uint64_t)instruction[12] << 0) |
                 ((uint64_t)instruction[13] << 8) |
                 ((uint64_t)instruction[14] << 16) |
                 ((uint64_t)instruction[15] << 24) |
                 ((uint64_t)instruction[16] << 32) |
                 ((uint64_t)instruction[17] << 40) |
                 ((uint64_t)instruction[18] << 48) |
                 ((uint64_t)instruction[19] << 56);

    return instr;
}

__device__ uint64_t isqrt_64(uint64_t x) {
    if (x < 2) {
        return x;
    }
    uint64_t y = (x + 1) / 2;
    while (y < x) {
        x = y;
        y = (x + x / x) / 2;
    }
    return x;
}

__device__ void post_instructions(VMState &vm) {
    uint64_t sum = 0;
    for (int i = 0; i < NB_REGS; ++i) {
        sum = sum + vm.regs[i];
    }

    uint8_t sum_bytes[8];
    sum_bytes[0] = (uint8_t)(sum >> 0);
    sum_bytes[1] = (uint8_t)(sum >> 8);
    sum_bytes[2] = (uint8_t)(sum >> 16);
    sum_bytes[3] = (uint8_t)(sum >> 24);
    sum_bytes[4] = (uint8_t)(sum >> 32);
    sum_bytes[5] = (uint8_t)(sum >> 40);
    sum_bytes[6] = (uint8_t)(sum >> 48);
    sum_bytes[7] = (uint8_t)(sum >> 56);

    blake2b_update(vm.prog_digest_state, sum_bytes, 8);

    Blake2bState temp_state = vm.prog_digest_state;
    uint8_t digest_out[64];
    blake2b_final(temp_state, digest_out, 64);

    for (int i = 0; i < 64; ++i) {
        vm.prog_seed[i] = digest_out[i];
    }

    vm.loop_counter++;
    vm.ip = 0;
}

#endif

