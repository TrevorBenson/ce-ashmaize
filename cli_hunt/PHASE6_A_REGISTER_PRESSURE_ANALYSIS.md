# Phase 6-A: Register Pressure Analysis

**Date**: Current  
**Status**: ✅ ROOT CAUSE IDENTIFIED

---

## Critical Finding

### Current Kernel Resource Usage (sm_89)

```
ptxas info: Used 255 registers (out of 256 max per thread!)
           768 bytes spill stores, 384 bytes spill loads  
           11,328 bytes stack frame
           704 bytes cmem[3], 436 bytes cmem[0]
```

### Why This Is THE Bottleneck

**Register Limit Math**:
- RTX 4090: 65,536 registers per SM
- Current: 255 regs/thread

**At 256 threads/block**:
- 255 × 256 = 65,280 registers ✅ (barely fits!)

**At 384 threads/block**:
- 255 × 384 = 97,920 registers ❌ (exceeds 65,536 limit!)
- **This is why block sizes >256 completely fail!**

**At 512 threads/block** (if we could reduce to 128 regs):
- 128 × 512 = 65,536 registers ✅ (perfect fit!)
- **Potential 2x occupancy improvement!**

---

## Performance Impact

### Register Spilling

**Current**: 768 bytes spill stores + 384 bytes spill loads per thread

**Impact**:
- Every spilled register = slow global memory access
- Can cost 100-400 cycles vs 1 cycle for register access
- **Estimate**: -30% to -50% performance loss from spills alone!

### Low Occupancy

**Current**: Max 256 threads/block due to register pressure
- RTX 4090 can support 1536 threads/SM
- With 256 threads/block: Only 6 blocks/SM = 1536 threads ✅
- But limited by warps and register pressure

**If reduced to 128 regs/thread**:
- Could run 512 threads/block
- 3 blocks/SM = 1536 threads
- Better latency hiding, more parallel execution

---

## Path to 2-3x Improvement

### Target: Reduce from 255 → 128 registers

**Expected gains**:
1. **Eliminate register spills**: +30-50% (120-150k H/s → 156-220k per GPU)
2. **Increase block size to 512**: +20-30% occupancy (→ 187-286k per GPU)
3. **Combined**: **~2-2.5x improvement = 520-715k H/s aggregate!**

This could get us close to or OVER 1 MH/s!

---

## Register Usage Analysis

### Major Register Consumers (Estimated)

**VM State** (~40-50 registers):
- 8x 64-bit registers (r[0-7]) = 16 regs
- Blake2b state (8x 64-bit h[], 16x 64-bit m[], counters) = ~26 regs
- IP, loop_counter, memory_counter = 3 regs
- prog_digest[64 bytes] = 16 regs
- prog_seed[64 bytes] = 16 regs
- **Subtotal: ~77 regs**

**Instruction Decode & Execution** (~30-40 registers):
- Instruction struct
- src1, src2, result
- Temp variables for ops
- Blake2b compression temps

**Memory Access** (~20-30 registers):
- ROM pointers
- Salt pointers  
- Output pointers
- Address calculations
- Temp buffers

**Blake2b Compression** (~60-80 registers):
- The biggest consumer! 
- v[16] working variables = 32 regs
- Message schedule temps
- Mixing function temps

**Loop Variables & Misc**: ~20-40 regs

**Total**: 255 registers (verified)

---

## Optimization Strategy

### Priority 1: Reduce Blake2b Register Usage (Target: -40 regs)

**Current**: Blake2b compression uses ~60-80 registers

**Options**:
1. **Reuse VM registers during Blake2b** (-16 regs)
   - VM registers r[0-7] not needed during Blake2b compression
   - Reuse them for v[0-7] in Blake2b
   
2. **Use shared memory for Blake2b state** (-30 regs)
   - Store v[16] in shared memory
   - Trade registers for shared memory bandwidth
   - May be slower but increases occupancy
   
3. **Optimize message schedule** (-10 regs)
   - Recompute sigma values instead of storing
   - Use constant memory

### Priority 2: Optimize VM State (Target: -20 regs)

**Options**:
1. **Move prog_digest to shared/global memory** (-16 regs)
   - Only needed at start/end
   - Don't keep in registers
   
2. **Move prog_seed to shared/global memory** (-16 regs)
   - Only needed at initialization
   
3. **Pack small variables** (-4 regs)
   - IP, loop_counter, memory_counter as single 64-bit
   - Use bitfields

### Priority 3: Eliminate Register Spills (Target: -50 regs)

**Use maxrregcount compiler directive**:
```cuda
__launch_bounds__(256, 6)  // 256 threads, 6 blocks/SM
```

**Manual optimization**:
- Identify spilled variables
- Move to shared memory or recompute
- Reduce temp variable lifetime

---

## Implementation Plan

### Phase A1: Quick Wins (2-3 hours)

1. **Add launch_bounds** (30 min)
   - Force specific occupancy
   - See if compiler can optimize better
   
2. **Move large buffers out of registers** (1-2 hours)
   - prog_digest → shared memory
   - prog_seed → shared memory
   - Test: Expect -32 regs, minor perf change

**Expected**: 220-230 registers, some spills remain

### Phase A2: Blake2b Optimization (3-4 hours)

1. **Reuse VM registers for Blake2b v[] array** (2 hours)
   - Careful refactoring
   - May need multiple passes
   
2. **Optimize message schedule** (1-2 hours)
   - Use constant memory for sigma
   - Recompute instead of store

**Expected**: 180-200 registers, reduced spills

### Phase A3: Aggressive Optimization (4-6 hours)

1. **Blake2b state to shared memory** (2-3 hours)
   - More complex refactoring
   - Test performance trade-offs
   
2. **Manual register allocation review** (2-3 hours)
   - Review PTX output
   - Identify remaining spills
   - Hand-optimize critical paths

**Expected**: 120-150 registers, no spills, 512 threads/block possible

---

## Testing Methodology

**After each change**:
1. Recompile with `-cubin --ptxas-options=-v`
2. Check register count
3. Run phase6_block_size_test.rs
4. Document performance change
5. Keep if positive, revert if negative

**Success Criteria**:
- **Good**: <200 registers (allowing 384 threads/block)
- **Great**: <128 registers (allowing 512 threads/block)
- **Excellent**: No register spills

---

## Estimated Performance Gains

| Optimization | Registers | Block Size | Expected H/s | Improvement |
|--------------|-----------|------------|--------------|-------------|
| Current | 255 | 256 | 273k | Baseline |
| Phase A1 | 220 | 256 | 290-310k | +6-14% |
| Phase A2 | 180 | 384 | 380-450k | +39-65% |
| Phase A3 | 128 | 512 | 520-650k | +90-138% |
| A3 + Other opts | 128 | 512 | **700k-1MH+** | **2.6-3.7x+** |

---

## Next Steps

1. ✅ Analysis complete
2. ⏳ Implement Phase A1 (Quick Wins)
3. ⏳ Test and document results
4. ⏳ Proceed to A2 if A1 successful
5. ⏳ Combine with other optimizations (shared memory, texture)

---

**Status**: Ready to implement  
**Confidence**: HIGH - this is the bottleneck  
**Potential**: 2-3x improvement, possibly reaching 1 MH/s!


**Date**: Current  
**Status**: ✅ ROOT CAUSE IDENTIFIED

---

## Critical Finding

### Current Kernel Resource Usage (sm_89)

```
ptxas info: Used 255 registers (out of 256 max per thread!)
           768 bytes spill stores, 384 bytes spill loads  
           11,328 bytes stack frame
           704 bytes cmem[3], 436 bytes cmem[0]
```

### Why This Is THE Bottleneck

**Register Limit Math**:
- RTX 4090: 65,536 registers per SM
- Current: 255 regs/thread

**At 256 threads/block**:
- 255 × 256 = 65,280 registers ✅ (barely fits!)

**At 384 threads/block**:
- 255 × 384 = 97,920 registers ❌ (exceeds 65,536 limit!)
- **This is why block sizes >256 completely fail!**

**At 512 threads/block** (if we could reduce to 128 regs):
- 128 × 512 = 65,536 registers ✅ (perfect fit!)
- **Potential 2x occupancy improvement!**

---

## Performance Impact

### Register Spilling

**Current**: 768 bytes spill stores + 384 bytes spill loads per thread

**Impact**:
- Every spilled register = slow global memory access
- Can cost 100-400 cycles vs 1 cycle for register access
- **Estimate**: -30% to -50% performance loss from spills alone!

### Low Occupancy

**Current**: Max 256 threads/block due to register pressure
- RTX 4090 can support 1536 threads/SM
- With 256 threads/block: Only 6 blocks/SM = 1536 threads ✅
- But limited by warps and register pressure

**If reduced to 128 regs/thread**:
- Could run 512 threads/block
- 3 blocks/SM = 1536 threads
- Better latency hiding, more parallel execution

---

## Path to 2-3x Improvement

### Target: Reduce from 255 → 128 registers

**Expected gains**:
1. **Eliminate register spills**: +30-50% (120-150k H/s → 156-220k per GPU)
2. **Increase block size to 512**: +20-30% occupancy (→ 187-286k per GPU)
3. **Combined**: **~2-2.5x improvement = 520-715k H/s aggregate!**

This could get us close to or OVER 1 MH/s!

---

## Register Usage Analysis

### Major Register Consumers (Estimated)

**VM State** (~40-50 registers):
- 8x 64-bit registers (r[0-7]) = 16 regs
- Blake2b state (8x 64-bit h[], 16x 64-bit m[], counters) = ~26 regs
- IP, loop_counter, memory_counter = 3 regs
- prog_digest[64 bytes] = 16 regs
- prog_seed[64 bytes] = 16 regs
- **Subtotal: ~77 regs**

**Instruction Decode & Execution** (~30-40 registers):
- Instruction struct
- src1, src2, result
- Temp variables for ops
- Blake2b compression temps

**Memory Access** (~20-30 registers):
- ROM pointers
- Salt pointers  
- Output pointers
- Address calculations
- Temp buffers

**Blake2b Compression** (~60-80 registers):
- The biggest consumer! 
- v[16] working variables = 32 regs
- Message schedule temps
- Mixing function temps

**Loop Variables & Misc**: ~20-40 regs

**Total**: 255 registers (verified)

---

## Optimization Strategy

### Priority 1: Reduce Blake2b Register Usage (Target: -40 regs)

**Current**: Blake2b compression uses ~60-80 registers

**Options**:
1. **Reuse VM registers during Blake2b** (-16 regs)
   - VM registers r[0-7] not needed during Blake2b compression
   - Reuse them for v[0-7] in Blake2b
   
2. **Use shared memory for Blake2b state** (-30 regs)
   - Store v[16] in shared memory
   - Trade registers for shared memory bandwidth
   - May be slower but increases occupancy
   
3. **Optimize message schedule** (-10 regs)
   - Recompute sigma values instead of storing
   - Use constant memory

### Priority 2: Optimize VM State (Target: -20 regs)

**Options**:
1. **Move prog_digest to shared/global memory** (-16 regs)
   - Only needed at start/end
   - Don't keep in registers
   
2. **Move prog_seed to shared/global memory** (-16 regs)
   - Only needed at initialization
   
3. **Pack small variables** (-4 regs)
   - IP, loop_counter, memory_counter as single 64-bit
   - Use bitfields

### Priority 3: Eliminate Register Spills (Target: -50 regs)

**Use maxrregcount compiler directive**:
```cuda
__launch_bounds__(256, 6)  // 256 threads, 6 blocks/SM
```

**Manual optimization**:
- Identify spilled variables
- Move to shared memory or recompute
- Reduce temp variable lifetime

---

## Implementation Plan

### Phase A1: Quick Wins (2-3 hours)

1. **Add launch_bounds** (30 min)
   - Force specific occupancy
   - See if compiler can optimize better
   
2. **Move large buffers out of registers** (1-2 hours)
   - prog_digest → shared memory
   - prog_seed → shared memory
   - Test: Expect -32 regs, minor perf change

**Expected**: 220-230 registers, some spills remain

### Phase A2: Blake2b Optimization (3-4 hours)

1. **Reuse VM registers for Blake2b v[] array** (2 hours)
   - Careful refactoring
   - May need multiple passes
   
2. **Optimize message schedule** (1-2 hours)
   - Use constant memory for sigma
   - Recompute instead of store

**Expected**: 180-200 registers, reduced spills

### Phase A3: Aggressive Optimization (4-6 hours)

1. **Blake2b state to shared memory** (2-3 hours)
   - More complex refactoring
   - Test performance trade-offs
   
2. **Manual register allocation review** (2-3 hours)
   - Review PTX output
   - Identify remaining spills
   - Hand-optimize critical paths

**Expected**: 120-150 registers, no spills, 512 threads/block possible

---

## Testing Methodology

**After each change**:
1. Recompile with `-cubin --ptxas-options=-v`
2. Check register count
3. Run phase6_block_size_test.rs
4. Document performance change
5. Keep if positive, revert if negative

**Success Criteria**:
- **Good**: <200 registers (allowing 384 threads/block)
- **Great**: <128 registers (allowing 512 threads/block)
- **Excellent**: No register spills

---

## Estimated Performance Gains

| Optimization | Registers | Block Size | Expected H/s | Improvement |
|--------------|-----------|------------|--------------|-------------|
| Current | 255 | 256 | 273k | Baseline |
| Phase A1 | 220 | 256 | 290-310k | +6-14% |
| Phase A2 | 180 | 384 | 380-450k | +39-65% |
| Phase A3 | 128 | 512 | 520-650k | +90-138% |
| A3 + Other opts | 128 | 512 | **700k-1MH+** | **2.6-3.7x+** |

---

## Next Steps

1. ✅ Analysis complete
2. ⏳ Implement Phase A1 (Quick Wins)
3. ⏳ Test and document results
4. ⏳ Proceed to A2 if A1 successful
5. ⏳ Combine with other optimizations (shared memory, texture)

---

**Status**: Ready to implement  
**Confidence**: HIGH - this is the bottleneck  
**Potential**: 2-3x improvement, possibly reaching 1 MH/s!


**Date**: Current  
**Status**: ✅ ROOT CAUSE IDENTIFIED

---

## Critical Finding

### Current Kernel Resource Usage (sm_89)

```
ptxas info: Used 255 registers (out of 256 max per thread!)
           768 bytes spill stores, 384 bytes spill loads  
           11,328 bytes stack frame
           704 bytes cmem[3], 436 bytes cmem[0]
```

### Why This Is THE Bottleneck

**Register Limit Math**:
- RTX 4090: 65,536 registers per SM
- Current: 255 regs/thread

**At 256 threads/block**:
- 255 × 256 = 65,280 registers ✅ (barely fits!)

**At 384 threads/block**:
- 255 × 384 = 97,920 registers ❌ (exceeds 65,536 limit!)
- **This is why block sizes >256 completely fail!**

**At 512 threads/block** (if we could reduce to 128 regs):
- 128 × 512 = 65,536 registers ✅ (perfect fit!)
- **Potential 2x occupancy improvement!**

---

## Performance Impact

### Register Spilling

**Current**: 768 bytes spill stores + 384 bytes spill loads per thread

**Impact**:
- Every spilled register = slow global memory access
- Can cost 100-400 cycles vs 1 cycle for register access
- **Estimate**: -30% to -50% performance loss from spills alone!

### Low Occupancy

**Current**: Max 256 threads/block due to register pressure
- RTX 4090 can support 1536 threads/SM
- With 256 threads/block: Only 6 blocks/SM = 1536 threads ✅
- But limited by warps and register pressure

**If reduced to 128 regs/thread**:
- Could run 512 threads/block
- 3 blocks/SM = 1536 threads
- Better latency hiding, more parallel execution

---

## Path to 2-3x Improvement

### Target: Reduce from 255 → 128 registers

**Expected gains**:
1. **Eliminate register spills**: +30-50% (120-150k H/s → 156-220k per GPU)
2. **Increase block size to 512**: +20-30% occupancy (→ 187-286k per GPU)
3. **Combined**: **~2-2.5x improvement = 520-715k H/s aggregate!**

This could get us close to or OVER 1 MH/s!

---

## Register Usage Analysis

### Major Register Consumers (Estimated)

**VM State** (~40-50 registers):
- 8x 64-bit registers (r[0-7]) = 16 regs
- Blake2b state (8x 64-bit h[], 16x 64-bit m[], counters) = ~26 regs
- IP, loop_counter, memory_counter = 3 regs
- prog_digest[64 bytes] = 16 regs
- prog_seed[64 bytes] = 16 regs
- **Subtotal: ~77 regs**

**Instruction Decode & Execution** (~30-40 registers):
- Instruction struct
- src1, src2, result
- Temp variables for ops
- Blake2b compression temps

**Memory Access** (~20-30 registers):
- ROM pointers
- Salt pointers  
- Output pointers
- Address calculations
- Temp buffers

**Blake2b Compression** (~60-80 registers):
- The biggest consumer! 
- v[16] working variables = 32 regs
- Message schedule temps
- Mixing function temps

**Loop Variables & Misc**: ~20-40 regs

**Total**: 255 registers (verified)

---

## Optimization Strategy

### Priority 1: Reduce Blake2b Register Usage (Target: -40 regs)

**Current**: Blake2b compression uses ~60-80 registers

**Options**:
1. **Reuse VM registers during Blake2b** (-16 regs)
   - VM registers r[0-7] not needed during Blake2b compression
   - Reuse them for v[0-7] in Blake2b
   
2. **Use shared memory for Blake2b state** (-30 regs)
   - Store v[16] in shared memory
   - Trade registers for shared memory bandwidth
   - May be slower but increases occupancy
   
3. **Optimize message schedule** (-10 regs)
   - Recompute sigma values instead of storing
   - Use constant memory

### Priority 2: Optimize VM State (Target: -20 regs)

**Options**:
1. **Move prog_digest to shared/global memory** (-16 regs)
   - Only needed at start/end
   - Don't keep in registers
   
2. **Move prog_seed to shared/global memory** (-16 regs)
   - Only needed at initialization
   
3. **Pack small variables** (-4 regs)
   - IP, loop_counter, memory_counter as single 64-bit
   - Use bitfields

### Priority 3: Eliminate Register Spills (Target: -50 regs)

**Use maxrregcount compiler directive**:
```cuda
__launch_bounds__(256, 6)  // 256 threads, 6 blocks/SM
```

**Manual optimization**:
- Identify spilled variables
- Move to shared memory or recompute
- Reduce temp variable lifetime

---

## Implementation Plan

### Phase A1: Quick Wins (2-3 hours)

1. **Add launch_bounds** (30 min)
   - Force specific occupancy
   - See if compiler can optimize better
   
2. **Move large buffers out of registers** (1-2 hours)
   - prog_digest → shared memory
   - prog_seed → shared memory
   - Test: Expect -32 regs, minor perf change

**Expected**: 220-230 registers, some spills remain

### Phase A2: Blake2b Optimization (3-4 hours)

1. **Reuse VM registers for Blake2b v[] array** (2 hours)
   - Careful refactoring
   - May need multiple passes
   
2. **Optimize message schedule** (1-2 hours)
   - Use constant memory for sigma
   - Recompute instead of store

**Expected**: 180-200 registers, reduced spills

### Phase A3: Aggressive Optimization (4-6 hours)

1. **Blake2b state to shared memory** (2-3 hours)
   - More complex refactoring
   - Test performance trade-offs
   
2. **Manual register allocation review** (2-3 hours)
   - Review PTX output
   - Identify remaining spills
   - Hand-optimize critical paths

**Expected**: 120-150 registers, no spills, 512 threads/block possible

---

## Testing Methodology

**After each change**:
1. Recompile with `-cubin --ptxas-options=-v`
2. Check register count
3. Run phase6_block_size_test.rs
4. Document performance change
5. Keep if positive, revert if negative

**Success Criteria**:
- **Good**: <200 registers (allowing 384 threads/block)
- **Great**: <128 registers (allowing 512 threads/block)
- **Excellent**: No register spills

---

## Estimated Performance Gains

| Optimization | Registers | Block Size | Expected H/s | Improvement |
|--------------|-----------|------------|--------------|-------------|
| Current | 255 | 256 | 273k | Baseline |
| Phase A1 | 220 | 256 | 290-310k | +6-14% |
| Phase A2 | 180 | 384 | 380-450k | +39-65% |
| Phase A3 | 128 | 512 | 520-650k | +90-138% |
| A3 + Other opts | 128 | 512 | **700k-1MH+** | **2.6-3.7x+** |

---

## Next Steps

1. ✅ Analysis complete
2. ⏳ Implement Phase A1 (Quick Wins)
3. ⏳ Test and document results
4. ⏳ Proceed to A2 if A1 successful
5. ⏳ Combine with other optimizations (shared memory, texture)

---

**Status**: Ready to implement  
**Confidence**: HIGH - this is the bottleneck  
**Potential**: 2-3x improvement, possibly reaching 1 MH/s!




