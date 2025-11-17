#include "ashmaize_vm.cuh"

__device__ void execute_one_instruction(VMState &vm, const uint8_t *rom,
                                       const uint8_t *prog_chunk, uint32_t rom_size, bool debug = false) {
    Instruction instr = decode_instruction(prog_chunk);
    
    if (debug) {
        printf("[GPU Instr 37] Decoded: opcode=%u, op1=%u, op2=%u, r1=%u, r2=%u, r3=%u\n",
            instr.opcode, instr.op1, instr.op2, instr.r1, instr.r2, instr.r3);
        printf("               lit1=0x%016llx, lit2=0x%016llx\n",
            (unsigned long long)instr.lit1, (unsigned long long)instr.lit2);
        printf("               Before: regs[%u]=0x%016llx\n",
            instr.r3, (unsigned long long)vm.regs[instr.r3]);
    }

    uint64_t src1, src2;

    // Debug flag for operand evaluation
    bool debug_mem = debug;
    
    // Always evaluate src1
    if (instr.op1 < 5) {
        src1 = vm.regs[instr.r1];
    } else if (instr.op1 < 9) {
        if (debug_mem) printf("[GPU] src1 memory access:\n");
        src1 = mem_access64(vm, rom, instr.lit1, rom_size, debug_mem);
    } else if (instr.op1 < 13) {
        src1 = instr.lit1;
        if (debug_mem) printf("[GPU] src1 literal: 0x%016llx\n", (unsigned long long)src1);
    } else if (instr.op1 < 14) {
        src1 = special1_value64(vm.prog_digest_state);
        if (debug_mem) printf("[GPU] src1 special1: 0x%016llx\n", (unsigned long long)src1);
    } else {
        src1 = special2_value64(vm.mem_digest_state);
        if (debug_mem) printf("[GPU] src1 special2: 0x%016llx\n", (unsigned long long)src1);
    }

    // Only evaluate src2 for Op3 instructions (opcodes that need two operands)
    // Op2 instructions: ISqrt(128-137), BitRev(138-147), RotL(188-203), RotR(204-219), Neg(220-239)
    // Op3 instructions: everything else
    bool is_op2 = (instr.opcode >= 128 && instr.opcode < 148) ||   // ISqrt, BitRev
                  (instr.opcode >= 188 && instr.opcode < 240);      // RotL, RotR, Neg
    bool is_op3 = !is_op2;
    if (is_op3) {
        if (instr.op2 < 5) {
            src2 = vm.regs[instr.r2];
            if (debug_mem) printf("[GPU] src2 register[%u]: 0x%016llx\n", instr.r2, (unsigned long long)src2);
        } else if (instr.op2 < 9) {
            if (debug_mem) printf("[GPU] src2 memory access:\n");
            src2 = mem_access64(vm, rom, instr.lit2, rom_size, debug_mem);
        } else if (instr.op2 < 13) {
            src2 = instr.lit2;
            if (debug_mem) printf("[GPU] src2 literal: 0x%016llx\n", (unsigned long long)src2);
        } else if (instr.op2 < 14) {
            src2 = special1_value64(vm.prog_digest_state);
            if (debug_mem) printf("[GPU] src2 special1: 0x%016llx\n", (unsigned long long)src2);
        } else {
            src2 = special2_value64(vm.mem_digest_state);
            if (debug_mem) printf("[GPU] src2 special2: 0x%016llx\n", (unsigned long long)src2);
        }
    } else {
        src2 = 0;  // Not used for Op2 instructions
    }
    
    if (debug_mem) {
        printf("[GPU] Computed src1=0x%016llx, src2=0x%016llx\n",
            (unsigned long long)src1, (unsigned long long)src2);
    }
    
    if (debug) {
        printf("               src1=0x%016llx, src2=0x%016llx\n",
            (unsigned long long)src1, (unsigned long long)src2);
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
        // BUG #14: CPU's Modulo operation actually does DIVISION (src1 / src2)
        // This is a copy-paste bug in the CPU, but we must match it!
        if (debug) {
            printf("               Modulo (BUG: actually Division): src2 != 0? %s\n", src2 != 0 ? "true" : "false");
        }
        result = (src2 != 0) ? src1 / src2 : special1_value64(vm.prog_digest_state);
        if (debug && src2 == 0) {
            printf("               Using special1_value64=0x%016llx\n", (unsigned long long)result);
        } else if (debug) {
            printf("               Division result (CPU bug): 0x%016llx / 0x%016llx = 0x%016llx\n",
                (unsigned long long)src1, (unsigned long long)src2, (unsigned long long)result);
        }
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
        // RotL - handle edge case where r1=0 would cause shift by 64
        uint32_t shift = instr.r1 & 0x3F;
        result = shift == 0 ? src1 : ((src1 << shift) | (src1 >> (64 - shift)));
    } else if (instr.opcode < 220) {
        // RotR - handle edge case where r1=0 would cause shift by 64
        uint32_t shift = instr.r1 & 0x3F;
        result = shift == 0 ? src1 : ((src1 >> shift) | (src1 << (64 - shift)));
    } else if (instr.opcode < 240) {
        result = ~src1;
    } else if (instr.opcode < 248) {
        result = src1 & src2;
    } else {
        // Blake2b Hash operation (opcode 248-255)
        // The v parameter (opcode - 248) selects which 8-byte chunk from the 64-byte output
        uint8_t v = instr.opcode - 248;  // v is 0-7
        
        uint8_t input[16];
        for (int i = 0; i < 8; ++i) {
            input[i] = (src1 >> (i * 8)) & 0xFF;
        }
        for (int i = 0; i < 8; ++i) {
            input[8 + i] = (src2 >> (i * 8)) & 0xFF;
        }
        
        if (debug_mem) {
            printf("[GPU] Blake2b Hash operation (v=%u):\n", v);
            printf("  Input (16 bytes): ");
            for (int i = 0; i < 16; i++) printf("%02x", input[i]);
            printf("\n");
        }
        
        uint8_t hash_out[64];
        blake2b(hash_out, 64, input, 16);
        
        if (debug_mem) {
            printf("  Hash output (64 bytes):\n");
            for (int i = 0; i < 64; i++) {
                if (i % 16 == 0 && i > 0) printf("\n");
                if (i % 16 == 0) printf("    ");
                printf("%02x", hash_out[i]);
            }
            printf("\n");
        }
        
        // Select the v-th 8-byte chunk (v ranges from 0 to 7)
        uint32_t offset = v * 8;
        result = ((uint64_t)hash_out[offset + 0] << 0) |
                 ((uint64_t)hash_out[offset + 1] << 8) |
                 ((uint64_t)hash_out[offset + 2] << 16) |
                 ((uint64_t)hash_out[offset + 3] << 24) |
                 ((uint64_t)hash_out[offset + 4] << 32) |
                 ((uint64_t)hash_out[offset + 5] << 40) |
                 ((uint64_t)hash_out[offset + 6] << 48) |
                 ((uint64_t)hash_out[offset + 7] << 56);
        
        if (debug_mem) {
            printf("  Extracted chunk %u (offset %u): 0x%016llx\n", v, offset, (unsigned long long)result);
        }
    }

    vm.regs[instr.r3] = result;
    
    if (debug) {
        printf("               After: regs[%u]=0x%016llx\n",
            instr.r3, (unsigned long long)vm.regs[instr.r3]);
    }
    
    vm.ip = vm.ip + 1;
    
    // Update prog_digest with the instruction chunk (20 bytes)
    blake2b_update(vm.prog_digest_state, prog_chunk, INSTR_SIZE);
}

extern "C" __global__ void ashmaize_hash_kernel(
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

    const uint8_t* salt = salts + tid * salt_len;
    uint8_t* program = programs + tid * program_size;
    uint8_t* result = results + tid * 64;

    VMState vm;
    vm_init(vm, rom_digest, 64, salt, salt_len);

    uint32_t loop_mem_prev = 0;
    for (uint32_t loop = 0; loop < nb_loops; ++loop) {
        hprime(program, program_size, vm.prog_seed, 64);

        for (uint32_t instr_idx = 0; instr_idx < nb_instrs; ++instr_idx) {
            // Use instr_idx because program gets reshuffled each loop (hprime above)
            execute_one_instruction(vm, rom_data, program + instr_idx * INSTR_SIZE, rom_size, false);
        }

        post_instructions(vm);
        
        // Debug disabled - Bug #14 fixed!
        loop_mem_prev = vm.memory_counter;
    }

    Blake2bState final_prog_state = vm.prog_digest_state;
    uint8_t prog_final[64];
    blake2b_final(final_prog_state, prog_final, 64);

    Blake2bState final_mem_state = vm.mem_digest_state;
    uint8_t mem_final[64];
    blake2b_final(final_mem_state, mem_final, 64);

    // Finalize: blake2b(prog_digest || mem_digest || memory_counter || all_regs)
    uint8_t final_input[64 + 64 + 4 + (NB_REGS * 8)];
    int pos = 0;
    
    for (int i = 0; i < 64; ++i) {
        final_input[pos++] = prog_final[i];
    }
    for (int i = 0; i < 64; ++i) {
        final_input[pos++] = mem_final[i];
    }
    
    // memory_counter as little-endian u32
    final_input[pos++] = (uint8_t)(vm.memory_counter >> 0);
    final_input[pos++] = (uint8_t)(vm.memory_counter >> 8);
    final_input[pos++] = (uint8_t)(vm.memory_counter >> 16);
    final_input[pos++] = (uint8_t)(vm.memory_counter >> 24);
    
    // All registers as little-endian u64
    for (int i = 0; i < NB_REGS; ++i) {
        final_input[pos++] = (uint8_t)(vm.regs[i] >> 0);
        final_input[pos++] = (uint8_t)(vm.regs[i] >> 8);
        final_input[pos++] = (uint8_t)(vm.regs[i] >> 16);
        final_input[pos++] = (uint8_t)(vm.regs[i] >> 24);
        final_input[pos++] = (uint8_t)(vm.regs[i] >> 32);
        final_input[pos++] = (uint8_t)(vm.regs[i] >> 40);
        final_input[pos++] = (uint8_t)(vm.regs[i] >> 48);
        final_input[pos++] = (uint8_t)(vm.regs[i] >> 56);
    }
    
    blake2b(result, 64, final_input, pos);
}

