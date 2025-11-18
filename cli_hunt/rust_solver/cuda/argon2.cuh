#ifndef ARGON2_CUH
#define ARGON2_CUH

#include "blake2b.cuh"

__device__ void hprime(uint8_t *output, uint32_t output_len, const uint8_t *input, uint32_t input_len) {
    if (output_len <= 64) {
        uint8_t temp[BLAKE2B_OUTBYTES + 4 + 512];
        temp[0] = output_len & 0xFF;
        temp[1] = (output_len >> 8) & 0xFF;
        temp[2] = (output_len >> 16) & 0xFF;
        temp[3] = (output_len >> 24) & 0xFF;

        for (uint32_t i = 0; i < input_len; ++i) {
            temp[4 + i] = input[i];
        }

        blake2b(output, output_len, temp, 4 + input_len);
        return;
    }

    uint32_t output_len_copy = output_len;

    uint8_t v0_input[BLAKE2B_OUTBYTES + 512];
    v0_input[0] = output_len_copy & 0xFF;
    v0_input[1] = (output_len_copy >> 8) & 0xFF;
    v0_input[2] = (output_len_copy >> 16) & 0xFF;
    v0_input[3] = (output_len_copy >> 24) & 0xFF;
    for (uint32_t i = 0; i < input_len; ++i) {
        v0_input[4 + i] = input[i];
    }

    uint8_t v0_hash[64];
    blake2b(v0_hash, 64, v0_input, 4 + input_len);

    uint8_t vi_prev[64];
    for (int i = 0; i < 64; ++i) {
        vi_prev[i] = v0_hash[i];
    }

    for (int i = 0; i < 32 && (uint32_t)i < output_len; ++i) {
        output[i] = vi_prev[i];
    }

    uint32_t bytes = output_len - 32;
    uint32_t pos = 32;

    while (bytes > 64) {
        uint8_t vi_hash[64];
        blake2b(vi_hash, 64, vi_prev, 64);

        for (int i = 0; i < 64; ++i) {
            vi_prev[i] = vi_hash[i];
        }

        for (int i = 0; i < 32; ++i) {
            if (pos + i < output_len) {
                output[pos + i] = vi_prev[i];
            }
        }

        bytes -= 32;
        pos += 32;
    }

    if (bytes > 0) {
        uint8_t temp[64];
        blake2b(temp, bytes, vi_prev, 64);
        for (uint32_t i = 0; i < bytes; ++i) {
            if (pos + i < output_len) {
                output[pos + i] = temp[i];
            }
        }
    }
}

#endif

