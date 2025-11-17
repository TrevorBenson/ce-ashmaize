// Compare opcode ranges between CPU enum and GPU manual checks

fn main() {
    println!("=== CPU Instruction Decoding (from src/b2.rs) ===\n");
    
    println!("Op2 Instructions:");
    println!("  ISqrt:  128..138 (10 opcodes)");
    println!("  BitRev: 138..148 (10 opcodes)");
    println!("  RotL:   188..204 (16 opcodes)");
    println!("  RotR:   204..220 (16 opcodes)");
    println!("  Neg:    220..240 (20 opcodes)");
    println!("  Total Op2: 10 + 10 + 16 + 16 + 20 = 72 opcodes");
    
    println!("\nOp3 Instructions (everything else):");
    println!("  Add:    0..40    (40 opcodes)");
    println!("  Mul:    40..80   (40 opcodes)");
    println!("  MulH:   80..96   (16 opcodes)");
    println!("  Div:    96..112  (16 opcodes)");
    println!("  Mod:    112..128 (16 opcodes)");
    println!("  Xor:    148..188 (40 opcodes)");
    println!("  And:    240..248 (8 opcodes)");
    println!("  Hash:   248..256 (8 opcodes)");
    println!("  Total Op3: 40 + 40 + 16 + 16 + 16 + 40 + 8 + 8 = 184 opcodes");
    
    println!("\n=== GPU Manual Opcode Checks (from cuda/ashmaize.cu) ===\n");
    
    println!("is_op2 check:");
    println!("  (opcode >= 128 && opcode < 148) ||  // 128-147: ISqrt + BitRev");
    println!("  (opcode >= 188 && opcode < 240)     // 188-239: RotL + RotR + Neg");
    
    println!("\nCoverage:");
    println!("  128-147: 20 opcodes (ISqrt 128-137 + BitRev 138-147)");
    println!("  188-239: 52 opcodes (RotL 188-203 + RotR 204-219 + Neg 220-239)");
    println!("  Total: 20 + 52 = 72 opcodes");
    
    println!("\n=== VERIFICATION ===");
    println!("CPU Op2: 72 opcodes");
    println!("GPU Op2: 72 opcodes");
    println!("✓ Count matches!");
    
    println!("\nBut let's check individual ranges:");
    println!("CPU ISqrt:  128-137 (inclusive)");
    println!("GPU check:  >= 128 && < 148 (covers 128-147)");
    println!("  This includes ISqrt (128-137) AND BitRev (138-147) ✓");
    
    println!("\nCPU BitRev: 138-147 (inclusive)");
    println!("GPU check:  >= 128 && < 148 (covers 128-147)");
    println!("  This includes BitRev (138-147) ✓");
    
    println!("\nCPU RotL:   188-203 (inclusive)");
    println!("GPU check:  >= 188 && < 240 (covers 188-239)");
    println!("  This includes RotL (188-203) ✓");
    
    println!("\nCPU RotR:   204-219 (inclusive)");
    println!("GPU check:  >= 188 && < 240 (covers 188-239)");
    println!("  This includes RotR (204-219) ✓");
    
    println!("\nCPU Neg:    220-239 (inclusive)");
    println!("GPU check:  >= 188 && < 240 (covers 188-239)");
    println!("  This includes Neg (220-239) ✓");
    
    println!("\n=== CONCLUSION ===");
    println!("GPU opcode ranges correctly identify all Op2 instructions!");
    println!("is_op2 logic appears to be correct.");
    println!("\nThe 63 extra memory accesses must come from something else...");
}

