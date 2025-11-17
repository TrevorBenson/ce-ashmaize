# Phase 6: Real Advanced Optimizations - Implementation Plan

**Goal**: Test if 1-2 MH/s (1,000,000 - 2,000,000 H/s) is achievable with 4x RTX 4090  
**Current**: ~294k H/s  
**Target**: 3.4x - 6.8x improvement  
**Time Investment**: Worth it to find maximum performance

---

## Testable Optimizations (Priority Order)

### Tier 1: High ROI, Medium Complexity (Start Here)

#### 1. Shared Memory Caching ⭐⭐⭐
**Expected**: +15-30% (341-382k H/s)  
**Complexity**: Medium  
**Time**: 2-3 hours  

**Implementation**:
- Cache frequently accessed ROM chunks in `__shared__` memory (48KB per SM)
- 8KB cache should cover hot ROM regions
- Cooperative loading by all threads in block

**Files to modify**:
- Create: `cuda/ashmaize_shared_memory.cu`
- Update: `build.rs` (compile new kernel)
- Update: `src/gpu.rs` (add shared_mem variant)
- Create: `examples/test_shared_memory.rs`

#### 2. Increase Block Size / Occupancy ⭐⭐⭐
**Expected**: +10-20% (323-353k H/s)  
**Complexity**: Low  
**Time**: 30 minutes  

**Implementation**:
- Current: Likely 256 threads/block
- Test: 512, 768, 1024 threads/block
- Maximize occupancy on RTX 4090

**Files to modify**:
- Update: `src/gpu.rs` (adjust launch parameters)
- Create: `examples/test_block_sizes.rs`

#### 3. Reduce Register Pressure ⭐⭐
**Expected**: +5-15% (309-338k H/s)  
**Complexity**: Medium  
**Time**: 1-2 hours  

**Implementation**:
- Analyze register usage (`nvcc --ptxas-options=-v`)
- Reduce local variables
- Reuse registers
- May allow more threads/SM

**Files to modify**:
- Update: `cuda/ashmaize.cu` (optimize register usage)

---

### Tier 2: Medium-High ROI, Higher Complexity

#### 4. Texture Memory for ROM ⭐⭐
**Expected**: +10-20% (323-353k H/s)  
**Complexity**: Medium-High  
**Time**: 2-3 hours  

**Implementation**:
- Bind ROM to texture cache
- Automatic caching and prefetching by GPU
- Good for random access patterns

**Files to modify**:
- Create: `cuda/ashmaize_texture.cu`
- Update: `src/gpu.rs` (texture binding)

#### 5. Warp-Level Optimizations ⭐⭐
**Expected**: +5-10% (309-323k H/s)  
**Complexity**: High  
**Time**: 2-4 hours  

**Implementation**:
- Minimize branch divergence
- Use warp shuffle for VM state sharing
- Align branches on warp boundaries

**Files to modify**:
- Update: `cuda/ashmaize_vm.cuh` (reduce divergence)

---

### Tier 3: Potential Game-Changers (If Tier 1-2 Don't Reach Goal)

#### 6. Persistent Threads (Grid-Stride Loop) ⭐⭐⭐⭐
**Expected**: +20-40% (353-412k H/s) - Eliminates kernel launch overhead  
**Complexity**: High  
**Time**: 3-5 hours  

**Implementation**:
- Kernel stays resident, processes multiple batches
- Grid-stride loop pattern
- Eliminates kernel launch overhead (~10-20µs per launch)

**Files to modify**:
- Create: `cuda/ashmaize_persistent.cu`
- Update: `src/gpu.rs` (persistent kernel management)

#### 7. Multi-Stream Execution ⭐⭐
**Expected**: +10-20% (323-353k H/s) - Overlap compute and transfer  
**Complexity**: Medium  
**Time**: 2-3 hours  

**Implementation**:
- Create multiple CUDA streams
- Overlap H2D, compute, D2H
- Pipeline batches

**Approach**: May need to use raw CUDA API or extend cudarc

---

### Tier 4: Combined Optimizations (Final Push)

#### 8. All Successful Optimizations Combined
**Expected**: **Multiplicative gains** - Could reach 1-2 MH/s!  
**Example**: Shared Mem (+25%) × Block Size (+15%) × Texture (+15%) × Persistent (+30%) = **2.15x = 632k H/s**  
**Better Example**: If each gives upper bound: 1.3 × 1.2 × 1.2 × 1.4 = **2.62x = 770k H/s**  
**Optimistic**: With synergies: **3-4x = 882k - 1.18 MH/s**

---

## Implementation Strategy

### Phase A: Quick Wins (4-6 hours)
1. ✅ Test Block Size variations (30 min)
2. ✅ Implement Shared Memory (2-3 hours)
3. ✅ Test combination (30 min)

**Target after Phase A**: 350-400k H/s (+19-36%)

### Phase B: Medium Optimizations (4-6 hours)
4. ✅ Reduce Register Pressure (1-2 hours)
5. ✅ Implement Texture Memory (2-3 hours)
6. ✅ Test combinations (1 hour)

**Target after Phase B**: 400-500k H/s (+36-70%)

### Phase C: Advanced (if needed, 6-10 hours)
7. ✅ Implement Persistent Threads (3-5 hours)
8. ✅ Implement Multi-Stream (2-3 hours)
9. ✅ Test all combinations (2 hours)

**Target after Phase C**: 600k - 1.2 MH/s (+2-4x)

---

## Testing Methodology

### For Each Optimization:
1. Implement kernel variant
2. Run 30s test (quick feedback)
3. If promising (+5% or more), run 60s verification
4. Document results
5. Keep if positive, revert if negative

### Combination Testing:
- Test top 2-3 individual optimizations combined
- Test all positive optimizations together
- Final 60s verification of best combination

### Success Criteria:
- **Good**: 400k+ H/s (+36%)
- **Great**: 600k+ H/s (+2x)
- **Excellent**: 1 MH/s+ (+3.4x)
- **Outstanding**: 1.5-2 MH/s (+5-7x)

---

## Risk Management

### Hash Correctness
- **CRITICAL**: Verify CPU = GPU after each kernel modification
- Test with `minimal_hash_test.rs` after each change
- Keep known-good kernel as backup

### Performance Regression
- If optimization is negative, revert immediately
- Document why it didn't work
- Move to next optimization

### Time Boxing
- If an optimization takes >2x estimated time, document and move on
- Can return later if needed

---

## Starting Point: Baseline Measurement

Before starting optimizations, establish baseline:
```bash
# Run current best configuration 3x for statistical baseline
cargo run --release --features cuda --example multi_gpu_final

# Expected: ~294k H/s ± 5%
# This is our comparison point
```

---

## File Structure

```
cuda/
  ashmaize.cu (original, working)
  ashmaize_shared_memory.cu (new)
  ashmaize_texture.cu (new)
  ashmaize_persistent.cu (new)
  ashmaize_optimized.cu (combined best)

examples/
  test_block_sizes.rs
  test_shared_memory.rs
  test_texture_memory.rs
  test_register_pressure.rs
  test_persistent_threads.rs
  test_multi_stream.rs
  test_all_optimizations.rs

src/
  gpu.rs (add new kernel variants)
```

---

## Expected Timeline

**Conservative** (testing each carefully):
- Day 1 (8 hours): Tier 1 optimizations → Target: 400k H/s
- Day 2 (8 hours): Tier 2 + combinations → Target: 500-600k H/s
- Day 3 (8 hours): Tier 3 if needed → Target: 1 MH/s+

**Aggressive** (if optimizations work well):
- Session 1 (4 hours): Quick wins → 350-400k H/s
- Session 2 (4 hours): Medium opts → 500k H/s
- Session 3 (4 hours): Combinations → 700k-1 MH/s+

---

## Decision Point

After each phase, evaluate:
- **If target reached (1 MH/s+)**: Stop, document, integrate
- **If progress good (+20% per phase)**: Continue to next phase
- **If diminishing returns (<5% per opt)**: Stop, document best config

---

## Let's Start!

**First optimization to implement**: Block Size Tuning (30 minutes, low risk)

This will:
1. Test different thread block sizes (256, 512, 768, 1024)
2. Measure impact on performance
3. Give us quick feedback on optimization potential
4. Establish pattern for other optimizations

**Ready to begin implementation?**

---

**Status**: PLAN READY  
**Next**: Implement block size testing  
**Goal**: Find path to 1-2 MH/s  
**Time Investment**: 12-24 hours total (worth it!)


**Goal**: Test if 1-2 MH/s (1,000,000 - 2,000,000 H/s) is achievable with 4x RTX 4090  
**Current**: ~294k H/s  
**Target**: 3.4x - 6.8x improvement  
**Time Investment**: Worth it to find maximum performance

---

## Testable Optimizations (Priority Order)

### Tier 1: High ROI, Medium Complexity (Start Here)

#### 1. Shared Memory Caching ⭐⭐⭐
**Expected**: +15-30% (341-382k H/s)  
**Complexity**: Medium  
**Time**: 2-3 hours  

**Implementation**:
- Cache frequently accessed ROM chunks in `__shared__` memory (48KB per SM)
- 8KB cache should cover hot ROM regions
- Cooperative loading by all threads in block

**Files to modify**:
- Create: `cuda/ashmaize_shared_memory.cu`
- Update: `build.rs` (compile new kernel)
- Update: `src/gpu.rs` (add shared_mem variant)
- Create: `examples/test_shared_memory.rs`

#### 2. Increase Block Size / Occupancy ⭐⭐⭐
**Expected**: +10-20% (323-353k H/s)  
**Complexity**: Low  
**Time**: 30 minutes  

**Implementation**:
- Current: Likely 256 threads/block
- Test: 512, 768, 1024 threads/block
- Maximize occupancy on RTX 4090

**Files to modify**:
- Update: `src/gpu.rs` (adjust launch parameters)
- Create: `examples/test_block_sizes.rs`

#### 3. Reduce Register Pressure ⭐⭐
**Expected**: +5-15% (309-338k H/s)  
**Complexity**: Medium  
**Time**: 1-2 hours  

**Implementation**:
- Analyze register usage (`nvcc --ptxas-options=-v`)
- Reduce local variables
- Reuse registers
- May allow more threads/SM

**Files to modify**:
- Update: `cuda/ashmaize.cu` (optimize register usage)

---

### Tier 2: Medium-High ROI, Higher Complexity

#### 4. Texture Memory for ROM ⭐⭐
**Expected**: +10-20% (323-353k H/s)  
**Complexity**: Medium-High  
**Time**: 2-3 hours  

**Implementation**:
- Bind ROM to texture cache
- Automatic caching and prefetching by GPU
- Good for random access patterns

**Files to modify**:
- Create: `cuda/ashmaize_texture.cu`
- Update: `src/gpu.rs` (texture binding)

#### 5. Warp-Level Optimizations ⭐⭐
**Expected**: +5-10% (309-323k H/s)  
**Complexity**: High  
**Time**: 2-4 hours  

**Implementation**:
- Minimize branch divergence
- Use warp shuffle for VM state sharing
- Align branches on warp boundaries

**Files to modify**:
- Update: `cuda/ashmaize_vm.cuh` (reduce divergence)

---

### Tier 3: Potential Game-Changers (If Tier 1-2 Don't Reach Goal)

#### 6. Persistent Threads (Grid-Stride Loop) ⭐⭐⭐⭐
**Expected**: +20-40% (353-412k H/s) - Eliminates kernel launch overhead  
**Complexity**: High  
**Time**: 3-5 hours  

**Implementation**:
- Kernel stays resident, processes multiple batches
- Grid-stride loop pattern
- Eliminates kernel launch overhead (~10-20µs per launch)

**Files to modify**:
- Create: `cuda/ashmaize_persistent.cu`
- Update: `src/gpu.rs` (persistent kernel management)

#### 7. Multi-Stream Execution ⭐⭐
**Expected**: +10-20% (323-353k H/s) - Overlap compute and transfer  
**Complexity**: Medium  
**Time**: 2-3 hours  

**Implementation**:
- Create multiple CUDA streams
- Overlap H2D, compute, D2H
- Pipeline batches

**Approach**: May need to use raw CUDA API or extend cudarc

---

### Tier 4: Combined Optimizations (Final Push)

#### 8. All Successful Optimizations Combined
**Expected**: **Multiplicative gains** - Could reach 1-2 MH/s!  
**Example**: Shared Mem (+25%) × Block Size (+15%) × Texture (+15%) × Persistent (+30%) = **2.15x = 632k H/s**  
**Better Example**: If each gives upper bound: 1.3 × 1.2 × 1.2 × 1.4 = **2.62x = 770k H/s**  
**Optimistic**: With synergies: **3-4x = 882k - 1.18 MH/s**

---

## Implementation Strategy

### Phase A: Quick Wins (4-6 hours)
1. ✅ Test Block Size variations (30 min)
2. ✅ Implement Shared Memory (2-3 hours)
3. ✅ Test combination (30 min)

**Target after Phase A**: 350-400k H/s (+19-36%)

### Phase B: Medium Optimizations (4-6 hours)
4. ✅ Reduce Register Pressure (1-2 hours)
5. ✅ Implement Texture Memory (2-3 hours)
6. ✅ Test combinations (1 hour)

**Target after Phase B**: 400-500k H/s (+36-70%)

### Phase C: Advanced (if needed, 6-10 hours)
7. ✅ Implement Persistent Threads (3-5 hours)
8. ✅ Implement Multi-Stream (2-3 hours)
9. ✅ Test all combinations (2 hours)

**Target after Phase C**: 600k - 1.2 MH/s (+2-4x)

---

## Testing Methodology

### For Each Optimization:
1. Implement kernel variant
2. Run 30s test (quick feedback)
3. If promising (+5% or more), run 60s verification
4. Document results
5. Keep if positive, revert if negative

### Combination Testing:
- Test top 2-3 individual optimizations combined
- Test all positive optimizations together
- Final 60s verification of best combination

### Success Criteria:
- **Good**: 400k+ H/s (+36%)
- **Great**: 600k+ H/s (+2x)
- **Excellent**: 1 MH/s+ (+3.4x)
- **Outstanding**: 1.5-2 MH/s (+5-7x)

---

## Risk Management

### Hash Correctness
- **CRITICAL**: Verify CPU = GPU after each kernel modification
- Test with `minimal_hash_test.rs` after each change
- Keep known-good kernel as backup

### Performance Regression
- If optimization is negative, revert immediately
- Document why it didn't work
- Move to next optimization

### Time Boxing
- If an optimization takes >2x estimated time, document and move on
- Can return later if needed

---

## Starting Point: Baseline Measurement

Before starting optimizations, establish baseline:
```bash
# Run current best configuration 3x for statistical baseline
cargo run --release --features cuda --example multi_gpu_final

# Expected: ~294k H/s ± 5%
# This is our comparison point
```

---

## File Structure

```
cuda/
  ashmaize.cu (original, working)
  ashmaize_shared_memory.cu (new)
  ashmaize_texture.cu (new)
  ashmaize_persistent.cu (new)
  ashmaize_optimized.cu (combined best)

examples/
  test_block_sizes.rs
  test_shared_memory.rs
  test_texture_memory.rs
  test_register_pressure.rs
  test_persistent_threads.rs
  test_multi_stream.rs
  test_all_optimizations.rs

src/
  gpu.rs (add new kernel variants)
```

---

## Expected Timeline

**Conservative** (testing each carefully):
- Day 1 (8 hours): Tier 1 optimizations → Target: 400k H/s
- Day 2 (8 hours): Tier 2 + combinations → Target: 500-600k H/s
- Day 3 (8 hours): Tier 3 if needed → Target: 1 MH/s+

**Aggressive** (if optimizations work well):
- Session 1 (4 hours): Quick wins → 350-400k H/s
- Session 2 (4 hours): Medium opts → 500k H/s
- Session 3 (4 hours): Combinations → 700k-1 MH/s+

---

## Decision Point

After each phase, evaluate:
- **If target reached (1 MH/s+)**: Stop, document, integrate
- **If progress good (+20% per phase)**: Continue to next phase
- **If diminishing returns (<5% per opt)**: Stop, document best config

---

## Let's Start!

**First optimization to implement**: Block Size Tuning (30 minutes, low risk)

This will:
1. Test different thread block sizes (256, 512, 768, 1024)
2. Measure impact on performance
3. Give us quick feedback on optimization potential
4. Establish pattern for other optimizations

**Ready to begin implementation?**

---

**Status**: PLAN READY  
**Next**: Implement block size testing  
**Goal**: Find path to 1-2 MH/s  
**Time Investment**: 12-24 hours total (worth it!)


**Goal**: Test if 1-2 MH/s (1,000,000 - 2,000,000 H/s) is achievable with 4x RTX 4090  
**Current**: ~294k H/s  
**Target**: 3.4x - 6.8x improvement  
**Time Investment**: Worth it to find maximum performance

---

## Testable Optimizations (Priority Order)

### Tier 1: High ROI, Medium Complexity (Start Here)

#### 1. Shared Memory Caching ⭐⭐⭐
**Expected**: +15-30% (341-382k H/s)  
**Complexity**: Medium  
**Time**: 2-3 hours  

**Implementation**:
- Cache frequently accessed ROM chunks in `__shared__` memory (48KB per SM)
- 8KB cache should cover hot ROM regions
- Cooperative loading by all threads in block

**Files to modify**:
- Create: `cuda/ashmaize_shared_memory.cu`
- Update: `build.rs` (compile new kernel)
- Update: `src/gpu.rs` (add shared_mem variant)
- Create: `examples/test_shared_memory.rs`

#### 2. Increase Block Size / Occupancy ⭐⭐⭐
**Expected**: +10-20% (323-353k H/s)  
**Complexity**: Low  
**Time**: 30 minutes  

**Implementation**:
- Current: Likely 256 threads/block
- Test: 512, 768, 1024 threads/block
- Maximize occupancy on RTX 4090

**Files to modify**:
- Update: `src/gpu.rs` (adjust launch parameters)
- Create: `examples/test_block_sizes.rs`

#### 3. Reduce Register Pressure ⭐⭐
**Expected**: +5-15% (309-338k H/s)  
**Complexity**: Medium  
**Time**: 1-2 hours  

**Implementation**:
- Analyze register usage (`nvcc --ptxas-options=-v`)
- Reduce local variables
- Reuse registers
- May allow more threads/SM

**Files to modify**:
- Update: `cuda/ashmaize.cu` (optimize register usage)

---

### Tier 2: Medium-High ROI, Higher Complexity

#### 4. Texture Memory for ROM ⭐⭐
**Expected**: +10-20% (323-353k H/s)  
**Complexity**: Medium-High  
**Time**: 2-3 hours  

**Implementation**:
- Bind ROM to texture cache
- Automatic caching and prefetching by GPU
- Good for random access patterns

**Files to modify**:
- Create: `cuda/ashmaize_texture.cu`
- Update: `src/gpu.rs` (texture binding)

#### 5. Warp-Level Optimizations ⭐⭐
**Expected**: +5-10% (309-323k H/s)  
**Complexity**: High  
**Time**: 2-4 hours  

**Implementation**:
- Minimize branch divergence
- Use warp shuffle for VM state sharing
- Align branches on warp boundaries

**Files to modify**:
- Update: `cuda/ashmaize_vm.cuh` (reduce divergence)

---

### Tier 3: Potential Game-Changers (If Tier 1-2 Don't Reach Goal)

#### 6. Persistent Threads (Grid-Stride Loop) ⭐⭐⭐⭐
**Expected**: +20-40% (353-412k H/s) - Eliminates kernel launch overhead  
**Complexity**: High  
**Time**: 3-5 hours  

**Implementation**:
- Kernel stays resident, processes multiple batches
- Grid-stride loop pattern
- Eliminates kernel launch overhead (~10-20µs per launch)

**Files to modify**:
- Create: `cuda/ashmaize_persistent.cu`
- Update: `src/gpu.rs` (persistent kernel management)

#### 7. Multi-Stream Execution ⭐⭐
**Expected**: +10-20% (323-353k H/s) - Overlap compute and transfer  
**Complexity**: Medium  
**Time**: 2-3 hours  

**Implementation**:
- Create multiple CUDA streams
- Overlap H2D, compute, D2H
- Pipeline batches

**Approach**: May need to use raw CUDA API or extend cudarc

---

### Tier 4: Combined Optimizations (Final Push)

#### 8. All Successful Optimizations Combined
**Expected**: **Multiplicative gains** - Could reach 1-2 MH/s!  
**Example**: Shared Mem (+25%) × Block Size (+15%) × Texture (+15%) × Persistent (+30%) = **2.15x = 632k H/s**  
**Better Example**: If each gives upper bound: 1.3 × 1.2 × 1.2 × 1.4 = **2.62x = 770k H/s**  
**Optimistic**: With synergies: **3-4x = 882k - 1.18 MH/s**

---

## Implementation Strategy

### Phase A: Quick Wins (4-6 hours)
1. ✅ Test Block Size variations (30 min)
2. ✅ Implement Shared Memory (2-3 hours)
3. ✅ Test combination (30 min)

**Target after Phase A**: 350-400k H/s (+19-36%)

### Phase B: Medium Optimizations (4-6 hours)
4. ✅ Reduce Register Pressure (1-2 hours)
5. ✅ Implement Texture Memory (2-3 hours)
6. ✅ Test combinations (1 hour)

**Target after Phase B**: 400-500k H/s (+36-70%)

### Phase C: Advanced (if needed, 6-10 hours)
7. ✅ Implement Persistent Threads (3-5 hours)
8. ✅ Implement Multi-Stream (2-3 hours)
9. ✅ Test all combinations (2 hours)

**Target after Phase C**: 600k - 1.2 MH/s (+2-4x)

---

## Testing Methodology

### For Each Optimization:
1. Implement kernel variant
2. Run 30s test (quick feedback)
3. If promising (+5% or more), run 60s verification
4. Document results
5. Keep if positive, revert if negative

### Combination Testing:
- Test top 2-3 individual optimizations combined
- Test all positive optimizations together
- Final 60s verification of best combination

### Success Criteria:
- **Good**: 400k+ H/s (+36%)
- **Great**: 600k+ H/s (+2x)
- **Excellent**: 1 MH/s+ (+3.4x)
- **Outstanding**: 1.5-2 MH/s (+5-7x)

---

## Risk Management

### Hash Correctness
- **CRITICAL**: Verify CPU = GPU after each kernel modification
- Test with `minimal_hash_test.rs` after each change
- Keep known-good kernel as backup

### Performance Regression
- If optimization is negative, revert immediately
- Document why it didn't work
- Move to next optimization

### Time Boxing
- If an optimization takes >2x estimated time, document and move on
- Can return later if needed

---

## Starting Point: Baseline Measurement

Before starting optimizations, establish baseline:
```bash
# Run current best configuration 3x for statistical baseline
cargo run --release --features cuda --example multi_gpu_final

# Expected: ~294k H/s ± 5%
# This is our comparison point
```

---

## File Structure

```
cuda/
  ashmaize.cu (original, working)
  ashmaize_shared_memory.cu (new)
  ashmaize_texture.cu (new)
  ashmaize_persistent.cu (new)
  ashmaize_optimized.cu (combined best)

examples/
  test_block_sizes.rs
  test_shared_memory.rs
  test_texture_memory.rs
  test_register_pressure.rs
  test_persistent_threads.rs
  test_multi_stream.rs
  test_all_optimizations.rs

src/
  gpu.rs (add new kernel variants)
```

---

## Expected Timeline

**Conservative** (testing each carefully):
- Day 1 (8 hours): Tier 1 optimizations → Target: 400k H/s
- Day 2 (8 hours): Tier 2 + combinations → Target: 500-600k H/s
- Day 3 (8 hours): Tier 3 if needed → Target: 1 MH/s+

**Aggressive** (if optimizations work well):
- Session 1 (4 hours): Quick wins → 350-400k H/s
- Session 2 (4 hours): Medium opts → 500k H/s
- Session 3 (4 hours): Combinations → 700k-1 MH/s+

---

## Decision Point

After each phase, evaluate:
- **If target reached (1 MH/s+)**: Stop, document, integrate
- **If progress good (+20% per phase)**: Continue to next phase
- **If diminishing returns (<5% per opt)**: Stop, document best config

---

## Let's Start!

**First optimization to implement**: Block Size Tuning (30 minutes, low risk)

This will:
1. Test different thread block sizes (256, 512, 768, 1024)
2. Measure impact on performance
3. Give us quick feedback on optimization potential
4. Establish pattern for other optimizations

**Ready to begin implementation?**

---

**Status**: PLAN READY  
**Next**: Implement block size testing  
**Goal**: Find path to 1-2 MH/s  
**Time Investment**: 12-24 hours total (worth it!)




