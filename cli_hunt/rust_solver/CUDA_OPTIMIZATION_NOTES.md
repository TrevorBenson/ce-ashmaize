# CUDA Optimization Notes

## Performance Profiling Guide

Use NVIDIA Nsight Compute to profile the CUDA kernel:

```bash
ncu --set full --export profile_report ./target/release/ashmaize-solver --use-gpu --address <addr> --challenge-id <id> --difficulty <diff> --no-pre-mine <npm> --latest-submission <ls> --no-pre-mine-hour <npmh>
```

## Key Optimization Opportunities

### 1. Thread Configuration

**Current Settings:**
- Threads per block: 256
- Blocks per grid: Dynamic based on num_hashes

**Optimization Targets:**
- Experiment with 128, 256, 512 threads per block
- Profile occupancy vs register pressure tradeoff
- Each thread is independent (embarrassingly parallel)

### 2. Memory Optimization

**ROM Data (~1GB):**
- Currently in global memory
- Consider texture memory for read-only ROM access
- Potential 2-3x speedup for memory-bound operations

**Program Buffer (~5KB per thread):**
- Currently in global memory per thread
- Could use shared memory if small enough
- Profile: shared vs global memory performance

**Constant Memory:**
- Blake2b IV and SIGMA already in constant memory
- Consider moving other read-only data

### 3. Register Pressure

**Current State:**
- VMState struct contains multiple Blake2bState structs
- Each Blake2bState has significant register usage (h[8], buf[128])

**Optimization:**
- Profile register usage per thread with `--metrics` in ncu
- Consider reducing local variables in hot paths
- May need to spill to local memory if registers exceed limit

### 4. Kernel Launch Configuration

**Batch Size:**
- Default: 1024 hashes per batch
- Profile optimal batch sizes: 512, 1024, 2048, 4096
- Balance: larger batches = better GPU utilization, but longer latency

**Stream Optimization:**
- Current implementation uses single stream
- Consider multiple streams for overlapped compute + memory transfer
- Pipeline: H2D transfer batch N while computing batch N-1

### 5. Blake2b Optimization

**Current:**
- Pure device implementation
- No use of specialized instructions

**Potential:**
- Use `__byte_perm()` for byte operations
- Leverage warp-level primitives if applicable
- Consider cublas or thrust for any parallel reductions

### 6. Argon2 H' Optimization

**Current:**
- Iterative implementation with multiple Blake2b calls
- Potential bottleneck for large output lengths

**Optimization:**
- Profile time spent in hprime
- Most common case is 64 bytes (fast path)
- Larger sizes (program_size ~5KB) trigger slow path

### 7. Instruction Execution

**Observation:**
- 256 instructions executed per loop
- 8 loops total = 2048 instruction executions per hash
- Each instruction involves operand fetch + execution + register write

**Optimization:**
- Profile branch divergence in execute_one_instruction
- Opcode distribution may cause divergence
- Consider separate kernels for different instruction types

### 8. Memory Access Patterns

**mem_access64 function:**
- Accesses ROM in 64-byte chunks
- Modulo operation: `addr % rom_size`
- Updates mem_digest_state

**Optimization:**
- Coalesce memory accesses across warp
- Consider prefetching ROM chunks
- Profile cache hit rates

## Expected Performance Targets

Based on Metal implementation benchmarks:

### CPU Baseline:
- Single thread: ~1.4 ms per hash (720 μs * ~2x for safety)
- 5 threads: ~280 hashes/sec

### GPU Targets:

**NVIDIA RTX 3080 (8704 CUDA cores):**
- Conservative: 5,000 H/s (18x speedup)
- Optimistic: 10,000 H/s (36x speedup)

**NVIDIA RTX 4090 (16384 CUDA cores):**
- Conservative: 10,000 H/s
- Optimistic: 20,000 H/s

**NVIDIA A100 (6912 CUDA cores, HBM2):**
- Conservative: 8,000 H/s
- Optimistic: 15,000 H/s
- High memory bandwidth advantage

### Bottleneck Analysis:

**If Memory-Bound:**
- ROM access is primary bottleneck
- Texture memory or caching improvements help
- Batch size matters less

**If Compute-Bound:**
- Blake2b compression rounds are bottleneck
- Register optimization critical
- Larger batches help

**If Launch-Bound:**
- Kernel launch overhead dominates
- Increase batch size significantly
- Use streams for overlap

## Profiling Checklist

1. **Kernel Duration:** Time per hash (target: <0.1 ms)
2. **Memory Throughput:** GB/s for ROM reads (compare to peak bandwidth)
3. **Occupancy:** Active warps / maximum warps
4. **Register Usage:** Registers per thread (target: <64 for good occupancy)
5. **Branch Divergence:** % of divergent branches in instruction execution
6. **Cache Hit Rate:** L1/L2 cache hit rates for ROM access
7. **Warp Execution Efficiency:** Stalls due to memory, execution, or synchronization

## Next Steps

1. Run baseline profiling with current implementation
2. Identify primary bottleneck (compute vs memory)
3. Implement top 3 optimizations based on profile data
4. Re-profile and iterate
5. Document final performance numbers

## Advanced Optimizations (Future)

- **Multi-GPU:** Distribute work across multiple GPUs
- **Kernel Fusion:** Combine multiple Blake2b calls into single kernel
- **Custom Hash Instructions:** If CUDA provides crypto acceleration
- **Dynamic Parallelism:** Nested kernel launches for program execution
- **PTX-Level Optimization:** Hand-tune critical paths

