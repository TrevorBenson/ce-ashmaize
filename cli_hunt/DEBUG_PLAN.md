# Debug Plan - Finding Remaining Bugs

## Status: 7 bugs fixed, ~90% complete

### Verified Working ✅
- Blake2b primitive
- Argon2 H' primitive
- VM initialization
- Instruction decoding
- Opcode/operand mappings
- prog_digest updates
- ROM access calculation
- Wrapping arithmetic (natural in CUDA)
- XOR operations in post_instructions

### Main Loop Comparison

**CPU (src/b2.rs):**
```rust
for _ in 0..nb_loops {
    vm.execute(rom, nb_instrs);
}

// Inside execute:
self.program.shuffle(&self.prog_seed);  // Calls hprime
for _ in 0..instr {
    self.step(rom)  // execute_one_instruction + ip++
}
self.post_instructions()
```

**GPU (cuda/ashmaize.cu):**
```cuda
for (uint32_t loop = 0; loop < nb_loops; ++loop) {
    hprime(program, program_size, vm.prog_seed, 64);
    
    for (uint32_t instr_idx = 0; instr_idx < nb_instrs; ++instr_idx) {
        execute_one_instruction(vm, rom_data, program + instr_idx * INSTR_SIZE, rom_size);
    }
    
    post_instructions(vm);
}
```

### Key Difference Found!

**CPU**: Uses `vm.ip` to index into program, wraps around with modulo
**GPU**: Uses `instr_idx` directly as sequential index into program

**This is the issue!** 

The CPU's `program.at(vm.ip)` uses wrapping with modulo:
```rust
let start = (i as usize).wrapping_mul(INSTR_SIZE) % self.instructions.len();
```

The GPU just does `program + instr_idx * INSTR_SIZE` which is sequential!

**But wait** - if we're executing instructions 0..255 in order, these should be the same...

Unless... let me check if vm.ip is used anywhere besides incrementing.

Actually, the CPU increments ip in `step()` but the program access uses `vm.ip`. So if ip wraps or is used differently, that could cause issues.

## Next Action

Check if there are any differences in how instructions are accessed from the program buffer.

The CPU accesses: `*vm.program.at(vm.ip)`
The GPU accesses: `program + instr_idx * INSTR_SIZE`

If nb_instrs <= program.instructions.len() / INSTR_SIZE, these should be equivalent.

But the CPU's post_instructions sets ip = 0 implicitly? No, I don't see that. Let me check the vm_init to see what ip starts at.

Actually, I need to verify: does the CPU's ip get reset between loops?

## Hypothesis

The issue might be that vm.ip is being used by execute_one_instruction internally, and the GPU is passing the program chunk directly but vm.ip might still be getting incremented and used elsewhere.

Wait - I just added prog_digest update to execute_one_instruction! Let me check if that's using prog_chunk correctly.

The CPU does: `vm.prog_digest.update(&prog_chunk);`
The GPU does: `blake2b_update(vm.prog_digest_state, prog_chunk, INSTR_SIZE);`

These should be equivalent.

## Action: Add Debug Logging

Need to add printf to GPU kernel to see intermediate values and compare with CPU.

