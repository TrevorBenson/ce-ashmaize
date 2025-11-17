// Test VM initialization
// NOTE: blake2b.cuh, argon2.cuh, ashmaize_vm.cuh included by build script

extern "C" __global__ void test_vm_init_kernel(
    const uint8_t* rom_digest,
    const uint8_t* salt,
    uint32_t salt_len,
    uint64_t* output_regs,      // NB_REGS * 8 bytes
    uint8_t* output_prog_seed   // 64 bytes
) {
    uint32_t tid = blockIdx.x * blockDim.x + threadIdx.x;
    
    if (tid == 0) {
        VMState vm;
        vm_init(vm, rom_digest, 64, salt, salt_len);
        
        // Copy registers
        for (int i = 0; i < NB_REGS; ++i) {
            output_regs[i] = vm.regs[i];
        }
        
        // Copy prog_seed
        for (int i = 0; i < 64; ++i) {
            output_prog_seed[i] = vm.prog_seed[i];
        }
    }
}

