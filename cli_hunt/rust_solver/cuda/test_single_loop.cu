// Test a single loop iteration
// NOTE: includes added by build script

// Forward declare the functions we need (they're in ashmaize_vm.cuh and ashmaize.cu)
__device__ void execute_one_instruction(VMState &vm, const uint8_t *rom, const uint8_t *prog_chunk, uint32_t rom_size);
__device__ void post_instructions(VMState &vm);

extern "C" __global__ void test_single_loop_kernel(
    const uint8_t* rom_data,
    uint32_t rom_size,
    const uint8_t* rom_digest,
    const uint8_t* salt,
    uint32_t salt_len,
    uint32_t nb_instrs,
    uint32_t program_size,
    uint64_t* output_regs,      // NB_REGS values
    uint8_t* output_prog_seed,  // 64 bytes
    uint32_t* output_mem_counter
) {
    uint32_t tid = blockIdx.x * blockDim.x + threadIdx.x;
    
    if (tid == 0) {
        // Initialize VM
        VMState vm;
        vm_init(vm, rom_digest, 64, salt, salt_len);
        
        // Allocate program buffer
        uint8_t program[5120];  // Max: 256 * 20
        
        // Execute ONE loop iteration
        hprime(program, program_size, vm.prog_seed, 64);
        
        for (uint32_t instr_idx = 0; instr_idx < nb_instrs; ++instr_idx) {
            execute_one_instruction(vm, rom_data, program + instr_idx * INSTR_SIZE, rom_size);
        }
        
        post_instructions(vm);
        
        // Copy output
        for (int i = 0; i < NB_REGS; ++i) {
            output_regs[i] = vm.regs[i];
        }
        
        for (int i = 0; i < 64; ++i) {
            output_prog_seed[i] = vm.prog_seed[i];
        }
        
        *output_mem_counter = vm.memory_counter;
    }
}

