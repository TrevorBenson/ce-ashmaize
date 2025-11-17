# Bug #9 Analysis: Register Value Discrepancy

## Summary
After fixing ROM parameters in test code, we now have:
- ✓ ROM digest matches
- ✓ prog_seed matches
- ✓ Program buffer matches
- ✓ memory_counter matches (850)
- ✗ Register values DIFFERENT!

## Status
Bug #9 was NOT a real bug in GPU code - just test code using wrong ROM parameters.
The ACTUAL issue is register value divergence despite identical inputs and memory access patterns.

## Current Status (After Bug #8 Fix)

### What Matches
1. ROM digest: `17c436644b8a8317...`
2. init_buffer_input: identical
3. prog_seed: `be0d98823d659f0a...`
4. Program buffer: `fab66c83ed786b89...`
5. memory_counter: 850 (both CPU and GPU)
6. loop_counter: 8 (both CPU and GPU)

### What DOESN'T Match
**CPU Before Finalize:**
- regs[0]: `0xbdd8234fcca7ab60`
- regs[1]: `0x2a1ab89f73033493`
- regs[31]: `0x9f25ccf5f2ee0ade`
- Final hash: `0ec51a39c5ba9d33...`

**GPU Before Finalize:**
- regs[0]: `0xdeb5babdaea07a94`
- regs[1]: `0xed961d3ac44eb537`
- regs[31]: `0xb92cbc7bb7103374`
- Final hash: `f4492b121af62236...`

## Hypothesis
Instruction execution is producing different results even though:
- Same program is being executed
- Same number of memory accesses occur
- Same number of loops complete

This suggests a bug in one of the instruction operations (arithmetic, bitwise, or special value computation).

## Next Steps
1. Add instruction-level tracing for first 5-10 instructions
2. Compare register values after each instruction
3. Identify the FIRST instruction where results diverge
4. Deep-dive into that specific instruction's implementation

