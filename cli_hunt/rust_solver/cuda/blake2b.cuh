#ifndef BLAKE2B_CUH
#define BLAKE2B_CUH

#include <stdint.h>

#define BLAKE2B_BLOCKBYTES 128
#define BLAKE2B_OUTBYTES 64

__constant__ uint32_t SIGMA[160] = {
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
    14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3,
    11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4,
    7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8,
    9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13,
    2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9,
    12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11,
    13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10,
    6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5,
    10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0
};

__constant__ uint64_t blake2b_IV[8] = {
    0x6a09e667f3bcc908ULL, 0xbb67ae8584caa73bULL,
    0x3c6ef372fe94f82bULL, 0xa54ff53a5f1d36f1ULL,
    0x510e527fade682d1ULL, 0x9b05688c2b3e6c1fULL,
    0x1f83d9abfb41bd6bULL, 0x5be0cd19137e2179ULL
};

struct Blake2bState {
    uint64_t h[8];
    uint64_t t[2];
    uint64_t f[2];
    uint8_t buf[BLAKE2B_BLOCKBYTES];
    uint64_t buflen;
    bool last_node;
};

__device__ inline void blake2b_G(uint64_t &a, uint64_t &b, uint64_t &c, uint64_t &d, uint64_t x, uint64_t y) {
    a = a + b + x;
    d = (d ^ a);
    d = (d >> 32) | (d << 32);
    c = c + d;
    b = (b ^ c);
    b = (b >> 24) | (b << 40);
    a = a + b + y;
    d = (d ^ a);
    d = (d >> 16) | (d << 48);
    c = c + d;
    b = (b ^ c);
    b = (b >> 63) | (b << 1);
}

__device__ void blake2b_round(Blake2bState &S, const uint64_t *m) {
    uint64_t v[16];

    for (int i = 0; i < 8; ++i) v[i] = S.h[i];
    for (int i = 0; i < 8; ++i) v[i + 8] = blake2b_IV[i];

    v[12] ^= S.t[0];
    v[13] ^= S.t[1];
    v[14] ^= S.f[0];
    v[15] ^= S.f[1];

    for (int r = 0; r < 12; ++r) {
        uint32_t s_idx = (r % 10) * 16;

        blake2b_G(v[0], v[4], v[8], v[12], m[SIGMA[s_idx + 0]], m[SIGMA[s_idx + 1]]);
        blake2b_G(v[1], v[5], v[9], v[13], m[SIGMA[s_idx + 2]], m[SIGMA[s_idx + 3]]);
        blake2b_G(v[2], v[6], v[10], v[14], m[SIGMA[s_idx + 4]], m[SIGMA[s_idx + 5]]);
        blake2b_G(v[3], v[7], v[11], v[15], m[SIGMA[s_idx + 6]], m[SIGMA[s_idx + 7]]);
        blake2b_G(v[0], v[5], v[10], v[15], m[SIGMA[s_idx + 8]], m[SIGMA[s_idx + 9]]);
        blake2b_G(v[1], v[6], v[11], v[12], m[SIGMA[s_idx + 10]], m[SIGMA[s_idx + 11]]);
        blake2b_G(v[2], v[7], v[8], v[13], m[SIGMA[s_idx + 12]], m[SIGMA[s_idx + 13]]);
        blake2b_G(v[3], v[4], v[9], v[14], m[SIGMA[s_idx + 14]], m[SIGMA[s_idx + 15]]);
    }

    for (int i = 0; i < 8; ++i) {
        S.h[i] ^= v[i] ^ v[i + 8];
    }
}

__device__ void blake2b_init(Blake2bState &S, uint32_t outlen) {
    for (int i = 0; i < 8; ++i) {
        S.h[i] = blake2b_IV[i];
    }
    S.h[0] ^= 0x01010000ULL ^ (uint64_t)outlen;
    S.t[0] = S.t[1] = S.f[0] = S.f[1] = 0;
    S.buflen = 0;
    S.last_node = false;
}

__device__ void blake2b_update(Blake2bState &S, const uint8_t *in, uint64_t inlen) {
    const uint8_t *current_in = in;
    while (inlen > 0) {
        uint64_t left = S.buflen;
        uint64_t fill = BLAKE2B_BLOCKBYTES - left;

        if (inlen > fill) {
            for (uint32_t i = 0; i < fill; ++i) {
                S.buf[left + i] = current_in[i];
            }
            S.buflen += fill;

            S.t[0] += BLAKE2B_BLOCKBYTES;
            if (S.t[0] < BLAKE2B_BLOCKBYTES) S.t[1]++;

            uint64_t m[16];
            for (uint32_t i = 0; i < 16; ++i) {
                m[i] = ((uint64_t)S.buf[i * 8 + 0] << 0) |
                       ((uint64_t)S.buf[i * 8 + 1] << 8) |
                       ((uint64_t)S.buf[i * 8 + 2] << 16) |
                       ((uint64_t)S.buf[i * 8 + 3] << 24) |
                       ((uint64_t)S.buf[i * 8 + 4] << 32) |
                       ((uint64_t)S.buf[i * 8 + 5] << 40) |
                       ((uint64_t)S.buf[i * 8 + 6] << 48) |
                       ((uint64_t)S.buf[i * 8 + 7] << 56);
            }

            blake2b_round(S, m);

            current_in += fill;
            inlen -= fill;
            S.buflen = 0;
        } else {
            for (uint32_t i = 0; i < inlen; ++i) {
                S.buf[left + i] = current_in[i];
            }
            S.buflen += inlen;
            inlen = 0;
        }
    }
}

__device__ void blake2b_final(Blake2bState &S, uint8_t *out, uint32_t outlen) {
    uint64_t lastblock = S.buflen;
    S.t[0] += lastblock;
    if (S.t[0] < lastblock) S.t[1]++;

    S.f[0] = ~0ULL;

    if (S.buflen > 0) {
        for (uint8_t i = (uint8_t)lastblock; i < BLAKE2B_BLOCKBYTES; ++i) {
            S.buf[i] = 0;
        }

        uint64_t m[16];
        for (uint32_t i = 0; i < 16; ++i) {
            m[i] = ((uint64_t)S.buf[i * 8 + 0] << 0) |
                   ((uint64_t)S.buf[i * 8 + 1] << 8) |
                   ((uint64_t)S.buf[i * 8 + 2] << 16) |
                   ((uint64_t)S.buf[i * 8 + 3] << 24) |
                   ((uint64_t)S.buf[i * 8 + 4] << 32) |
                   ((uint64_t)S.buf[i * 8 + 5] << 40) |
                   ((uint64_t)S.buf[i * 8 + 6] << 48) |
                   ((uint64_t)S.buf[i * 8 + 7] << 56);
        }

        blake2b_round(S, m);
    }

    for (uint32_t i = 0; i < 8; ++i) {
        uint64_t h = S.h[i];
        out[i * 8 + 0] = (uint8_t)(h >> 0);
        out[i * 8 + 1] = (uint8_t)(h >> 8);
        out[i * 8 + 2] = (uint8_t)(h >> 16);
        out[i * 8 + 3] = (uint8_t)(h >> 24);
        out[i * 8 + 4] = (uint8_t)(h >> 32);
        out[i * 8 + 5] = (uint8_t)(h >> 40);
        out[i * 8 + 6] = (uint8_t)(h >> 48);
        out[i * 8 + 7] = (uint8_t)(h >> 56);
    }
}

__device__ void blake2b(uint8_t *out, uint32_t outlen, const uint8_t *in, uint32_t inlen) {
    Blake2bState S;
    blake2b_init(S, outlen);
    blake2b_update(S, in, inlen);
    blake2b_final(S, out, outlen);
}

#endif

