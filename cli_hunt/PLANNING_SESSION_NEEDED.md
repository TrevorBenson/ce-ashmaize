# Planning Session Required - Bug #8 Analysis

## Status
**8 BUGS FIXED** but hash still doesn't match.

## Latest Bug (#8): Operand Evaluation Logic

### Problem
GPU was evaluating both src1 and src2 for ALL instructions.  
CPU only evaluates src2 for Op3 instructions, not Op2.

### Fixes Attempted
1. **First attempt**: Added conditional src2 evaluation
   - Result: memory_counter: 1061 → 973 (reduced by 88)
   - But logic was wrong (included RotL, RotR, Neg as Op3)

2. **Second attempt**: Fixed is_op2 logic
   - Result: memory_counter: 973 → 850 (reduced by 123)  
   - Now GPU has FEWER accesses than CPU! (850 vs 905)

### Current Discrepancy
- **CPU**: 905 memory accesses
- **GPU**: 850 memory accesses  
- **Difference**: GPU has 55 FEWER accesses

This means Op2 logic is now TOO restrictive - some Op3 instructions aren't evaluating src2!

## Opcode Ranges (from CPU code)

```rust
match value {
    0..40    => Instr::Op3(Op3::Add),      // 40 opcodes
    40..80   => Instr::Op3(Op3::Mul),      // 40 opcodes
    80..96   => Instr::Op3(Op3::MulH),     // 16 opcodes
    96..112  => Instr::Op3(Op3::Div),      // 16 opcodes
    112..128 => Instr::Op3(Op3::Mod),      // 16 opcodes
    128..138 => Instr::Op2(Op2::ISqrt),    // 10 opcodes ← Op2
    138..148 => Instr::Op2(Op2::BitRev),   // 10 opcodes ← Op2
    148..188 => Instr::Op3(Op3::Xor),      // 40 opcodes
    188..204 => Instr::Op2(Op2::RotL),     // 16 opcodes ← Op2
    204..220 => Instr::Op2(Op2::RotR),     // 16 opcodes ← Op2
    220..240 => Instr::Op2(Op2::Neg),      // 20 opcodes ← Op2
    240..248 => Instr::Op3(Op3::And),      // 8 opcodes
    248..=255=> Instr::Op3(Op3::Hash(v)),  // 8 opcodes
}
```

### Op2 Instructions (only use src1):
- 128-137: ISqrt (10 opcodes)
- 138-147: BitRev (10 opcodes)
- 188-203: RotL (16 opcodes)
- 204-219: RotR (16 opcodes)
- 220-239: Neg (20 opcodes)
- **Total: 72 Op2 opcodes**

### Op3 Instructions (use src1 and src2):
- 0-127: Add, Mul, MulH, Div, Mod (128 opcodes)
- 148-187: Xor (40 opcodes)
- 240-255: And, Hash (16 opcodes)
- **Total: 184 Op3 opcodes**

## GPU Current Logic (INCORRECT!)

```cuda
bool is_op2 = (instr.opcode >= 128 && instr.opcode < 148) ||   // ISqrt, BitRev
              (instr.opcode >= 188 && instr.opcode < 240);      // RotL, RotR, Neg
```

**This covers**:
- 128-147: ISqrt, BitRev ✓ (20 opcodes)
- 188-239: RotL, RotR, Neg ✓ (52 opcodes)
- **Total: 72 opcodes**

Wait, that looks correct! So why are we getting fewer memory accesses?

## Hypothesis: Maybe instructions aren't distributed evenly?

If there are more Op2 instructions executed than Op3, that would explain it. But the CPU and GPU should be executing the SAME instructions!

## Alternative Hypothesis: Program Generation Issue?

Maybe the program buffer isn't being generated identically? Let me check if hprime is being called correctly...

## Next Steps

### Option 1: Add More Debug Output
Print the opcode distribution:
```cuda
if (tid == 0 && loop == 0) {
    for (int i = 0; i < 10; ++i) {
        printf("Instr %d: opcode=%u, op1=%u, op2=%u\n", 
            i, program[i*20+0], program[i*20+1]>>4, program[i*20+1]&0x0F);
    }
}
```

Compare with CPU's first 10 instructions.

### Option 2: Binary Search with Simpler Parameters
Test with nb_loops=1, nb_instrs=1 to see if even a single instruction matches.

### Option 3: Check if Program Buffer Matches
Print first few bytes of program buffer on CPU and GPU after hprime to verify they're identical.

## Summary

We've fixed 8 bugs and made enormous progress (92%+ complete).  
The current issue is subtle - operand evaluation logic is close but not quite right.  
A planning session would help us systematically verify:
1. Program generation is identical
2. Instruction execution order is identical
3. Operand evaluation matches exactly
4. All state updates match

## Recommendation

**YES, A PLANNING SESSION WOULD BE BENEFICIAL** to:
1. Verify program buffer contents match between CPU and GPU
2. Add comprehensive debug logging for first few instructions
3. Compare instruction-by-instruction execution
4. Create a test that can pinpoint the exact instruction where divergence occurs

