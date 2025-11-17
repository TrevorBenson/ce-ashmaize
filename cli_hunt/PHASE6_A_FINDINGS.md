# Phase 6-A: Register Optimization Findings

## Test 1: __launch_bounds__(256, 6) ❌ FAILED

**Change**: Added `__launch_bounds__(256, 6)` to kernel

**Results**:
```
Before: 255 regs,   768 spill stores,   384 spill loads → 273k H/s
After:   40 regs, 11,422 spill stores, 13,336 spill loads → 131k H/s (-52%!)
```

**Analysis**:
- Compiler reduced registers from 255 → 40
- But spilled 14x more data to global memory
- Register spills are EXTREMELY expensive (100-400 cycles vs 1 cycle)
- **Result**: Massive performance loss

**Lesson**: Can't just force low register count - need balanced optimization

---

## New Strategy: Manual Optimization

### Target: 150-180 registers
- Allows 384 threads/block (150×384 = 57,600 < 65,536)
- Minimal spilling
- Balance between occupancy and memory access

### Approach: Identify Largest Register Consumers

From code analysis, estimated register usage:

**Blake2b State** (~50-70 regs):
- h[8] = 16 regs
- m[16] = 32 regs  
- t[2], f[2] = 4 regs
- Compression working variables v[16] = 32 regs during compression
- **Optimization**: Move some to shared memory

**VM State** (~40 regs):
- regs[8] = 16 regs
- prog_digest (64 bytes) = 16 regs ← **Target for removal**
- prog_seed (64 bytes) = 16 regs ← **Only needed at init**
- mem_digest_state (partial) = ~20 regs
- IP, loop_counter, memory_counter = 3 regs

**Instruction Processing** (~30-40 regs):
- Decoded instruction
- src1, src2, result
- Address calculations
- Temporaries

**Others**: ~50-80 regs

---

## Next Steps

### Phase A1b: Move Large Buffers to Shared Memory

**Target**: prog_seed (only needed at init)
- Currently: 16 registers × entire kernel lifetime
- Optimization: Load from shared memory at init only
- **Expected**: -16 registers

**Implementation**:
1. Allocate shared memory for prog_seeds
2. Load collaboratively at kernel start
3. Read from shared memory during init
4. Remove from VMState registers

**Estimated gain**: 255 → 239 registers, similar performance

Let's implement this carefully and test!

---

**Status**: Test 1 reverted, moving to manual optimization  
**Next**: Implement shared memory for prog_seed


## Test 1: __launch_bounds__(256, 6) ❌ FAILED

**Change**: Added `__launch_bounds__(256, 6)` to kernel

**Results**:
```
Before: 255 regs,   768 spill stores,   384 spill loads → 273k H/s
After:   40 regs, 11,422 spill stores, 13,336 spill loads → 131k H/s (-52%!)
```

**Analysis**:
- Compiler reduced registers from 255 → 40
- But spilled 14x more data to global memory
- Register spills are EXTREMELY expensive (100-400 cycles vs 1 cycle)
- **Result**: Massive performance loss

**Lesson**: Can't just force low register count - need balanced optimization

---

## New Strategy: Manual Optimization

### Target: 150-180 registers
- Allows 384 threads/block (150×384 = 57,600 < 65,536)
- Minimal spilling
- Balance between occupancy and memory access

### Approach: Identify Largest Register Consumers

From code analysis, estimated register usage:

**Blake2b State** (~50-70 regs):
- h[8] = 16 regs
- m[16] = 32 regs  
- t[2], f[2] = 4 regs
- Compression working variables v[16] = 32 regs during compression
- **Optimization**: Move some to shared memory

**VM State** (~40 regs):
- regs[8] = 16 regs
- prog_digest (64 bytes) = 16 regs ← **Target for removal**
- prog_seed (64 bytes) = 16 regs ← **Only needed at init**
- mem_digest_state (partial) = ~20 regs
- IP, loop_counter, memory_counter = 3 regs

**Instruction Processing** (~30-40 regs):
- Decoded instruction
- src1, src2, result
- Address calculations
- Temporaries

**Others**: ~50-80 regs

---

## Next Steps

### Phase A1b: Move Large Buffers to Shared Memory

**Target**: prog_seed (only needed at init)
- Currently: 16 registers × entire kernel lifetime
- Optimization: Load from shared memory at init only
- **Expected**: -16 registers

**Implementation**:
1. Allocate shared memory for prog_seeds
2. Load collaboratively at kernel start
3. Read from shared memory during init
4. Remove from VMState registers

**Estimated gain**: 255 → 239 registers, similar performance

Let's implement this carefully and test!

---

**Status**: Test 1 reverted, moving to manual optimization  
**Next**: Implement shared memory for prog_seed


## Test 1: __launch_bounds__(256, 6) ❌ FAILED

**Change**: Added `__launch_bounds__(256, 6)` to kernel

**Results**:
```
Before: 255 regs,   768 spill stores,   384 spill loads → 273k H/s
After:   40 regs, 11,422 spill stores, 13,336 spill loads → 131k H/s (-52%!)
```

**Analysis**:
- Compiler reduced registers from 255 → 40
- But spilled 14x more data to global memory
- Register spills are EXTREMELY expensive (100-400 cycles vs 1 cycle)
- **Result**: Massive performance loss

**Lesson**: Can't just force low register count - need balanced optimization

---

## New Strategy: Manual Optimization

### Target: 150-180 registers
- Allows 384 threads/block (150×384 = 57,600 < 65,536)
- Minimal spilling
- Balance between occupancy and memory access

### Approach: Identify Largest Register Consumers

From code analysis, estimated register usage:

**Blake2b State** (~50-70 regs):
- h[8] = 16 regs
- m[16] = 32 regs  
- t[2], f[2] = 4 regs
- Compression working variables v[16] = 32 regs during compression
- **Optimization**: Move some to shared memory

**VM State** (~40 regs):
- regs[8] = 16 regs
- prog_digest (64 bytes) = 16 regs ← **Target for removal**
- prog_seed (64 bytes) = 16 regs ← **Only needed at init**
- mem_digest_state (partial) = ~20 regs
- IP, loop_counter, memory_counter = 3 regs

**Instruction Processing** (~30-40 regs):
- Decoded instruction
- src1, src2, result
- Address calculations
- Temporaries

**Others**: ~50-80 regs

---

## Next Steps

### Phase A1b: Move Large Buffers to Shared Memory

**Target**: prog_seed (only needed at init)
- Currently: 16 registers × entire kernel lifetime
- Optimization: Load from shared memory at init only
- **Expected**: -16 registers

**Implementation**:
1. Allocate shared memory for prog_seeds
2. Load collaboratively at kernel start
3. Read from shared memory during init
4. Remove from VMState registers

**Estimated gain**: 255 → 239 registers, similar performance

Let's implement this carefully and test!

---

**Status**: Test 1 reverted, moving to manual optimization  
**Next**: Implement shared memory for prog_seed




