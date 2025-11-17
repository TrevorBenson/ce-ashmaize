# Phase 6: Advanced Optimization Testing Plan

## Objective
Push performance beyond the current 286k H/s by testing advanced CUDA optimizations individually and in combinations, regardless of the original 90% target goal.

## Current Baseline
- **Configuration**: Async + Clone + 131,072 batch per GPU
- **Performance**: 285,996 H/s (4x RTX 4090)
- **Per GPU**: 71,499 H/s

## Advanced Optimizations to Test

### 1. CUDA Streams (Overlapping Compute + Data Transfer)
**Theory**: Use multiple streams per GPU to overlap data transfer with kernel execution.

**Expected Impact**: +5-15% (if memory bandwidth is a bottleneck)

**Implementation**:
```rust
// Create multiple streams per GPU
let streams_per_gpu = 2;  // Test: 2, 3, 4
for stream_id in 0..streams_per_gpu {
    // Each stream handles a sub-batch
    let sub_batch = batch_per_gpu / streams_per_gpu;
    // Launch kernel in stream with async data transfer
}
```

**Test File**: Create `multi_gpu_streams.rs`

**Test Duration**: 20 seconds per configuration
**Timeout**: 40 seconds

**Configurations to Test**:
- 1 stream (baseline)
- 2 streams per GPU
- 3 streams per GPU
- 4 streams per GPU

---

### 2. Pinned (Page-Locked) Memory
**Theory**: Use `cudaHostAlloc` for pinned memory to speed up data transfers between host and device.

**Expected Impact**: +3-10% (faster H2D/D2H transfers)

**Implementation**:
```rust
// Allocate pinned memory for salts and results
// Use cudarc's pinned memory allocation if available
// Or manage via custom CUDA allocator
```

**Test File**: Create `multi_gpu_pinned.rs`

**Test Duration**: 20 seconds
**Timeout**: 40 seconds

---

### 3. Unified Memory (Zero-Copy Revisited)
**Theory**: Previous zero-copy test used Arc<Vec>. Try CUDA Unified Memory instead.

**Expected Impact**: +0-5% (might be neutral or slightly positive with proper prefetching)

**Implementation**:
```rust
// Use CUDA Unified Memory (managed memory)
// With explicit prefetch hints to GPUs
cudaMemPrefetchAsync(ptr, size, device_id);
```

**Test File**: Create `multi_gpu_unified_memory.rs`

**Test Duration**: 20 seconds
**Timeout**: 40 seconds

---

### 4. Persistent Kernel (Grid-Stride Loop)
**Theory**: Launch kernel once with all GPU threads, keep them resident, feed work via atomic counters.

**Expected Impact**: +10-20% (eliminates kernel launch overhead entirely)

**Implementation**:
```cuda
__global__ void persistent_hash_kernel(
    WorkQueue* queue,
    AtomicCounter* work_counter,
    Results* results
) {
    // Each thread loops indefinitely
    while (true) {
        uint64_t work_id = atomicAdd(work_counter, 1);
        if (work_id >= total_work) break;
        
        // Process work_id
        // ...
    }
}
```

**Test File**: Create `multi_gpu_persistent_kernel.rs`

**Test Duration**: 20 seconds
**Timeout**: 40 seconds

**Note**: This is different from "persistent workers" (which was CPU-side thread management). This is GPU-side kernel persistence.

---

### 5. Kernel Fusion (Blake2b + Argon2 + VM in single kernel)
**Theory**: Current implementation might have multiple kernel launches. Fuse into one large kernel.

**Expected Impact**: +5-10% (reduced launch overhead if applicable)

**Investigation Required**:
- Check if current implementation already uses single kernel
- If using multiple kernels, fuse them

**Test File**: Verify current `ashmaize.cu`, potentially create `multi_gpu_fused.rs`

**Test Duration**: 20 seconds
**Timeout**: 40 seconds

---

### 6. Warp-Level Optimizations
**Theory**: Optimize VM execution for warp-level parallelism (32 threads execute in lockstep).

**Expected Impact**: +5-15% (better instruction throughput)

**Implementation**:
- Minimize warp divergence in conditionals
- Use warp-level primitives (`__shfl_*`, `__ballot_sync`)
- Align data structures to warp boundaries

**Test File**: Create optimized `ashmaize_warp.cu` and `multi_gpu_warp_opt.rs`

**Test Duration**: 20 seconds
**Timeout**: 40 seconds

---

### 7. Shared Memory for ROM/Program
**Theory**: Use shared memory for frequently accessed ROM chunks or program data.

**Expected Impact**: +10-20% (if memory access is bottleneck)

**Implementation**:
```cuda
__shared__ uint8_t shared_rom[ROM_CHUNK_SIZE];
__shared__ uint8_t shared_program[PROGRAM_SIZE];

// Load into shared memory collaboratively
// Use shared memory for reads
```

**Test File**: Create optimized `ashmaize_shared.cu` and `multi_gpu_shared_mem.rs`

**Test Duration**: 20 seconds
**Timeout**: 40 seconds

---

### 8. Texture Memory for ROM
**Theory**: Use texture cache for ROM access (read-only, cached, 2D locality).

**Expected Impact**: +5-15% (better cache utilization for ROM reads)

**Implementation**:
```cuda
texture<uint8_t, 1, cudaReadModeElementType> rom_texture;

// Bind ROM to texture
cudaBindTexture(0, rom_texture, rom_data, rom_size);

// Read via tex1Dfetch
uint8_t value = tex1Dfetch(rom_texture, addr);
```

**Test File**: Create optimized `ashmaize_texture.cu` and `multi_gpu_texture.rs`

**Test Duration**: 20 seconds
**Timeout**: 40 seconds

---

### 9. Constant Memory for VM Config
**Theory**: Use constant memory for read-only VM configuration (faster than global memory).

**Expected Impact**: +2-5% (small improvement)

**Implementation**:
```cuda
__constant__ uint32_t nb_loops;
__constant__ uint32_t nb_instrs;
__constant__ uint8_t rom_digest[64];
```

**Test File**: Modify `ashmaize.cu` and test with `multi_gpu_const_mem.rs`

**Test Duration**: 20 seconds
**Timeout**: 40 seconds

---

### 10. Dynamic Parallelism (Nested Kernel Launches)
**Theory**: Launch child kernels from GPU for adaptive work distribution.

**Expected Impact**: +0-5% (unlikely to help, but worth testing)

**Implementation**:
```cuda
__global__ void parent_kernel() {
    // Conditionally launch child kernels based on work
    if (condition) {
        child_kernel<<<blocks, threads>>>(args);
    }
}
```

**Test File**: Create `multi_gpu_dynamic.rs`

**Test Duration**: 20 seconds
**Timeout**: 40 seconds

**Note**: Requires compute capability 7.5+ (RTX 2000+), which we have.

---

## Testing Methodology

### Phase 6A: Individual Testing (10 tests)
Test each optimization individually against the baseline (Async + Clone + 131k batch).

**For each optimization**:
1. Implement test file
2. Run 20-second test with 40-second timeout
3. Record hashrate
4. Calculate improvement vs baseline (285,996 H/s)
5. Decision: Keep if > 0% improvement, Skip if negative

**Expected Timeline**: ~4-5 hours (10 tests * 20-30 minutes each)

### Phase 6B: Combination Testing
After Phase 6A, combine ALL positive optimizations and test combinations systematically.

**Strategy**: Start with top 3 performers, add one optimization at a time.

**Example Combinations** (assuming hypothetical results):
- Baseline: 286k H/s
- + Persistent Kernel: 315k H/s (+10%)
- + Shared Memory: 346k H/s (+20% over baseline)
- + Texture Memory: 380k H/s (+33% over baseline)
- + Warp Opts: 410k H/s (+43% over baseline)
- etc.

**Test Matrix**:
```
Test all 2-way combinations of positive optimizations
Test all 3-way combinations of top 5 optimizations  
Test all 4-way combinations of top 3 optimizations
Test full combination of all positive optimizations
```

**Timeout Handling**:
- Each test: 20 seconds expected, 40 seconds timeout
- If test hangs at 40 seconds, kill and mark as FAIL
- Continue to next test

---

## Implementation Priority

### Tier 1: High Expected Impact (Test First)
1. ✅ **Persistent Kernel** - Eliminates launch overhead entirely
2. ✅ **Shared Memory** - Frequently accessed data
3. ✅ **CUDA Streams** - Overlap compute + transfer

### Tier 2: Medium Expected Impact
4. ✅ **Texture Memory** - ROM access optimization
5. ✅ **Warp-Level Opts** - Instruction throughput
6. ✅ **Pinned Memory** - Faster H2D/D2H

### Tier 3: Lower Expected Impact (Test Last)
7. ✅ **Constant Memory** - Small config data
8. ✅ **Unified Memory** - Revisit zero-copy
9. ⏸️ **Kernel Fusion** - Need to verify current state first
10. ⏸️ **Dynamic Parallelism** - Unlikely to help

---

## Test File Template

All test files should follow this structure for consistency:

```rust
// Phase 6: [Optimization Name]
// Expected: +X% improvement
// Test Duration: 20s, Timeout: 40s

use ashmaize::{Rom, RomGenerationType};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::thread;
use std::time::{Instant, Duration};

#[cfg(feature = "cuda")]
use ashmaize_solver::gpu::CudaAshmaize;

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * MB;
const TEST_DURATION: u64 = 20;
const TIMEOUT_SECS: u64 = 40;

fn main() {
    #[cfg(not(feature = "cuda"))]
    {
        eprintln!("This example requires the 'cuda' feature");
        std::process::exit(1);
    }

    #[cfg(feature = "cuda")]
    {
        println!("=== Phase 6: [Optimization Name] ===\n");
        
        // Setup
        let _init = CudaAshmaize::new().expect("CUDA init failed");
        let gpu_count = CudaAshmaize::get_device_count().expect("Failed to get GPU count");
        
        let rom = Arc::new(Rom::new(
            b"0",
            RomGenerationType::TwoStep {
                pre_size: 16 * MB,
                mixing_numbers: 4,
            },
            1 * GB,
        ));
        
        let batch_per_gpu = 131_072;  // Optimal from Phase 4
        
        println!("Configuration:");
        println!("  GPUs: {}", gpu_count);
        println!("  Batch per GPU: {}", batch_per_gpu);
        println!("  Duration: {}s", TEST_DURATION);
        println!("  Timeout: {}s\n", TIMEOUT_SECS);
        
        // Run test with timeout
        let result = run_test_with_timeout(gpu_count, batch_per_gpu, rom, TEST_DURATION, TIMEOUT_SECS);
        
        match result {
            Some(hashrate) => {
                let baseline = 285_996.0;
                let improvement = ((hashrate - baseline) / baseline) * 100.0;
                
                println!("\n=== Results ===");
                println!("Hashrate: {:.2} H/s", hashrate);
                println!("Baseline: {:.2} H/s", baseline);
                println!("Change: {:+.2}% ({:+.2} H/s)", improvement, hashrate - baseline);
                
                if improvement > 0.0 {
                    println!("✅ IMPROVEMENT - Keep for combination testing");
                } else {
                    println!("❌ NO IMPROVEMENT - Skip");
                }
            }
            None => {
                println!("❌ TEST TIMEOUT - Hung after {}s", TIMEOUT_SECS);
            }
        }
    }
}

#[cfg(feature = "cuda")]
fn run_test_with_timeout(
    gpu_count: usize,
    batch_per_gpu: usize,
    rom: Arc<Rom>,
    duration: u64,
    timeout: u64
) -> Option<f64> {
    use std::sync::mpsc;
    
    let (tx, rx) = mpsc::channel();
    
    let test_thread = thread::spawn(move || {
        // Implement optimization-specific test logic here
        // Return hashrate
        let hashrate = run_optimization_test(gpu_count, batch_per_gpu, rom, duration);
        tx.send(hashrate).ok();
    });
    
    // Wait for result with timeout
    match rx.recv_timeout(Duration::from_secs(timeout)) {
        Ok(hashrate) => {
            test_thread.join().ok();
            Some(hashrate)
        }
        Err(_) => {
            println!("Warning: Test exceeded timeout, terminating...");
            // Thread will be dropped/killed
            None
        }
    }
}

#[cfg(feature = "cuda")]
fn run_optimization_test(
    gpu_count: usize,
    batch_per_gpu: usize,
    rom: Arc<Rom>,
    duration: u64
) -> f64 {
    // Implement optimization-specific logic
    // Return final hashrate
    0.0
}
```

---

## Success Criteria

**Individual Test**:
- ✅ KEEP: Improvement > 0%
- ❌ SKIP: Improvement ≤ 0% or timeout

**Combination Test**:
- ✅ KEEP: Combined improvement > sum of individual improvements (synergy)
- ⚠️  EVALUATE: Combined improvement ≈ sum of individual (no synergy, but additive)
- ❌ SKIP: Combined improvement < sum of individual (negative interaction)

---

## Expected Outcomes

### Conservative Scenario (5-10% total improvement)
- Final: ~300-315k H/s
- Per GPU: ~75-79k H/s

### Moderate Scenario (15-25% total improvement)
- Final: ~330-360k H/s
- Per GPU: ~82-90k H/s

### Optimistic Scenario (30-50% total improvement)
- Final: ~372-430k H/s
- Per GPU: ~93-107k H/s

### Breakthrough Scenario (50%+ improvement)
- Final: 430k+ H/s
- Per GPU: 107k+ H/s

---

## Status

**Phase 6A (Individual Testing)**: ⏳ PENDING
**Phase 6B (Combination Testing)**: ⏳ PENDING
**Estimated Completion**: 6-8 hours of testing

**Current Baseline**: 285,996 H/s (4x RTX 4090)
**Target**: Maximize performance (no upper limit)

---

**Next Step**: Begin with Tier 1 optimizations (Persistent Kernel, Shared Memory, CUDA Streams)

