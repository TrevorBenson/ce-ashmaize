# Phase 6: Efficient Testing Plan

**Goal**: Test all feasible Phase 6 optimizations systematically  
**Priority**: Hash correctness > Performance  
**Method**: Incremental kernel modifications with validation

---

## Optimizations to Test (Prioritized by Ease)

### Tier 1: Kernel Parameter Tuning (30 min each)

These can be tested by modifying launch parameters or adding simple directives:

1. **`maxrregcount` Directive** (Easy)
   - Add `__launch_bounds__` with `maxrregcount`
   - Force specific register usage
   - Test: 128, 192, 224 registers
   
2. **Occupancy Tuning** (Easy)
   - Test different threads/block with occupancy hints
   - Already done partially, but can test with hints
   
3. **Unroll Pragmas** (Easy)
   - Add `#pragma unroll` to instruction loop
   - May reduce loop overhead

### Tier 2: Memory Access Optimizations (1-2 hours each)

4. **Constant Memory for ROM Digest** (Medium)
   - Move `rom_digest` to `__constant__` memory
   - 32 bytes, frequently accessed
   - Should be easy win

5. **Vectorized Loads** (Medium)
   - Use `float4` or `uint4` for ROM/salt loading
   - May improve memory bandwidth

6. **Aligned Memory Access** (Easy)
   - Ensure all memory accesses are aligned
   - Add `__align__` directives

### Tier 3: Instruction-Level Optimizations (2-3 hours each)

7. **Warp-Level Optimizations** (Medium-Hard)
   - Minimize branch divergence
   - Use `__any()`, `__all()` for conditionals
   - Reorder to align branches

8. **Inline Functions** (Easy)
   - Add `__forceinline__` to hot functions
   - Reduce function call overhead

### Tier 4: Advanced (If needed, 4+ hours each)

9. **Shared Memory** (Hard - already attempted)
10. **Texture Memory** (Hard)
11. **Persistent Kernel** (Very Hard)

---

## Testing Protocol

### For Each Optimization:

**Step 1: Baseline Capture** (if not done)
```bash
cargo run --release --features cuda --example minimal_hash_test
# Save output as baseline
```

**Step 2: Implement Optimization**
- Modify kernel with clear comments
- Keep changes minimal and focused

**Step 3: Hash Correctness Test** ⚠️ CRITICAL
```bash
cargo run --release --features cuda --example minimal_hash_test
# Compare with baseline
# If different: INVALID - revert or fix
# If same: VALID - proceed to Step 4
```

**Step 4: Performance Test**
```bash
cargo run --release --features cuda --example multi_gpu_final
# Run 3x for statistical confidence
# Record: min, max, avg hashrate
```

**Step 5: Decision**
- If +5% or more: KEEP
- If +1% to +5%: KEEP (marginal)
- If -2% to +1%: NEUTRAL (keep if no downsides)
- If -2% or worse: DISCARD

**Step 6: Document**
- Update results file with findings
- Mark as kept/discarded
- Note any side effects

---

## Quick Wins to Start With

Let me implement the EASIEST optimizations first to build momentum:

### Test 1: Constant Memory for ROM Digest (15 min)

**Change**:
```cuda
__constant__ uint8_t rom_digest_const[32];

// In kernel:
blake2b_update(&final_state, rom_digest_const, 32);
```

**Expected**: +2-5% (ROM digest accessed frequently)
**Risk**: Very Low
**Hash Correctness**: Should not change

### Test 2: Inline Function Hints (10 min)

**Change**:
```cuda
__device__ __forceinline__ uint64_t mem_access64(...)
__device__ __forceinline__ void execute_one_instruction(...)
```

**Expected**: +1-3% (reduce call overhead)
**Risk**: Very Low
**Hash Correctness**: Should not change

### Test 3: Loop Unrolling (10 min)

**Change**:
```cuda
#pragma unroll 8
for (uint32_t instr_idx = 0; instr_idx < nb_instrs; ++instr_idx) {
```

**Expected**: +2-8% (reduce loop overhead)
**Risk**: Low (may increase register pressure)
**Hash Correctness**: Should not change

---

## Implementation Order (Next 3-4 Hours)

1. ✅ Baseline established (275k H/s)
2. ⏳ Test 1: Constant Memory (30 min)
   - Implement, verify correctness, test performance
3. ⏳ Test 2: Inline Hints (20 min)
   - Implement, verify correctness, test performance
4. ⏳ Test 3: Loop Unrolling (30 min)
   - Implement, verify correctness, test performance
5. ⏳ Test 4: Combination (Tests 1+2+3) (30 min)
   - Combine successful opts, test performance
6. ⏳ Test 5: Warp Optimizations (1-2 hours)
   - Analyze divergence, implement fixes, test
7. ⏳ Test 6: Vectorized Loads (1 hour)
   - Implement, test

**Total Time**: 3-4 hours for quick wins  
**Expected Outcome**: 300-350k H/s (+9-27%)  
**Then**: Evaluate if advanced opts (shared mem, persistent kernel) worth pursuing

---

## Advantage of This Approach

1. **Fast iteration**: 15-30 min per test
2. **Low risk**: Small changes, easy to verify
3. **Cumulative**: Can combine successful opts
4. **Learning**: Understand what works/doesn't quickly
5. **Momentum**: Build confidence with quick wins

---

## Ready to Start?

I'll begin with Test 1 (Constant Memory) - should take ~30 minutes including:
- Implementation (5 min)
- Compilation (5 min)
- Hash correctness test (5 min)
- Performance test (10 min)
- Documentation (5 min)

Let's go! 🚀


**Goal**: Test all feasible Phase 6 optimizations systematically  
**Priority**: Hash correctness > Performance  
**Method**: Incremental kernel modifications with validation

---

## Optimizations to Test (Prioritized by Ease)

### Tier 1: Kernel Parameter Tuning (30 min each)

These can be tested by modifying launch parameters or adding simple directives:

1. **`maxrregcount` Directive** (Easy)
   - Add `__launch_bounds__` with `maxrregcount`
   - Force specific register usage
   - Test: 128, 192, 224 registers
   
2. **Occupancy Tuning** (Easy)
   - Test different threads/block with occupancy hints
   - Already done partially, but can test with hints
   
3. **Unroll Pragmas** (Easy)
   - Add `#pragma unroll` to instruction loop
   - May reduce loop overhead

### Tier 2: Memory Access Optimizations (1-2 hours each)

4. **Constant Memory for ROM Digest** (Medium)
   - Move `rom_digest` to `__constant__` memory
   - 32 bytes, frequently accessed
   - Should be easy win

5. **Vectorized Loads** (Medium)
   - Use `float4` or `uint4` for ROM/salt loading
   - May improve memory bandwidth

6. **Aligned Memory Access** (Easy)
   - Ensure all memory accesses are aligned
   - Add `__align__` directives

### Tier 3: Instruction-Level Optimizations (2-3 hours each)

7. **Warp-Level Optimizations** (Medium-Hard)
   - Minimize branch divergence
   - Use `__any()`, `__all()` for conditionals
   - Reorder to align branches

8. **Inline Functions** (Easy)
   - Add `__forceinline__` to hot functions
   - Reduce function call overhead

### Tier 4: Advanced (If needed, 4+ hours each)

9. **Shared Memory** (Hard - already attempted)
10. **Texture Memory** (Hard)
11. **Persistent Kernel** (Very Hard)

---

## Testing Protocol

### For Each Optimization:

**Step 1: Baseline Capture** (if not done)
```bash
cargo run --release --features cuda --example minimal_hash_test
# Save output as baseline
```

**Step 2: Implement Optimization**
- Modify kernel with clear comments
- Keep changes minimal and focused

**Step 3: Hash Correctness Test** ⚠️ CRITICAL
```bash
cargo run --release --features cuda --example minimal_hash_test
# Compare with baseline
# If different: INVALID - revert or fix
# If same: VALID - proceed to Step 4
```

**Step 4: Performance Test**
```bash
cargo run --release --features cuda --example multi_gpu_final
# Run 3x for statistical confidence
# Record: min, max, avg hashrate
```

**Step 5: Decision**
- If +5% or more: KEEP
- If +1% to +5%: KEEP (marginal)
- If -2% to +1%: NEUTRAL (keep if no downsides)
- If -2% or worse: DISCARD

**Step 6: Document**
- Update results file with findings
- Mark as kept/discarded
- Note any side effects

---

## Quick Wins to Start With

Let me implement the EASIEST optimizations first to build momentum:

### Test 1: Constant Memory for ROM Digest (15 min)

**Change**:
```cuda
__constant__ uint8_t rom_digest_const[32];

// In kernel:
blake2b_update(&final_state, rom_digest_const, 32);
```

**Expected**: +2-5% (ROM digest accessed frequently)
**Risk**: Very Low
**Hash Correctness**: Should not change

### Test 2: Inline Function Hints (10 min)

**Change**:
```cuda
__device__ __forceinline__ uint64_t mem_access64(...)
__device__ __forceinline__ void execute_one_instruction(...)
```

**Expected**: +1-3% (reduce call overhead)
**Risk**: Very Low
**Hash Correctness**: Should not change

### Test 3: Loop Unrolling (10 min)

**Change**:
```cuda
#pragma unroll 8
for (uint32_t instr_idx = 0; instr_idx < nb_instrs; ++instr_idx) {
```

**Expected**: +2-8% (reduce loop overhead)
**Risk**: Low (may increase register pressure)
**Hash Correctness**: Should not change

---

## Implementation Order (Next 3-4 Hours)

1. ✅ Baseline established (275k H/s)
2. ⏳ Test 1: Constant Memory (30 min)
   - Implement, verify correctness, test performance
3. ⏳ Test 2: Inline Hints (20 min)
   - Implement, verify correctness, test performance
4. ⏳ Test 3: Loop Unrolling (30 min)
   - Implement, verify correctness, test performance
5. ⏳ Test 4: Combination (Tests 1+2+3) (30 min)
   - Combine successful opts, test performance
6. ⏳ Test 5: Warp Optimizations (1-2 hours)
   - Analyze divergence, implement fixes, test
7. ⏳ Test 6: Vectorized Loads (1 hour)
   - Implement, test

**Total Time**: 3-4 hours for quick wins  
**Expected Outcome**: 300-350k H/s (+9-27%)  
**Then**: Evaluate if advanced opts (shared mem, persistent kernel) worth pursuing

---

## Advantage of This Approach

1. **Fast iteration**: 15-30 min per test
2. **Low risk**: Small changes, easy to verify
3. **Cumulative**: Can combine successful opts
4. **Learning**: Understand what works/doesn't quickly
5. **Momentum**: Build confidence with quick wins

---

## Ready to Start?

I'll begin with Test 1 (Constant Memory) - should take ~30 minutes including:
- Implementation (5 min)
- Compilation (5 min)
- Hash correctness test (5 min)
- Performance test (10 min)
- Documentation (5 min)

Let's go! 🚀


**Goal**: Test all feasible Phase 6 optimizations systematically  
**Priority**: Hash correctness > Performance  
**Method**: Incremental kernel modifications with validation

---

## Optimizations to Test (Prioritized by Ease)

### Tier 1: Kernel Parameter Tuning (30 min each)

These can be tested by modifying launch parameters or adding simple directives:

1. **`maxrregcount` Directive** (Easy)
   - Add `__launch_bounds__` with `maxrregcount`
   - Force specific register usage
   - Test: 128, 192, 224 registers
   
2. **Occupancy Tuning** (Easy)
   - Test different threads/block with occupancy hints
   - Already done partially, but can test with hints
   
3. **Unroll Pragmas** (Easy)
   - Add `#pragma unroll` to instruction loop
   - May reduce loop overhead

### Tier 2: Memory Access Optimizations (1-2 hours each)

4. **Constant Memory for ROM Digest** (Medium)
   - Move `rom_digest` to `__constant__` memory
   - 32 bytes, frequently accessed
   - Should be easy win

5. **Vectorized Loads** (Medium)
   - Use `float4` or `uint4` for ROM/salt loading
   - May improve memory bandwidth

6. **Aligned Memory Access** (Easy)
   - Ensure all memory accesses are aligned
   - Add `__align__` directives

### Tier 3: Instruction-Level Optimizations (2-3 hours each)

7. **Warp-Level Optimizations** (Medium-Hard)
   - Minimize branch divergence
   - Use `__any()`, `__all()` for conditionals
   - Reorder to align branches

8. **Inline Functions** (Easy)
   - Add `__forceinline__` to hot functions
   - Reduce function call overhead

### Tier 4: Advanced (If needed, 4+ hours each)

9. **Shared Memory** (Hard - already attempted)
10. **Texture Memory** (Hard)
11. **Persistent Kernel** (Very Hard)

---

## Testing Protocol

### For Each Optimization:

**Step 1: Baseline Capture** (if not done)
```bash
cargo run --release --features cuda --example minimal_hash_test
# Save output as baseline
```

**Step 2: Implement Optimization**
- Modify kernel with clear comments
- Keep changes minimal and focused

**Step 3: Hash Correctness Test** ⚠️ CRITICAL
```bash
cargo run --release --features cuda --example minimal_hash_test
# Compare with baseline
# If different: INVALID - revert or fix
# If same: VALID - proceed to Step 4
```

**Step 4: Performance Test**
```bash
cargo run --release --features cuda --example multi_gpu_final
# Run 3x for statistical confidence
# Record: min, max, avg hashrate
```

**Step 5: Decision**
- If +5% or more: KEEP
- If +1% to +5%: KEEP (marginal)
- If -2% to +1%: NEUTRAL (keep if no downsides)
- If -2% or worse: DISCARD

**Step 6: Document**
- Update results file with findings
- Mark as kept/discarded
- Note any side effects

---

## Quick Wins to Start With

Let me implement the EASIEST optimizations first to build momentum:

### Test 1: Constant Memory for ROM Digest (15 min)

**Change**:
```cuda
__constant__ uint8_t rom_digest_const[32];

// In kernel:
blake2b_update(&final_state, rom_digest_const, 32);
```

**Expected**: +2-5% (ROM digest accessed frequently)
**Risk**: Very Low
**Hash Correctness**: Should not change

### Test 2: Inline Function Hints (10 min)

**Change**:
```cuda
__device__ __forceinline__ uint64_t mem_access64(...)
__device__ __forceinline__ void execute_one_instruction(...)
```

**Expected**: +1-3% (reduce call overhead)
**Risk**: Very Low
**Hash Correctness**: Should not change

### Test 3: Loop Unrolling (10 min)

**Change**:
```cuda
#pragma unroll 8
for (uint32_t instr_idx = 0; instr_idx < nb_instrs; ++instr_idx) {
```

**Expected**: +2-8% (reduce loop overhead)
**Risk**: Low (may increase register pressure)
**Hash Correctness**: Should not change

---

## Implementation Order (Next 3-4 Hours)

1. ✅ Baseline established (275k H/s)
2. ⏳ Test 1: Constant Memory (30 min)
   - Implement, verify correctness, test performance
3. ⏳ Test 2: Inline Hints (20 min)
   - Implement, verify correctness, test performance
4. ⏳ Test 3: Loop Unrolling (30 min)
   - Implement, verify correctness, test performance
5. ⏳ Test 4: Combination (Tests 1+2+3) (30 min)
   - Combine successful opts, test performance
6. ⏳ Test 5: Warp Optimizations (1-2 hours)
   - Analyze divergence, implement fixes, test
7. ⏳ Test 6: Vectorized Loads (1 hour)
   - Implement, test

**Total Time**: 3-4 hours for quick wins  
**Expected Outcome**: 300-350k H/s (+9-27%)  
**Then**: Evaluate if advanced opts (shared mem, persistent kernel) worth pursuing

---

## Advantage of This Approach

1. **Fast iteration**: 15-30 min per test
2. **Low risk**: Small changes, easy to verify
3. **Cumulative**: Can combine successful opts
4. **Learning**: Understand what works/doesn't quickly
5. **Momentum**: Build confidence with quick wins

---

## Ready to Start?

I'll begin with Test 1 (Constant Memory) - should take ~30 minutes including:
- Implementation (5 min)
- Compilation (5 min)
- Hash correctness test (5 min)
- Performance test (10 min)
- Documentation (5 min)

Let's go! 🚀




