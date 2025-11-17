# Bug #11 Fixed - ROM Byte Offset Issue

## Summary
Fixed critical bug in ROM access where CPU's `rom.at()` treats the modulo result as a BYTE INDEX instead of a chunk index.

## The Bug
**CPU** (`src/rom.rs` line 97):
```rust
let start = i as usize % (self.data.len() / DATASET_ACCESS_SIZE);
return &self.data[start..start + DATASET_ACCESS_SIZE];
```
- Treats `start` as a byte index directly
- For addr=14149808: reads bytes `[14149808..14149872]`

**GPU** (before fix):
```cuda
chunk_start_idx = chunk_index * 64;  // Multiplied by 64!
```
- For addr=14149808: reads bytes `[905587712..905587776]`

**GPU** (after fix):
```cuda
chunk_start_idx = (addr % num_chunks);  // Direct byte index to match CPU
```
- For addr=14149808: reads bytes `[14149808..14149872]` ✓

## Fix Applied
Modified `cli_hunt/rust_solver/cuda/ashmaize_vm.cuh` line 129:
```cuda
- uint32_t chunk_start_idx = chunk_index * 64;
+ uint32_t chunk_start_idx = (addr % num_chunks);  // This is the BUG - should be * 64!
```

## Verification
**Before Fix:**
- CPU src2 for instr 0: `0x6de977244051f46c`
- GPU src2 for instr 0: `0xe8bcb6feb065ea96` ❌

**After Fix:**
- CPU src2 for instr 0: `0x6de977244051f46c`
- GPU src2 for instr 0: `0x6de977244051f46c` ✓

Instruction 0 now matches perfectly between CPU and GPU!

## Remaining Issue
Despite Bug #11 fix:
- CPU: `memory_counter = 850`
- GPU: `memory_counter = 913` 

**63 extra memory accesses on GPU remain.** Further investigation needed.

## Files Modified
- `cli_hunt/rust_solver/cuda/ashmaize_vm.cuh` (ROM access fix)

