# Critical Bugs Found and Fixed

## Summary
Found 5 bugs in the CUDA implementation that prevented CPU/GPU hash matching.

## Bug #1: loop_counter Encoding (FIXED)
**Location**: `cuda/ashmaize_vm.cuh` line 227-237  
**Issue**: loop_counter was encoded as 8 bytes (u64) instead of 4 bytes (u32)  
**Fix**: Changed from 136-byte mixing_input to 132-byte, only encode 4 bytes  
**Impact**: Hash changed from 62a2... to 55d4...

## Bug #2: Modulo Operator (FIXED)
**Location**: `cuda/ashmaize.cu` line 56  
**Issue**: Opcode range 112-127 was using `/` (division) instead of `%` (modulo)  
**Fix**: Changed `src1 / src2` to `src1 % src2`  
**Impact**: Hash changed from 55d4... to bc3f...

## Bug #3: Rotate Edge Case (FIXED - minor impact)
**Location**: `cuda/ashmaize.cu` lines 69-75  
**Issue**: RotL/RotR with shift=0 would cause undefined behavior (shift by 64)  
**Fix**: Added explicit check for shift==0  
**Impact**: No visible change (edge case rarely hit)

## Bug #4: ISqrt Implementation (FIXED)
**Location**: `cuda/ashmaize_vm.cuh` line 181-191  
**Issue**: Newton-Raphson formula was incorrect: `y = (x + x / x) / 2` should be `z = (z + x / z) / 2`  
**Fix**: Corrected the iterative square root algorithm  
**Impact**: Hash changed from bc3f... to e3f3...

## Bug #5: Blake2b Hash Chunk Selection (FIXED)
**Location**: `cuda/ashmaize.cu` lines 80-106  
**Issue**: Opcode 248-255 encodes which 8-byte chunk to return from 64-byte Blake2b output, but GPU always returned first chunk  
**Fix**: Added `v = instr.opcode - 248` and `offset = v * 8` to select correct chunk  
**Impact**: Hash changed from e3f3... to 83b7...

## Current Status
**Target**: 0ec51a39c5ba9d33  
**Current**: 83b757dea1a2e980  

Still not matching, but steady progress! Each fix changes the output.

## Progress Summary

| Fix # | Bug | Hash Result | Status |
|-------|-----|-------------|--------|
| Initial | - | 62a2f31a... | ✗ |
| 1 | loop_counter | 55d4c966... | ✗ |
| 2 | Modulo | bc3f7a5a... | ✗ |
| 3 | Rotate | bc3f7a5a... | ✗ (no change) |
| 4 | ISqrt | e3f36271... | ✗ |
| 5 | Blake2b chunk | 83b757de... | ✗ |

## Next Investigation
- Check mem_access64 more thoroughly
- Verify special1_value64 and special2_value64  
- Check sum calculation in post_instructions
- Verify wrapping behavior matches

## All Bugs Were In Metal Port
These bugs were introduced during the macOS Metal → Linux CUDA port.
The CPU implementation is correct and serves as the reference.
