// Test Blake2b implementation with known test vectors
#include "blake2b.cuh"
#include <stdio.h>

extern "C" __global__ void test_blake2b_kernel(uint8_t* results) {
    uint32_t tid = blockIdx.x * blockDim.x + threadIdx.x;
    
    if (tid == 0) {
        // Test 1: Empty input
        uint8_t out1[64];
        blake2b(out1, 64, nullptr, 0);
        for (int i = 0; i < 64; ++i) {
            results[i] = out1[i];
        }
    }
    
    if (tid == 1) {
        // Test 2: "abc"
        uint8_t input2[] = {0x61, 0x62, 0x63};  // "abc"
        uint8_t out2[64];
        blake2b(out2, 64, input2, 3);
        for (int i = 0; i < 64; ++i) {
            results[64 + i] = out2[i];
        }
    }
    
    if (tid == 2) {
        // Test 3: "test"
        uint8_t input3[] = {0x74, 0x65, 0x73, 0x74};  // "test"
        uint8_t out3[64];
        blake2b(out3, 64, input3, 4);
        for (int i = 0; i < 64; ++i) {
            results[128 + i] = out3[i];
        }
    }
}

