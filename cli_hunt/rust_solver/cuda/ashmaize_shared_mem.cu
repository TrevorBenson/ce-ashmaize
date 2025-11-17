// Ashmaize kernel with Shared Memory ROM optimization
// This is a MINIMAL modification to test shared memory caching

#include "ashmaize_vm.cuh"

// Shared memory size - 16KB per block
#define SHARED_ROM_SIZE 16384

// Modified mem_access64 that uses shared ROM cache
__device__ uint64_t mem_access64_shared(VMState &vm, const uint8_t *rom_global, 
                                        const uint8_t *rom_shared, uint64_t addr, 
                                        uint32_t rom_size) {
    uint64_t byte_offset = addr;  // Bug #11: chunk index used as byte offset
    
    const uint8_t* chunk_ptr;
    
    // Check if in shared memory range
    if (byte_offset + 64 <= SHARED_ROM_SIZE && byte_offset + 64 <= rom_size) {
        chunk_ptr = rom_shared + byte_offset;
    } else if (byte_offset + 64 <= rom_size) {
        chunk_ptr = rom_global + byte_offset;
    } else {
        return 0;
    }
    
    blake2b_update(vm.mem_digest_state, chunk_ptr, 64);
    
    uint32_t word_offset = vm.memory_counter % 8;
    uint64_t result = ((uint64_t*)chunk_ptr)[word_offset];
    
    vm.memory_counter++;
    return result;
}

// Copy of execute_one_instruction but using shared ROM
__device__ void execute_one_instruction_shared(VMState &vm, const uint8_t *rom_global,
                                               const uint8_t *rom_shared, const uint8_t *prog_chunk, 
                                               uint32_t rom_size) {
    Instruction instr = decode_instruction(prog_chunk);

    uint64_t src1, src2;
    
    // Evaluate src1
    if (instr.op1 < 5) {
        src1 = vm.regs[instr.r1];
    } else if (instr.op1 < 9) {
        src1 = mem_access64_shared(vm, rom_global, rom_shared, instr.lit1, rom_size);
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
            src2 = mem_access64_shared(vm, rom_global, rom_shared, instr.lit2, rom_size);
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

    // Execute instruction (same as original)
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
            result = src1 / src2;  // Bug #14: CPU modulo is division
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
        result = mem_access64_shared(vm, rom_global, rom_shared, src1, rom_size);
    }

    vm.regs[instr.r3] = result;
    blake2b_update(vm.prog_digest_state, prog_chunk, INSTR_SIZE);
}

extern "C" __global__ void ashmaize_hash_kernel_shared_mem(
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
    
    // Allocate shared memory for ROM cache (16KB)
    __shared__ uint8_t shared_rom[SHARED_ROM_SIZE];
    
    // Cooperatively load ROM into shared memory
    uint32_t bytes_per_thread = (SHARED_ROM_SIZE + blockDim.x - 1) / blockDim.x;
    uint32_t start_byte = threadIdx.x * bytes_per_thread;
    uint32_t end_byte = min(start_byte + bytes_per_thread, SHARED_ROM_SIZE);
    
    for (uint32_t i = start_byte; i < end_byte && i < rom_size; i++) {
        shared_rom[i] = rom_data[i];
    }
    __syncthreads();  // Wait for all threads to finish loading

    // Rest is IDENTICAL to original kernel
    const uint8_t* salt = salts + tid * salt_len;
    uint8_t* program = programs + tid * program_size;
    uint8_t* result = results + tid * 64;

    VMState vm;
    vm_init(vm, rom_digest, 64, salt, salt_len);

    for (uint32_t loop = 0; loop < nb_loops; ++loop) {
        hprime(program, program_size, vm.prog_seed, 64);

        for (uint32_t instr_idx = 0; instr_idx < nb_instrs; ++instr_idx) {
            execute_one_instruction_shared(vm, rom_data, shared_rom, program + instr_idx * INSTR_SIZE, rom_size);
        }

        post_instructions(vm);
    }

    Blake2bState final_prog_state = vm.prog_digest_state;
    uint8_t prog_final[64];
    blake2b_final(final_prog_state, prog_final, 64);

    Blake2bState final_mem_state = vm.mem_digest_state;
    uint8_t mem_final[64];
    blake2b_final(final_mem_state, mem_final, 64);

    Blake2bState final_state;
    cudablake2b_init(&final_state, 64);
    blake2b_update(&final_state, rom_digest, 32);
    blake2b_update(&final_state, prog_final, 64);
    
    uint32_t loop_counter_32 = (uint32_t)vm.loop_counter;
    blake2b_update(&final_state, (const uint8_t*)&loop_counter_32, 4);
    blake2b_update(&final_state, (const uint8_t*)&vm.memory_counter, 4);
    
    for (int i = 0; i < 8; i++) {
        blake2b_update(&final_state, (const uint8_t*)&vm.regs[i], 8);
    }
    
    blake2b_final(&final_state, result, 64);
}



#include "ashmaize_vm.cuh"

// Shared memory size - 16KB per block
#define SHARED_ROM_SIZE 16384

// Modified mem_access64 that uses shared ROM cache
__device__ uint64_t mem_access64_shared(VMState &vm, const uint8_t *rom_global, 
                                        const uint8_t *rom_shared, uint64_t addr, 
                                        uint32_t rom_size) {
    uint64_t byte_offset = addr;  // Bug #11: chunk index used as byte offset
    
    const uint8_t* chunk_ptr;
    
    // Check if in shared memory range
    if (byte_offset + 64 <= SHARED_ROM_SIZE && byte_offset + 64 <= rom_size) {
        chunk_ptr = rom_shared + byte_offset;
    } else if (byte_offset + 64 <= rom_size) {
        chunk_ptr = rom_global + byte_offset;
    } else {
        return 0;
    }
    
    blake2b_update(vm.mem_digest_state, chunk_ptr, 64);
    
    uint32_t word_offset = vm.memory_counter % 8;
    uint64_t result = ((uint64_t*)chunk_ptr)[word_offset];
    
    vm.memory_counter++;
    return result;
}

// Copy of execute_one_instruction but using shared ROM
__device__ void execute_one_instruction_shared(VMState &vm, const uint8_t *rom_global,
                                               const uint8_t *rom_shared, const uint8_t *prog_chunk, 
                                               uint32_t rom_size) {
    Instruction instr = decode_instruction(prog_chunk);

    uint64_t src1, src2;
    
    // Evaluate src1
    if (instr.op1 < 5) {
        src1 = vm.regs[instr.r1];
    } else if (instr.op1 < 9) {
        src1 = mem_access64_shared(vm, rom_global, rom_shared, instr.lit1, rom_size);
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
            src2 = mem_access64_shared(vm, rom_global, rom_shared, instr.lit2, rom_size);
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

    // Execute instruction (same as original)
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
            result = src1 / src2;  // Bug #14: CPU modulo is division
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
        result = mem_access64_shared(vm, rom_global, rom_shared, src1, rom_size);
    }

    vm.regs[instr.r3] = result;
    blake2b_update(vm.prog_digest_state, prog_chunk, INSTR_SIZE);
}

extern "C" __global__ void ashmaize_hash_kernel_shared_mem(
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
    
    // Allocate shared memory for ROM cache (16KB)
    __shared__ uint8_t shared_rom[SHARED_ROM_SIZE];
    
    // Cooperatively load ROM into shared memory
    uint32_t bytes_per_thread = (SHARED_ROM_SIZE + blockDim.x - 1) / blockDim.x;
    uint32_t start_byte = threadIdx.x * bytes_per_thread;
    uint32_t end_byte = min(start_byte + bytes_per_thread, SHARED_ROM_SIZE);
    
    for (uint32_t i = start_byte; i < end_byte && i < rom_size; i++) {
        shared_rom[i] = rom_data[i];
    }
    __syncthreads();  // Wait for all threads to finish loading

    // Rest is IDENTICAL to original kernel
    const uint8_t* salt = salts + tid * salt_len;
    uint8_t* program = programs + tid * program_size;
    uint8_t* result = results + tid * 64;

    VMState vm;
    vm_init(vm, rom_digest, 64, salt, salt_len);

    for (uint32_t loop = 0; loop < nb_loops; ++loop) {
        hprime(program, program_size, vm.prog_seed, 64);

        for (uint32_t instr_idx = 0; instr_idx < nb_instrs; ++instr_idx) {
            execute_one_instruction_shared(vm, rom_data, shared_rom, program + instr_idx * INSTR_SIZE, rom_size);
        }

        post_instructions(vm);
    }

    Blake2bState final_prog_state = vm.prog_digest_state;
    uint8_t prog_final[64];
    blake2b_final(final_prog_state, prog_final, 64);

    Blake2bState final_mem_state = vm.mem_digest_state;
    uint8_t mem_final[64];
    blake2b_final(final_mem_state, mem_final, 64);

    Blake2bState final_state;
    cudablake2b_init(&final_state, 64);
    blake2b_update(&final_state, rom_digest, 32);
    blake2b_update(&final_state, prog_final, 64);
    
    uint32_t loop_counter_32 = (uint32_t)vm.loop_counter;
    blake2b_update(&final_state, (const uint8_t*)&loop_counter_32, 4);
    blake2b_update(&final_state, (const uint8_t*)&vm.memory_counter, 4);
    
    for (int i = 0; i < 8; i++) {
        blake2b_update(&final_state, (const uint8_t*)&vm.regs[i], 8);
    }
    
    blake2b_final(&final_state, result, 64);
}



#include "ashmaize_vm.cuh"

// Shared memory size - 16KB per block
#define SHARED_ROM_SIZE 16384

// Modified mem_access64 that uses shared ROM cache
__device__ uint64_t mem_access64_shared(VMState &vm, const uint8_t *rom_global, 
                                        const uint8_t *rom_shared, uint64_t addr, 
                                        uint32_t rom_size) {
    uint64_t byte_offset = addr;  // Bug #11: chunk index used as byte offset
    
    const uint8_t* chunk_ptr;
    
    // Check if in shared memory range
    if (byte_offset + 64 <= SHARED_ROM_SIZE && byte_offset + 64 <= rom_size) {
        chunk_ptr = rom_shared + byte_offset;
    } else if (byte_offset + 64 <= rom_size) {
        chunk_ptr = rom_global + byte_offset;
    } else {
        return 0;
    }
    
    blake2b_update(vm.mem_digest_state, chunk_ptr, 64);
    
    uint32_t word_offset = vm.memory_counter % 8;
    uint64_t result = ((uint64_t*)chunk_ptr)[word_offset];
    
    vm.memory_counter++;
    return result;
}

// Copy of execute_one_instruction but using shared ROM
__device__ void execute_one_instruction_shared(VMState &vm, const uint8_t *rom_global,
                                               const uint8_t *rom_shared, const uint8_t *prog_chunk, 
                                               uint32_t rom_size) {
    Instruction instr = decode_instruction(prog_chunk);

    uint64_t src1, src2;
    
    // Evaluate src1
    if (instr.op1 < 5) {
        src1 = vm.regs[instr.r1];
    } else if (instr.op1 < 9) {
        src1 = mem_access64_shared(vm, rom_global, rom_shared, instr.lit1, rom_size);
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
            src2 = mem_access64_shared(vm, rom_global, rom_shared, instr.lit2, rom_size);
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

    // Execute instruction (same as original)
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
            result = src1 / src2;  // Bug #14: CPU modulo is division
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
        result = mem_access64_shared(vm, rom_global, rom_shared, src1, rom_size);
    }

    vm.regs[instr.r3] = result;
    blake2b_update(vm.prog_digest_state, prog_chunk, INSTR_SIZE);
}

extern "C" __global__ void ashmaize_hash_kernel_shared_mem(
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
    
    // Allocate shared memory for ROM cache (16KB)
    __shared__ uint8_t shared_rom[SHARED_ROM_SIZE];
    
    // Cooperatively load ROM into shared memory
    uint32_t bytes_per_thread = (SHARED_ROM_SIZE + blockDim.x - 1) / blockDim.x;
    uint32_t start_byte = threadIdx.x * bytes_per_thread;
    uint32_t end_byte = min(start_byte + bytes_per_thread, SHARED_ROM_SIZE);
    
    for (uint32_t i = start_byte; i < end_byte && i < rom_size; i++) {
        shared_rom[i] = rom_data[i];
    }
    __syncthreads();  // Wait for all threads to finish loading

    // Rest is IDENTICAL to original kernel
    const uint8_t* salt = salts + tid * salt_len;
    uint8_t* program = programs + tid * program_size;
    uint8_t* result = results + tid * 64;

    VMState vm;
    vm_init(vm, rom_digest, 64, salt, salt_len);

    for (uint32_t loop = 0; loop < nb_loops; ++loop) {
        hprime(program, program_size, vm.prog_seed, 64);

        for (uint32_t instr_idx = 0; instr_idx < nb_instrs; ++instr_idx) {
            execute_one_instruction_shared(vm, rom_data, shared_rom, program + instr_idx * INSTR_SIZE, rom_size);
        }

        post_instructions(vm);
    }

    Blake2bState final_prog_state = vm.prog_digest_state;
    uint8_t prog_final[64];
    blake2b_final(final_prog_state, prog_final, 64);

    Blake2bState final_mem_state = vm.mem_digest_state;
    uint8_t mem_final[64];
    blake2b_final(final_mem_state, mem_final, 64);

    Blake2bState final_state;
    cudablake2b_init(&final_state, 64);
    blake2b_update(&final_state, rom_digest, 32);
    blake2b_update(&final_state, prog_final, 64);
    
    uint32_t loop_counter_32 = (uint32_t)vm.loop_counter;
    blake2b_update(&final_state, (const uint8_t*)&loop_counter_32, 4);
    blake2b_update(&final_state, (const uint8_t*)&vm.memory_counter, 4);
    
    for (int i = 0; i < 8; i++) {
        blake2b_update(&final_state, (const uint8_t*)&vm.regs[i], 8);
    }
    
    blake2b_final(&final_state, result, 64);
}

