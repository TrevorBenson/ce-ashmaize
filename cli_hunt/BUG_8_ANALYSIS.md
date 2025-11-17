# Bug #8 Analysis - Memory Counter Discrepancy

## Finding
GPU has 1061 memory accesses vs CPU's 895 = **166 extra accesses**

## CPU Operand Logic (src/b2.rs)
```rust
let src1 = match op1 {
    Operand::Reg => vm.regs[r1 as usize],
    Operand::Memory => mem_access64!(vm, rom, lit1),
    Operand::Literal => lit1,
    Operand::Special1 => special1_value64!(vm),
    Operand::Special2 => special2_value64!(vm),
};
```

Operand ranges:
- 0-4: Reg (5 values)
- 5-8: Memory (4 values)
- 9-12: Literal (4 values)
- 13: Special1 (1 value)
- 14-15: Special2 (2 values)

## GPU Operand Logic (cuda/ashmaize.cu)
```cuda
if (instr.op1 < 5) {
    src1 = vm.regs[instr.r1];
} else if (instr.op1 < 9) {
    src1 = mem_access64(vm, rom, instr.lit1, rom_size);
} else if (instr.op1 < 13) {
    src1 = instr.lit1;
} else if (instr.op1 < 14) {
    src1 = special1_value64(vm.prog_digest_state);
} else {
    src1 = special2_value64(vm.mem_digest_state);
}
```

GPU ranges:
- 0-4: Reg (5 values) ✓
- 5-8: Memory (4 values) ✓
- 9-12: Literal (4 values) ✓
- 13: Special1 (1 value) ✓
- 14-15: Special2 (2 values) ✓

The ranges look correct!

## Hypothesis
The issue might be that BOTH operands are being evaluated even when one isn't needed?

Wait, looking at the CPU code, it evaluates both src1 and src2 regardless of the operation. Same with GPU. So that's not it.

## Another Hypothesis
Maybe special1_value64 or special2_value64 are somehow calling mem_access64 indirectly? Let me check those implementations.

Actually, looking at the code, special1 uses prog_digest_state and special2 uses mem_digest_state. They shouldn't be calling mem_access64.

## Real Issue?
Wait, let me check if Op2 instructions evaluate src2 when they shouldn't...

Looking at CPU:
```rust
Instr::Op2(operator) => {
    let src1 = match op1 { ... };  // Only evaluates src1
    
    let result = match operator {
        Op2::Neg => !src1,
        Op2::RotL => src1.rotate_left(r1 as u32),
        ...
    };
}
```

Looking at GPU... let me check if it evaluates src2 for Op2 instructions!

