// Test Argon2 H' (hprime) implementation
// Note: blake2b.cuh and argon2.cuh are included by the build script

extern "C" __global__ void test_hprime_kernel(
    const uint8_t* input,
    uint32_t input_len,
    uint8_t* output,
    uint32_t output_len
) {
    uint32_t tid = blockIdx.x * blockDim.x + threadIdx.x;
    
    if (tid == 0) {
        // Call hprime
        hprime(output, output_len, input, input_len);
    }
}

