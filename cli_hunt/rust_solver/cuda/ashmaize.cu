#include "ashmaize_vm.cuh"

__device__ void execute_one_instruction(VMState &vm, const uint8_t *rom,
                                       const uint8_t *prog_chunk, uint32_t rom_size) {
    Instruction instr = decode_instruction(prog_chunk);

    uint64_t src1, src2;

    if (instr.op1 < 5) {
        src1 = vm.regs[instr.r1];
    } else if (instr.op1 < 9) {
        src1 = mem_access64(vm, rom, instr.lit1, rom_size);
    } else if (instr.op1 < 13) {
        src1 = instr.lit1;
    } else if (instr.op1 < 14) {
        src1 = special1_value64(vm.prog_digest_state);
    } else {
        src1 = special2_value64(vm.mem_digest_state);
    }

    if (instr.op2 < 5) {
        src2 = vm.regs[instr.r2];
    } else if (instr.op2 < 9) {
        src2 = mem_access64(vm, rom, instr.lit2, rom_size);
    } else if (instr.op2 < 13) {
        src2 = instr.lit2;
    } else if (instr.op2 < 14) {
        src2 = special1_value64(vm.prog_digest_state);
    } else {
        src2 = special2_value64(vm.mem_digest_state);
    }

    uint64_t result;

    if (instr.opcode < 40) {
        result = src1 + src2;
    } else if (instr.opcode < 80) {
        result = src1 * src2;
    } else if (instr.opcode < 96) {
        uint64_t a_lo = src1 & 0xFFFFFFFF;
        uint64_t a_hi = src1 >> 32;
        uint64_t b_lo = src2 & 0xFFFFFFFF;
        uint64_t b_hi = src2 >> 32;

        uint64_t p0 = a_lo * b_lo;
        uint64_t p1 = a_lo * b_hi;
        uint64_t p2 = a_hi * b_lo;
        uint64_t p3 = a_hi * b_hi;

        uint64_t carry = ((p0 >> 32) + (p1 & 0xFFFFFFFF) + (p2 & 0xFFFFFFFF)) >> 32;
        result = p3 + (p1 >> 32) + (p2 >> 32) + carry;

    } else if (instr.opcode < 112) {
        result = (src2 != 0) ? src1 / src2 : special1_value64(vm.prog_digest_state);
    } else if (instr.opcode < 128) {
        result = (src2 != 0) ? src1 / src2 : special1_value64(vm.prog_digest_state);
    } else if (instr.opcode < 138) {
        result = isqrt_64(src1);
    } else if (instr.opcode < 148) {
        result = 0;
        uint64_t temp = src1;
        for (int i = 0; i < 64; i++) {
            result = (result << 1) | (temp & 1);
            temp >>= 1;
        }
    } else if (instr.opcode < 188) {
        result = src1 ^ src2;
    } else if (instr.opcode < 204) {
        result = (src1 << (instr.r1 & 0x3F)) | (src1 >> (64 - (instr.r1 & 0x3F)));
    } else if (instr.opcode < 220) {
        result = (src1 >> (instr.r1 & 0x3F)) | (src1 << (64 - (instr.r1 & 0x3F)));
    } else if (instr.opcode < 240) {
        result = ~src1;
    } else if (instr.opcode < 248) {
        result = src1 & src2;
    } else {
        uint8_t input[16];
        for (int i = 0; i < 8; ++i) {
            input[i] = (src1 >> (i * 8)) & 0xFF;
        }
        for (int i = 0; i < 8; ++i) {
            input[8 + i] = (src2 >> (i * 8)) & 0xFF;
        }
        
        uint8_t hash_out[64];
        blake2b(hash_out, 64, input, 16);
        
        result = ((uint64_t)hash_out[0] << 0) |
                 ((uint64_t)hash_out[1] << 8) |
                 ((uint64_t)hash_out[2] << 16) |
                 ((uint64_t)hash_out[3] << 24) |
                 ((uint64_t)hash_out[4] << 32) |
                 ((uint64_t)hash_out[5] << 40) |
                 ((uint64_t)hash_out[6] << 48) |
                 ((uint64_t)hash_out[7] << 56);
    }

    vm.regs[instr.r3] = result;
    vm.ip = vm.ip + 1;
}

__global__ void ashmaize_hash_kernel(
    const uint8_t* rom_data,
    uint32_t rom_size,
    const uint8_t* rom_digest,
    const uint8_t* salts,
    uint32_t salt_len,
    const uint8_t* initial_prog_seeds,
    uint8_t* programs,
    uint32_t nb_loops,
    uint32_t nb_instrs,
    uint32_t program_size,
    uint8_t* results,
    uint32_t num_hashes
) {
    uint32_t tid = blockIdx.x * blockDim.x + threadIdx.x;
    
    if (tid >= num_hashes) {
        return;
    }

    const uint8_t* salt = salts + tid * 32;
    uint8_t* program = programs + tid * program_size;
    uint8_t* result = results + tid * 64;

    VMState vm;
    vm_init(vm, rom_digest, 64, salt, salt_len);

    for (uint32_t loop = 0; loop < nb_loops; ++loop) {
        hprime(program, program_size, vm.prog_seed, 64);

        for (uint32_t instr_idx = 0; instr_idx < nb_instrs; ++instr_idx) {
            execute_one_instruction(vm, rom_data, program + instr_idx * INSTR_SIZE, rom_size);
        }

        post_instructions(vm);
    }

    Blake2bState final_prog_state = vm.prog_digest_state;
    uint8_t prog_final[64];
    blake2b_final(final_prog_state, prog_final, 64);

    Blake2bState final_mem_state = vm.mem_digest_state;
    uint8_t mem_final[64];
    blake2b_final(final_mem_state, mem_final, 64);

    uint8_t combined[128];
    for (int i = 0; i < 64; ++i) {
        combined[i] = prog_final[i];
        combined[64 + i] = mem_final[i];
    }

    blake2b(result, 64, combined, 128);
}

