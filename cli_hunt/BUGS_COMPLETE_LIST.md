# All Bugs Found and Fixed - Complete List

## Summary: 7 Bugs Found

| # | Bug | File | Fix | Impact |
|---|-----|------|-----|--------|
| 1 | loop_counter encoding | ashmaize_vm.cuh | 8→4 bytes | 62a2...→55d4... |
| 2 | Modulo operator | ashmaize.cu | /→% | 55d4...→bc3f... |
| 3 | Rotate edge case | ashmaize.cu | Add shift==0 check | bc3f... (no change) |
| 4 | ISqrt algorithm | ashmaize_vm.cuh | Fix Newton-Raphson | bc3f...→e3f3... |
| 5 | Blake2b chunk selection | ashmaize.cu | Add opcode param | e3f3...→83b7... |
| 6 | prog_digest update missing | ashmaize.cu | Add blake2b_update | 83b7...→83e2... |
| 7 | ROM access calculation | ashmaize_vm.cuh | Chunk index vs byte offset | 83e2...→fcf0... |

## Bug #1: loop_counter Encoding ✅
**Location**: `cuda/ashmaize_vm.cuh` line 227-237  
**Issue**: loop_counter was encoded as 8 bytes (u64) instead of 4 bytes (u32)  
**Fix**: Changed from 136-byte mixing_input to 132-byte, only encode 4 bytes  
**Root Cause**: Assumed u64, should be u32

## Bug #2: Modulo Operator ✅
**Location**: `cuda/ashmaize.cu` line 56  
**Issue**: Opcode range 112-127 was using `/` (division) instead of `%` (modulo)  
**Fix**: Changed `src1 / src2` to `src1 % src2`  
**Root Cause**: Copy-paste error from Div operation

## Bug #3: Rotate Edge Case ✅  
**Location**: `cuda/ashmaize.cu` lines 69-75  
**Issue**: RotL/RotR with shift=0 would cause undefined behavior (shift by 64)  
**Fix**: Added explicit check for shift==0  
**Root Cause**: Didn't handle edge case

## Bug #4: ISqrt Algorithm ✅
**Location**: `cuda/ashmaize_vm.cuh` line 181-191  
**Issue**: Newton-Raphson formula wrong: `y = (x + x/x)/2`  
**Fix**: Corrected to `z = (z + x/z)/2` with proper variables  
**Root Cause**: Variable reuse broke iterative algorithm

## Bug #5: Blake2b Hash Chunk Selection ✅
**Location**: `cuda/ashmaize.cu` lines 80-106  
**Issue**: Opcode 248-255 encodes which 8-byte chunk to return from 64-byte Blake2b output, but GPU always returned first chunk  
**Fix**: Added `v = instr.opcode - 248` and `offset = v * 8` to select correct chunk  
**Root Cause**: Didn't implement opcode parameter

## Bug #6: prog_digest Update Missing ✅
**Location**: `cuda/ashmaize.cu` line 110 (end of execute_one_instruction)  
**Issue**: CPU updates prog_digest with prog_chunk at end of every instruction execution, GPU didn't  
**Fix**: Added `blake2b_update(vm.prog_digest_state, prog_chunk, INSTR_SIZE);`  
**Root Cause**: Missed during porting

## Bug #7: ROM Access Calculation ✅
**Location**: `cuda/ashmaize_vm.cuh` line 124-134  
**Issue**: CPU treats addr as CHUNK INDEX (addr % num_chunks), GPU treated it as BYTE OFFSET (addr % rom_size)  
**Fix**: Changed to calculate chunk_index first: `chunk_index = addr % (rom_size/64)`, then `offset = chunk_index * 64`  
**Root Cause**: Misunderstood the CPU's rom.at() implementation

## Current Status
**Target**: 0ec51a39c5ba9d33  
**Current**: fcf0bacb22068bc9  

7 bugs fixed, still converging!

## Pattern
Every bug fix changes the output = making real progress!
All bugs were in the Metal→CUDA port.
CPU implementation is the correct reference.

