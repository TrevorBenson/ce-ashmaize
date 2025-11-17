# Bug #14: CPU Modulo Operation Does Division (CRITICAL FIX)

## Discovery
**Date:** Current session  
**Discovered by:** User observation  
**Impact:** CRITICAL - This was the final bug preventing hash correctness!

## Problem
The CPU's Modulo operation (opcodes 112-127, `Op3::Mod`) was performing **DIVISION** (`src1 / src2`) instead of **MODULO** (`src1 % src2`).

### CPU Implementation (Buggy)
```rust
Op3::Mod => {
    if src2 == 0 {
        special1_value64!(vm)
    } else {
        src1 / src2  // ❌ BUG! Should be src1 % src2
    }
}
```

This was a copy-paste bug from the `Op3::Div` operation.

## Root Cause
The CPU code for `Op3::Mod` was copied from `Op3::Div` but the operator was never changed from `/` to `%`.

## Symptoms
- **First divergence at instruction 37** (opcode 122, which is in the Modulo range)
- CPU produced `0x0000000000000000` for `regs[9]`
- GPU produced `0x7dfabfb5af526461` for `regs[9]`
- This cascaded into different register values for all subsequent instructions
- Final `sum_regs` diverged: CPU `0xf42c649be38b7aa1` vs GPU `0x4948c48f8c61118b`
- This caused different `prog_seed` values after loop 0, leading to completely different program shuffling

## Fix
Since the CPU is the reference implementation (even with bugs), the GPU was updated to match the CPU's buggy behavior:

### GPU Implementation (Updated to Match CPU Bug)
```cuda
} else if (instr.opcode < 128) {
    // BUG #14: CPU's Modulo operation actually does DIVISION (src1 / src2)
    // This is a copy-paste bug in the CPU, but we must match it!
    result = (src2 != 0) ? src1 / src2 : special1_value64(vm.prog_digest_state);
}
```

## Verification
After the fix, all test cases pass:

```
Testing nonce 0000000000012345:
CPU result: 0ec51a39c5ba9d33
GPU result: 0ec51a39c5ba9d33
✓ Results match!

Testing nonce 0000000000000001:
CPU result: a94f92649614e509
GPU result: a94f92649614e509
✓ Results match!

Testing nonce 00000000FFFFFFFF:
CPU result: 695b9146321147b2
GPU result: 695b9146321147b2
✓ Results match!
```

## Files Modified
- `cli_hunt/rust_solver/cuda/ashmaize.cu`: Line 89 changed from `src1 % src2` to `src1 / src2`

## Lessons Learned
1. **User domain expertise is invaluable** - The user immediately suspected the Modulo operation based on the symptoms
2. **Copy-paste errors are insidious** - The bug existed because code was duplicated without changing the operator
3. **Reference implementation bugs must be matched** - Even though the GPU implementation was "correct" mathematically, it had to match the CPU's buggy behavior for hash validation to work

## Impact
This was the **final bug** in the CPU vs GPU hash discrepancy. With this fix:
- ✅ All instructions now execute identically on CPU and GPU
- ✅ VM state (registers, memory_counter, loop_counter) matches perfectly
- ✅ Hash results are identical across all test cases
- ✅ GPU acceleration can now be used for production mining

## Related Bugs
This bug was only discovered after fixing Bugs #1-#13:
1. Loop counter encoding (4 vs 8 bytes)
2. Redundant loop counter increment
3. Division operator in wrong opcode range
4. Rotate edge case (shift by 64)
5. ISqrt algorithm
6. Missing prog_digest update
7. ROM access calculation
8. Op2/Op3 operand evaluation
9. ROM test parameters
10. Register index masking
11. CPU's rom.at() treats chunk index as byte offset
12. (False alarm - GPU ip handling)
13. (False alarm - GPU ip reset)
14. **Modulo does Division** ← This one!

Without fixing all previous bugs, this final bug would not have been visible in the test traces.

