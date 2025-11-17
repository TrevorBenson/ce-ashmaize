#include "ashmaize_vm.cuh"

// Shared Memory ROM Caching Kernel
// Cache first 16KB of ROM in shared memory for faster access

#define SHARED_ROM_SIZE 16384  // 16KB shared memory cache

__device__ void execute_one_instruction_shared_rom(VMState &vm, const uint8_t *rom,
                                       const uint8_t *prog_chunk, uint32_t rom_size,
                                       const uint8_t *shared_rom, bool debug = false) {
    Instruction instr = decode_instruction(prog_chunk);

    uint64_t src1, src2;

    bool debug_mem = debug;
    
    // Always evaluate src1
    if (instr.op1 < 5) {
        src1 = vm.regs[instr.r1];
    } else if (instr.op1 < 9) {
        // Memory access - try shared ROM first
        uint64_t addr = instr.lit1;
        uint64_t byte_offset = addr;  // Bug #11: chunk index as byte offset
        
        if (byte_offset + 64 <= SHARED_ROM_SIZE) {
            // Access from shared memory
            const uint8_t* chunk_ptr = shared_rom + byte_offset;
            blake2b_update(vm.mem_digest_state, chunk_ptr, 64);
            uint32_t word_offset = vm.memory_counter % 8;
            src1 = ((uint64_t*)chunk_ptr)[word_offset];
        } else if (byte_offset + 64 <= rom_size) {
            // Fall back to global memory
            const uint8_t* chunk_ptr = rom + byte_offset;
            blake2b_update(vm.mem_digest_state, chunk_ptr, 64);
            uint32_t word_offset = vm.memory_counter % 8;
            src1 = ((uint64_t*)chunk_ptr)[word_offset];
        } else {
            src1 = 0;
        }
        vm.memory_counter++;
    } else if (instr.op1 < 13) {
        src1 = instr.lit1;
    } else if (instr.op1 < 14) {
        src1 = special1_value64(vm.prog_digest_state);
    } else {
        src1 = special2_value64(vm.mem_digest_state);
    }

    // Only evaluate src2 for Op3 instructions
    bool is_op2 = (instr.opcode >= 128 && instr.opcode < 148) ||
                  (instr.opcode >= 188 && instr.opcode < 240);
    bool is_op3 = !is_op2;
    if (is_op3) {
        if (instr.op2 < 5) {
            src2 = vm.regs[instr.r2];
        } else if (instr.op2 < 9) {
            // Memory access - try shared ROM first
            uint64_t addr = instr.lit2;
            uint64_t byte_offset = addr;  // Bug #11
            
            if (byte_offset + 64 <= SHARED_ROM_SIZE) {
                // Access from shared memory
                const uint8_t* chunk_ptr = shared_rom + byte_offset;
                blake2b_update(vm.mem_digest_state, chunk_ptr, 64);
                uint32_t word_offset = vm.memory_counter % 8;
                src2 = ((uint64_t*)chunk_ptr)[word_offset];
            } else if (byte_offset + 64 <= rom_size) {
                // Fall back to global memory
                const uint8_t* chunk_ptr = rom + byte_offset;
                blake2b_update(vm.mem_digest_state, chunk_ptr, 64);
                uint32_t word_offset = vm.memory_counter % 8;
                src2 = ((uint64_t*)chunk_ptr)[word_offset];
            } else {
                src2 = 0;
            }
            vm.memory_counter++;
        } else if (instr.op2 < 13) {
            src2 = instr.lit2;
        } else if (instr.op2 < 14) {
            src2 = special1_value64(vm.prog_digest_state);
        } else {
            src2 = special2_value64(vm.mem_digest_state);
        }
    } else {
        src2 = 0;
    }

    // Execute instruction
    uint64_t result;
    if (instr.opcode < 32) {
        result = src1 + src2;
    }
    else if (instr.opcode < 64) {
        result = src1 ^ src2;
    }
    else if (instr.opcode < 96) {
        result = src1 * src2;
    }
    else if (instr.opcode < 112) {
        if (src2 != 0) {
            result = src1 / src2;
        } else {
            result = src1;
        }
    }
    else if (instr.opcode < 128) {
        if (src2 != 0) {
            result = src1 / src2;  // Bug #14: CPU does division for modulo
        } else {
            result = src1;
        }
    }
    else if (instr.opcode < 144) {
        uint32_t shift = src2 % 64;
        if (shift == 0) {
            result = src1;
        } else {
            result = (src1 << shift) | (src1 >> (64 - shift));
        }
    }
    else if (instr.opcode < 160) {
        uint32_t shift = src2 % 64;
        if (shift == 0) {
            result = src1;
        } else {
            result = (src1 >> shift) | (src1 << (64 - shift));
        }
    }
    else if (instr.opcode < 208) {
        result = isqrt_64(src1);
    }
    else {
        // Memory access - try shared ROM first
        uint64_t addr = src1;
        uint64_t byte_offset = addr;  // Bug #11
        
        if (byte_offset + 64 <= SHARED_ROM_SIZE) {
            // Access from shared memory
            const uint8_t* chunk_ptr = shared_rom + byte_offset;
            blake2b_update(vm.mem_digest_state, chunk_ptr, 64);
            uint32_t word_offset = vm.memory_counter % 8;
            result = ((uint64_t*)chunk_ptr)[word_offset];
        } else if (byte_offset + 64 <= rom_size) {
            // Fall back to global memory
            const uint8_t* chunk_ptr = rom + byte_offset;
            blake2b_update(vm.mem_digest_state, chunk_ptr, 64);
            uint32_t word_offset = vm.memory_counter % 8;
            result = ((uint64_t*)chunk_ptr)[word_offset];
        } else {
            result = 0;
        }
        vm.memory_counter++;
    }

    // Store result
    vm.regs[instr.r3] = result;

    // Update prog_digest
    blake2b_update(vm.prog_digest_state, prog_chunk, INSTR_SIZE);
}

extern "C" __global__ void ashmaize_hash_kernel_shared_rom(
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
    uint8_t* outputs,
    uint32_t num_hashes
) {
    uint32_t tid = blockIdx.x * blockDim.x + threadIdx.x;
    
    if (tid >= num_hashes) {
        return;
    }
    
    // Allocate shared memory for ROM cache
    __shared__ uint8_t shared_rom[SHARED_ROM_SIZE];
    
    // Cooperatively load ROM into shared memory
    // Each thread loads its portion
    uint32_t chunks_per_thread = (SHARED_ROM_SIZE + blockDim.x - 1) / blockDim.x;
    for (uint32_t i = 0; i < chunks_per_thread; i++) {
        uint32_t idx = threadIdx.x * chunks_per_thread + i;
        if (idx < SHARED_ROM_SIZE && idx < rom_size) {
            shared_rom[idx] = rom_data[idx];
        }
    }
    __syncthreads();  // Wait for all threads to finish loading
    
    // Initialize VM state
    VMState vm;
    
    // H' (Argon2-based initialization)
    const uint8_t* prog_seed = initial_prog_seeds + tid * 64;
    
    const uint8_t* salt = salts + tid * salt_len;
    cudahprime(salt, salt_len, vm.prog_digest, 64);
    
    // Initialize Blake2b for program digests
    cudablake2b_init(vm.prog_digest_state, 64);
    
    // Initialize Blake2b for memory digests
    vm.mem_digest_state.h[0] = 0x6a09e667f3bcc908ULL ^ 0x01010040ULL;
    vm.mem_digest_state.h[1] = 0xbb67ae8584caa73bULL;
    vm.mem_digest_state.h[2] = 0x3c6ef372fe94f82bULL;
    vm.mem_digest_state.h[3] = 0xa54ff53a5f1d36f1ULL;
    vm.mem_digest_state.h[4] = 0x510e527fade682d1ULL;
    vm.mem_digest_state.h[5] = 0x9b05688c2b3e6c1fULL;
    vm.mem_digest_state.h[6] = 0x1f83d9abfb41bd6bULL;
    vm.mem_digest_state.h[7] = 0x5be0cd19137e2179ULL;
    
    vm.mem_digest_state.t[0] = 0;
    vm.mem_digest_state.t[1] = 0;
    
    for (int i = 0; i < 16; i++) {
        vm.mem_digest_state.m[i] = 0;
    }
    
    // Initialize registers
    for (int i = 0; i < 8; i++) {
        vm.regs[i] = ((uint64_t*)prog_seed)[i];
    }
    
    vm.ip = 0;
    vm.loop_counter = 0;
    vm.memory_counter = 0;
    
    uint8_t* program = programs + tid * program_size;
    
    // Main execution loop
    for (uint32_t loop = 0; loop < nb_loops; loop++) {
        for (uint32_t i = 0; i < nb_instrs; i++) {
            uint8_t* prog_chunk = program + vm.ip * INSTR_SIZE;
            execute_one_instruction_shared_rom(vm, rom_data, prog_chunk, rom_size, shared_rom);
            vm.ip = (vm.ip + 1) % nb_instrs;
        }
        
        post_instructions(vm);
        vm.loop_counter++;
    }
    
    // Finalize
    uint8_t* output = outputs + tid * 64;
    
    blake2b_state final_state;
    cudablake2b_init(&final_state, 64);
    
    blake2b_update(&final_state, rom_digest, 32);
    
    uint8_t prog_digest_final[64];
    blake2b_final(&vm.prog_digest_state, prog_digest_final, 64);
    blake2b_update(&final_state, prog_digest_final, 64);
    
    uint32_t loop_counter_32 = (uint32_t)vm.loop_counter;
    blake2b_update(&final_state, (uint8_t*)&loop_counter_32, 4);
    blake2b_update(&final_state, (uint8_t*)&vm.memory_counter, 4);
    
    for (int i = 0; i < 8; i++) {
        blake2b_update(&final_state, (uint8_t*)&vm.regs[i], 8);
    }
    
    blake2b_final(&final_state, output, 64);
}


// Shared Memory ROM Caching Kernel
// Cache first 16KB of ROM in shared memory for faster access

#define SHARED_ROM_SIZE 16384  // 16KB shared memory cache

__device__ void execute_one_instruction_shared_rom(VMState &vm, const uint8_t *rom,
                                       const uint8_t *prog_chunk, uint32_t rom_size,
                                       const uint8_t *shared_rom, bool debug = false) {
    Instruction instr = decode_instruction(prog_chunk);

    uint64_t src1, src2;

    bool debug_mem = debug;
    
    // Always evaluate src1
    if (instr.op1 < 5) {
        src1 = vm.regs[instr.r1];
    } else if (instr.op1 < 9) {
        // Memory access - try shared ROM first
        uint64_t addr = instr.lit1;
        uint64_t byte_offset = addr;  // Bug #11: chunk index as byte offset
        
        if (byte_offset + 64 <= SHARED_ROM_SIZE) {
            // Access from shared memory
            const uint8_t* chunk_ptr = shared_rom + byte_offset;
            blake2b_update(vm.mem_digest_state, chunk_ptr, 64);
            uint32_t word_offset = vm.memory_counter % 8;
            src1 = ((uint64_t*)chunk_ptr)[word_offset];
        } else if (byte_offset + 64 <= rom_size) {
            // Fall back to global memory
            const uint8_t* chunk_ptr = rom + byte_offset;
            blake2b_update(vm.mem_digest_state, chunk_ptr, 64);
            uint32_t word_offset = vm.memory_counter % 8;
            src1 = ((uint64_t*)chunk_ptr)[word_offset];
        } else {
            src1 = 0;
        }
        vm.memory_counter++;
    } else if (instr.op1 < 13) {
        src1 = instr.lit1;
    } else if (instr.op1 < 14) {
        src1 = special1_value64(vm.prog_digest_state);
    } else {
        src1 = special2_value64(vm.mem_digest_state);
    }

    // Only evaluate src2 for Op3 instructions
    bool is_op2 = (instr.opcode >= 128 && instr.opcode < 148) ||
                  (instr.opcode >= 188 && instr.opcode < 240);
    bool is_op3 = !is_op2;
    if (is_op3) {
        if (instr.op2 < 5) {
            src2 = vm.regs[instr.r2];
        } else if (instr.op2 < 9) {
            // Memory access - try shared ROM first
            uint64_t addr = instr.lit2;
            uint64_t byte_offset = addr;  // Bug #11
            
            if (byte_offset + 64 <= SHARED_ROM_SIZE) {
                // Access from shared memory
                const uint8_t* chunk_ptr = shared_rom + byte_offset;
                blake2b_update(vm.mem_digest_state, chunk_ptr, 64);
                uint32_t word_offset = vm.memory_counter % 8;
                src2 = ((uint64_t*)chunk_ptr)[word_offset];
            } else if (byte_offset + 64 <= rom_size) {
                // Fall back to global memory
                const uint8_t* chunk_ptr = rom + byte_offset;
                blake2b_update(vm.mem_digest_state, chunk_ptr, 64);
                uint32_t word_offset = vm.memory_counter % 8;
                src2 = ((uint64_t*)chunk_ptr)[word_offset];
            } else {
                src2 = 0;
            }
            vm.memory_counter++;
        } else if (instr.op2 < 13) {
            src2 = instr.lit2;
        } else if (instr.op2 < 14) {
            src2 = special1_value64(vm.prog_digest_state);
        } else {
            src2 = special2_value64(vm.mem_digest_state);
        }
    } else {
        src2 = 0;
    }

    // Execute instruction
    uint64_t result;
    if (instr.opcode < 32) {
        result = src1 + src2;
    }
    else if (instr.opcode < 64) {
        result = src1 ^ src2;
    }
    else if (instr.opcode < 96) {
        result = src1 * src2;
    }
    else if (instr.opcode < 112) {
        if (src2 != 0) {
            result = src1 / src2;
        } else {
            result = src1;
        }
    }
    else if (instr.opcode < 128) {
        if (src2 != 0) {
            result = src1 / src2;  // Bug #14: CPU does division for modulo
        } else {
            result = src1;
        }
    }
    else if (instr.opcode < 144) {
        uint32_t shift = src2 % 64;
        if (shift == 0) {
            result = src1;
        } else {
            result = (src1 << shift) | (src1 >> (64 - shift));
        }
    }
    else if (instr.opcode < 160) {
        uint32_t shift = src2 % 64;
        if (shift == 0) {
            result = src1;
        } else {
            result = (src1 >> shift) | (src1 << (64 - shift));
        }
    }
    else if (instr.opcode < 208) {
        result = isqrt_64(src1);
    }
    else {
        // Memory access - try shared ROM first
        uint64_t addr = src1;
        uint64_t byte_offset = addr;  // Bug #11
        
        if (byte_offset + 64 <= SHARED_ROM_SIZE) {
            // Access from shared memory
            const uint8_t* chunk_ptr = shared_rom + byte_offset;
            blake2b_update(vm.mem_digest_state, chunk_ptr, 64);
            uint32_t word_offset = vm.memory_counter % 8;
            result = ((uint64_t*)chunk_ptr)[word_offset];
        } else if (byte_offset + 64 <= rom_size) {
            // Fall back to global memory
            const uint8_t* chunk_ptr = rom + byte_offset;
            blake2b_update(vm.mem_digest_state, chunk_ptr, 64);
            uint32_t word_offset = vm.memory_counter % 8;
            result = ((uint64_t*)chunk_ptr)[word_offset];
        } else {
            result = 0;
        }
        vm.memory_counter++;
    }

    // Store result
    vm.regs[instr.r3] = result;

    // Update prog_digest
    blake2b_update(vm.prog_digest_state, prog_chunk, INSTR_SIZE);
}

extern "C" __global__ void ashmaize_hash_kernel_shared_rom(
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
    uint8_t* outputs,
    uint32_t num_hashes
) {
    uint32_t tid = blockIdx.x * blockDim.x + threadIdx.x;
    
    if (tid >= num_hashes) {
        return;
    }
    
    // Allocate shared memory for ROM cache
    __shared__ uint8_t shared_rom[SHARED_ROM_SIZE];
    
    // Cooperatively load ROM into shared memory
    // Each thread loads its portion
    uint32_t chunks_per_thread = (SHARED_ROM_SIZE + blockDim.x - 1) / blockDim.x;
    for (uint32_t i = 0; i < chunks_per_thread; i++) {
        uint32_t idx = threadIdx.x * chunks_per_thread + i;
        if (idx < SHARED_ROM_SIZE && idx < rom_size) {
            shared_rom[idx] = rom_data[idx];
        }
    }
    __syncthreads();  // Wait for all threads to finish loading
    
    // Initialize VM state
    VMState vm;
    
    // H' (Argon2-based initialization)
    const uint8_t* prog_seed = initial_prog_seeds + tid * 64;
    
    const uint8_t* salt = salts + tid * salt_len;
    cudahprime(salt, salt_len, vm.prog_digest, 64);
    
    // Initialize Blake2b for program digests
    cudablake2b_init(vm.prog_digest_state, 64);
    
    // Initialize Blake2b for memory digests
    vm.mem_digest_state.h[0] = 0x6a09e667f3bcc908ULL ^ 0x01010040ULL;
    vm.mem_digest_state.h[1] = 0xbb67ae8584caa73bULL;
    vm.mem_digest_state.h[2] = 0x3c6ef372fe94f82bULL;
    vm.mem_digest_state.h[3] = 0xa54ff53a5f1d36f1ULL;
    vm.mem_digest_state.h[4] = 0x510e527fade682d1ULL;
    vm.mem_digest_state.h[5] = 0x9b05688c2b3e6c1fULL;
    vm.mem_digest_state.h[6] = 0x1f83d9abfb41bd6bULL;
    vm.mem_digest_state.h[7] = 0x5be0cd19137e2179ULL;
    
    vm.mem_digest_state.t[0] = 0;
    vm.mem_digest_state.t[1] = 0;
    
    for (int i = 0; i < 16; i++) {
        vm.mem_digest_state.m[i] = 0;
    }
    
    // Initialize registers
    for (int i = 0; i < 8; i++) {
        vm.regs[i] = ((uint64_t*)prog_seed)[i];
    }
    
    vm.ip = 0;
    vm.loop_counter = 0;
    vm.memory_counter = 0;
    
    uint8_t* program = programs + tid * program_size;
    
    // Main execution loop
    for (uint32_t loop = 0; loop < nb_loops; loop++) {
        for (uint32_t i = 0; i < nb_instrs; i++) {
            uint8_t* prog_chunk = program + vm.ip * INSTR_SIZE;
            execute_one_instruction_shared_rom(vm, rom_data, prog_chunk, rom_size, shared_rom);
            vm.ip = (vm.ip + 1) % nb_instrs;
        }
        
        post_instructions(vm);
        vm.loop_counter++;
    }
    
    // Finalize
    uint8_t* output = outputs + tid * 64;
    
    blake2b_state final_state;
    cudablake2b_init(&final_state, 64);
    
    blake2b_update(&final_state, rom_digest, 32);
    
    uint8_t prog_digest_final[64];
    blake2b_final(&vm.prog_digest_state, prog_digest_final, 64);
    blake2b_update(&final_state, prog_digest_final, 64);
    
    uint32_t loop_counter_32 = (uint32_t)vm.loop_counter;
    blake2b_update(&final_state, (uint8_t*)&loop_counter_32, 4);
    blake2b_update(&final_state, (uint8_t*)&vm.memory_counter, 4);
    
    for (int i = 0; i < 8; i++) {
        blake2b_update(&final_state, (uint8_t*)&vm.regs[i], 8);
    }
    
    blake2b_final(&final_state, output, 64);
}


// Shared Memory ROM Caching Kernel
// Cache first 16KB of ROM in shared memory for faster access

#define SHARED_ROM_SIZE 16384  // 16KB shared memory cache

__device__ void execute_one_instruction_shared_rom(VMState &vm, const uint8_t *rom,
                                       const uint8_t *prog_chunk, uint32_t rom_size,
                                       const uint8_t *shared_rom, bool debug = false) {
    Instruction instr = decode_instruction(prog_chunk);

    uint64_t src1, src2;

    bool debug_mem = debug;
    
    // Always evaluate src1
    if (instr.op1 < 5) {
        src1 = vm.regs[instr.r1];
    } else if (instr.op1 < 9) {
        // Memory access - try shared ROM first
        uint64_t addr = instr.lit1;
        uint64_t byte_offset = addr;  // Bug #11: chunk index as byte offset
        
        if (byte_offset + 64 <= SHARED_ROM_SIZE) {
            // Access from shared memory
            const uint8_t* chunk_ptr = shared_rom + byte_offset;
            blake2b_update(vm.mem_digest_state, chunk_ptr, 64);
            uint32_t word_offset = vm.memory_counter % 8;
            src1 = ((uint64_t*)chunk_ptr)[word_offset];
        } else if (byte_offset + 64 <= rom_size) {
            // Fall back to global memory
            const uint8_t* chunk_ptr = rom + byte_offset;
            blake2b_update(vm.mem_digest_state, chunk_ptr, 64);
            uint32_t word_offset = vm.memory_counter % 8;
            src1 = ((uint64_t*)chunk_ptr)[word_offset];
        } else {
            src1 = 0;
        }
        vm.memory_counter++;
    } else if (instr.op1 < 13) {
        src1 = instr.lit1;
    } else if (instr.op1 < 14) {
        src1 = special1_value64(vm.prog_digest_state);
    } else {
        src1 = special2_value64(vm.mem_digest_state);
    }

    // Only evaluate src2 for Op3 instructions
    bool is_op2 = (instr.opcode >= 128 && instr.opcode < 148) ||
                  (instr.opcode >= 188 && instr.opcode < 240);
    bool is_op3 = !is_op2;
    if (is_op3) {
        if (instr.op2 < 5) {
            src2 = vm.regs[instr.r2];
        } else if (instr.op2 < 9) {
            // Memory access - try shared ROM first
            uint64_t addr = instr.lit2;
            uint64_t byte_offset = addr;  // Bug #11
            
            if (byte_offset + 64 <= SHARED_ROM_SIZE) {
                // Access from shared memory
                const uint8_t* chunk_ptr = shared_rom + byte_offset;
                blake2b_update(vm.mem_digest_state, chunk_ptr, 64);
                uint32_t word_offset = vm.memory_counter % 8;
                src2 = ((uint64_t*)chunk_ptr)[word_offset];
            } else if (byte_offset + 64 <= rom_size) {
                // Fall back to global memory
                const uint8_t* chunk_ptr = rom + byte_offset;
                blake2b_update(vm.mem_digest_state, chunk_ptr, 64);
                uint32_t word_offset = vm.memory_counter % 8;
                src2 = ((uint64_t*)chunk_ptr)[word_offset];
            } else {
                src2 = 0;
            }
            vm.memory_counter++;
        } else if (instr.op2 < 13) {
            src2 = instr.lit2;
        } else if (instr.op2 < 14) {
            src2 = special1_value64(vm.prog_digest_state);
        } else {
            src2 = special2_value64(vm.mem_digest_state);
        }
    } else {
        src2 = 0;
    }

    // Execute instruction
    uint64_t result;
    if (instr.opcode < 32) {
        result = src1 + src2;
    }
    else if (instr.opcode < 64) {
        result = src1 ^ src2;
    }
    else if (instr.opcode < 96) {
        result = src1 * src2;
    }
    else if (instr.opcode < 112) {
        if (src2 != 0) {
            result = src1 / src2;
        } else {
            result = src1;
        }
    }
    else if (instr.opcode < 128) {
        if (src2 != 0) {
            result = src1 / src2;  // Bug #14: CPU does division for modulo
        } else {
            result = src1;
        }
    }
    else if (instr.opcode < 144) {
        uint32_t shift = src2 % 64;
        if (shift == 0) {
            result = src1;
        } else {
            result = (src1 << shift) | (src1 >> (64 - shift));
        }
    }
    else if (instr.opcode < 160) {
        uint32_t shift = src2 % 64;
        if (shift == 0) {
            result = src1;
        } else {
            result = (src1 >> shift) | (src1 << (64 - shift));
        }
    }
    else if (instr.opcode < 208) {
        result = isqrt_64(src1);
    }
    else {
        // Memory access - try shared ROM first
        uint64_t addr = src1;
        uint64_t byte_offset = addr;  // Bug #11
        
        if (byte_offset + 64 <= SHARED_ROM_SIZE) {
            // Access from shared memory
            const uint8_t* chunk_ptr = shared_rom + byte_offset;
            blake2b_update(vm.mem_digest_state, chunk_ptr, 64);
            uint32_t word_offset = vm.memory_counter % 8;
            result = ((uint64_t*)chunk_ptr)[word_offset];
        } else if (byte_offset + 64 <= rom_size) {
            // Fall back to global memory
            const uint8_t* chunk_ptr = rom + byte_offset;
            blake2b_update(vm.mem_digest_state, chunk_ptr, 64);
            uint32_t word_offset = vm.memory_counter % 8;
            result = ((uint64_t*)chunk_ptr)[word_offset];
        } else {
            result = 0;
        }
        vm.memory_counter++;
    }

    // Store result
    vm.regs[instr.r3] = result;

    // Update prog_digest
    blake2b_update(vm.prog_digest_state, prog_chunk, INSTR_SIZE);
}

extern "C" __global__ void ashmaize_hash_kernel_shared_rom(
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
    uint8_t* outputs,
    uint32_t num_hashes
) {
    uint32_t tid = blockIdx.x * blockDim.x + threadIdx.x;
    
    if (tid >= num_hashes) {
        return;
    }
    
    // Allocate shared memory for ROM cache
    __shared__ uint8_t shared_rom[SHARED_ROM_SIZE];
    
    // Cooperatively load ROM into shared memory
    // Each thread loads its portion
    uint32_t chunks_per_thread = (SHARED_ROM_SIZE + blockDim.x - 1) / blockDim.x;
    for (uint32_t i = 0; i < chunks_per_thread; i++) {
        uint32_t idx = threadIdx.x * chunks_per_thread + i;
        if (idx < SHARED_ROM_SIZE && idx < rom_size) {
            shared_rom[idx] = rom_data[idx];
        }
    }
    __syncthreads();  // Wait for all threads to finish loading
    
    // Initialize VM state
    VMState vm;
    
    // H' (Argon2-based initialization)
    const uint8_t* prog_seed = initial_prog_seeds + tid * 64;
    
    const uint8_t* salt = salts + tid * salt_len;
    cudahprime(salt, salt_len, vm.prog_digest, 64);
    
    // Initialize Blake2b for program digests
    cudablake2b_init(vm.prog_digest_state, 64);
    
    // Initialize Blake2b for memory digests
    vm.mem_digest_state.h[0] = 0x6a09e667f3bcc908ULL ^ 0x01010040ULL;
    vm.mem_digest_state.h[1] = 0xbb67ae8584caa73bULL;
    vm.mem_digest_state.h[2] = 0x3c6ef372fe94f82bULL;
    vm.mem_digest_state.h[3] = 0xa54ff53a5f1d36f1ULL;
    vm.mem_digest_state.h[4] = 0x510e527fade682d1ULL;
    vm.mem_digest_state.h[5] = 0x9b05688c2b3e6c1fULL;
    vm.mem_digest_state.h[6] = 0x1f83d9abfb41bd6bULL;
    vm.mem_digest_state.h[7] = 0x5be0cd19137e2179ULL;
    
    vm.mem_digest_state.t[0] = 0;
    vm.mem_digest_state.t[1] = 0;
    
    for (int i = 0; i < 16; i++) {
        vm.mem_digest_state.m[i] = 0;
    }
    
    // Initialize registers
    for (int i = 0; i < 8; i++) {
        vm.regs[i] = ((uint64_t*)prog_seed)[i];
    }
    
    vm.ip = 0;
    vm.loop_counter = 0;
    vm.memory_counter = 0;
    
    uint8_t* program = programs + tid * program_size;
    
    // Main execution loop
    for (uint32_t loop = 0; loop < nb_loops; loop++) {
        for (uint32_t i = 0; i < nb_instrs; i++) {
            uint8_t* prog_chunk = program + vm.ip * INSTR_SIZE;
            execute_one_instruction_shared_rom(vm, rom_data, prog_chunk, rom_size, shared_rom);
            vm.ip = (vm.ip + 1) % nb_instrs;
        }
        
        post_instructions(vm);
        vm.loop_counter++;
    }
    
    // Finalize
    uint8_t* output = outputs + tid * 64;
    
    blake2b_state final_state;
    cudablake2b_init(&final_state, 64);
    
    blake2b_update(&final_state, rom_digest, 32);
    
    uint8_t prog_digest_final[64];
    blake2b_final(&vm.prog_digest_state, prog_digest_final, 64);
    blake2b_update(&final_state, prog_digest_final, 64);
    
    uint32_t loop_counter_32 = (uint32_t)vm.loop_counter;
    blake2b_update(&final_state, (uint8_t*)&loop_counter_32, 4);
    blake2b_update(&final_state, (uint8_t*)&vm.memory_counter, 4);
    
    for (int i = 0; i < 8; i++) {
        blake2b_update(&final_state, (uint8_t*)&vm.regs[i], 8);
    }
    
    blake2b_final(&final_state, output, 64);
}




