# Bug #9: Root Cause Found - Memory Access Discrepancy

## Critical Discovery

After adding detailed instruction-level tracing with proper decoding, we found:

### Matching Components
1. ✓ ROM digest: Identical
2. ✓ prog_seed: Identical
3. ✓ Program buffer: Identical
4. ✓ Instruction decoding: Identical (verified for first 10 instructions)
5. ✓ memory_counter: 850 (both CPU and GPU)
6. ✓ Instructions using ONLY literals & registers: SAME results

### Divergence Point
**Instructions using memory operands produce DIFFERENT results!**

#### Instruction 0 (Blake2b Hash, opcode=250)
- Decoded: opcode=250, op1=11, op2=6, r1=27, r2=4, r3=3
- op2=6 → Memory operand
- CPU result: `regs[3]=0x5555698eee0fbaef`
- GPU result: `regs[3]=0x5694e7110ab9f4d3`
- **Status**: MISMATCH ✗

#### Instruction 1 (RotL, opcode=185)  
- Decoded: opcode=185, op1=6, op2=8, r1=21, r2=1, r3=4
- op1=6, op2=8 → Both memory operands
- CPU result: `regs[4]=0xd62adea5785e8b87`
- GPU result: `regs[4]=0x149633d675531c23`
- **Status**: MISMATCH ✗

#### Instruction 2 (Mul, opcode=63)
- Decoded: opcode=63, op1=9, op2=3, r1=9, r2=20, r3=20
- op1=9 → Literal, op2=3 → Register
- CPU result: `regs[20]=0x1ef592f0392ad6f4`
- GPU result: `regs[20]=0x1ef592f0392ad6f4`
- **Status**: MATCH ✓

## Conclusion

**The bug is in `mem_access64`!**

Despite fixing Bug #7 (ROM chunk access calculation), there's still a discrepancy in how `mem_access64` produces values on CPU vs GPU.

## Next Step

Need to add detailed logging inside `mem_access64` to see:
1. What `addr` value is being passed
2. What chunk is being read from ROM
3. What chunk index is calculated
4. What the mem_digest_state contains before/after update
5. What 8-byte value is being returned

This will pinpoint the exact difference in memory access behavior.

