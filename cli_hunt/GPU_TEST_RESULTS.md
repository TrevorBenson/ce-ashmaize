# GPU Testing Results - CUDA Implementation

**Test Date**: November 17, 2025  
**Test Environment**: scavvast03 (vast.ai cloud server)  
**GPUs**: 4x NVIDIA GeForce RTX 4090  
**CUDA Version**: 12.6  
**Driver Version**: 560.35.05

## Summary

✅ **GPU CUDA Initialization**: **WORKING**  
✅ **PTX Loading**: **WORKING**  
✅ **Kernel Execution**: **WORKING**  
⚠️ **Hash Correctness**: **NEEDS DEBUG** (GPU hashes differ from CPU)  
✅ **Hashrate Measurement**: **WORKING**

## Initial Hashrate Benchmark

**Single RTX 4090 Performance**:
- **Hashrate**: ~7,864 H/s
- **Batch Size**: 2,048 hashes
- **Duration**: 10 seconds
- **Total Hashes**: 79,872

## Issues Resolved

### 1. "named symbol not found" Error - **SOLVED** ✅

**Root Cause**: CUDA kernel function was using C++ name mangling  
**Solution**: Added `extern "C"` linkage to kernel function in `cuda/ashmaize.cu`

```cuda
extern "C" __global__ void ashmaize_hash_kernel(...) {
    // kernel code
}
```

### 2. cudarc Dependency - **SOLVED** ✅

**Issue**: Multiple configuration issues with cudarc  
**Solution**: 
- Used `cudarc = "0.12"` with `cuda-12060` feature (exact CUDA 12.6 match)
- Removed `cudart` static linking from `build.rs` (conflicted with cudarc dynamic loading)
- Enabled `driver` and `nvrtc` features

### 3. PTX Compilation - **SOLVED** ✅

**Issue**: PTX was being recompiled at runtime instead of using pre-compiled version  
**Solution**: Modified `compile_kernel()` to use `include_str!(concat!(env!("OUT_DIR"), "/ashmaize.ptx"))`

### 4. GPU Architecture - **SOLVED** ✅

**Issue**: Code was compiled for `sm_75` (Turing) instead of `sm_89` (Ada Lovelace)  
**Solution**: Updated `build.rs` and `gpu.rs` to use `-arch=sm_89` for RTX 4090

## Outstanding Issues

### Hash Mismatch Between CPU and GPU

**Test Case**: nonce=0, standard parameters  
**CPU Result**: `71f12529d9f14371...`  
**GPU Result**: `5894b3d2077ce710...`

**Possible Causes**:
1. Endianness differences in data handling
2. Salt padding/length handling mismatch
3. Initial VM state differences
4. BLAKE2b implementation differences
5. Argon2 H' function differences
6. VM instruction execution differences

**Next Steps**:
1. Add debug output to compare intermediate states
2. Verify BLAKE2b produces same results for known test vectors
3. Check Argon2 H' function output
4. Validate VM register initialization
5. Compare instruction-by-instruction execution

## Multi-GPU Status

**Available GPUs**: 4x RTX 4090  
**Multi-GPU Support**: Not yet tested  
**Next Steps**:
- Implement multi-GPU support in Rust bindings
- Test parallel execution across all 4 GPUs
- Measure aggregate hashrate

**Estimated Multi-GPU Performance** (if linear scaling):
- 4x RTX 4090: ~31,456 H/s (theoretical)

## Configuration Used

### Cargo.toml
```toml
cudarc = { version = "0.12", features = ["cuda-12060", "driver", "nvrtc"], default-features = false, optional = true }
```

### build.rs
```rust
// PTX compilation with sm_89 architecture
.args(&[
    "-ptx",
    "-arch=sm_89",
    "-I", cuda_dir.to_str().unwrap(),
    "-o", ptx_path.to_str().unwrap(),
    cuda_dir.join("ashmaize.cu").to_str().unwrap(),
])
```

### Key Fixes
1. `extern "C"` in CUDA kernel
2. No static cudart linking
3. Pre-compiled PTX loading
4. Correct GPU architecture (sm_89)

## Performance Notes

- Batch size of 2,048 appears optimal for initial testing
- GPU memory usage is low (< 100MB per batch)
- Kernel launch overhead is minimal
- Room for optimization in thread block configuration

## Recommendations

1. **Priority 1**: Debug and fix hash correctness issue
2. **Priority 2**: Implement multi-GPU support (4x GPUs available)
3. **Priority 3**: Optimize kernel parameters (threads per block, batch size)
4. **Priority 4**: Profile with Nsight Compute for bottlenecks
5. **Priority 5**: Test with larger ROM sizes and different parameters

## Test Commands

```bash
# Single hash test
./ashmaize-solver test-hash --address addr_test1vr6w0ja6rjz8gc3wvn6hh0v0chcsa2tg2ymzmz9e9x9 \
  --challenge-id 0000000000000000000000000000000000000000000000000000000000000001 \
  --difficulty 12 --no-pre-mine 0 --latest-submission 0 --no-pre-mine-hour 0 --nonce 12345

# Hashrate benchmark  
./ashmaize-solver benchmark --address addr_test1vr6w0ja6rjz8gc3wvn6hh0v0chcsa2tg2ymzmz9e9x9 \
  --challenge-id 0000000000000000000000000000000000000000000000000000000000000001 \
  --difficulty 12 --no-pre-mine 0 --latest-submission 0 --no-pre-mine-hour 0 \
  --duration-secs 10 --batch-size 2048
```

## Conclusion

The CUDA implementation is **functional and executing on GPU hardware**. The primary remaining task is debugging the hash correctness issue. Once resolved, the implementation should provide significant performance improvements over CPU-only solving, especially when utilizing all 4 available RTX 4090 GPUs.

**Estimated time to fix hash correctness**: 2-4 hours of debugging  
**Estimated time for multi-GPU support**: 1-2 hours after correctness fix  
**Total estimated performance gain**: 10-50x over CPU (depending on CPU baseline)

