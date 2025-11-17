// Test Blake2b Hash operation (opcode 250)

fn main() {
    println!("=== Blake2b Hash Operation Test ===\n");
    
    // Instruction 0 has opcode 250 (Blake2b Hash, v=2)
    let opcode = 250u8;
    let v = opcode - 248;  // v = 2
    
    // Both CPU and GPU have these values:
    let src1: u64 = 0x57aaaa9e896b78ed;
    let src2: u64 = 0xe8bcb6feb065ea96;
    
    println!("Opcode: {} (Blake2b Hash, v={})", opcode, v);
    println!("src1: 0x{:016x}", src1);
    println!("src2: 0x{:016x}", src2);
    
    // Create 16-byte input from src1 and src2 (little-endian)
    let mut input = [0u8; 16];
    input[0..8].copy_from_slice(&src1.to_le_bytes());
    input[8..16].copy_from_slice(&src2.to_le_bytes());
    
    print!("\nInput (16 bytes): ");
    for b in &input {
        print!("{:02x}", b);
    }
    println!();
    
    // Compute Blake2b-512
    let hash = blake2b_simd::Params::new()
        .hash_length(64)
        .to_state()
        .update(&input)
        .finalize();
    
    println!("\nBlake2b-512 output (64 bytes):");
    for (i, chunk) in hash.chunks(8).enumerate() {
        print!("  Chunk {}: ", i);
        for b in chunk {
            print!("{:02x}", b);
        }
        print!(" = 0x");
        let val = u64::from_le_bytes(*<&[u8; 8]>::try_from(chunk).unwrap());
        println!("{:016x}", val);
    }
    
    // Extract chunk v (should be chunk 2)
    let result_bytes = &hash[(v as usize) * 8..(v as usize) * 8 + 8];
    let result = u64::from_le_bytes(*<&[u8; 8]>::try_from(result_bytes).unwrap());
    
    println!("\nExtracted chunk {} (v={}): 0x{:016x}", v, v, result);
    println!("\n=== EXPECTED RESULTS ===");
    println!("CPU result: 0x5555698eee0fbaef");
    println!("GPU result: 0x5694e7110ab9f4d3");
    println!("Our result: 0x{:016x}", result);
    
    if result == 0x5555698eee0fbaef {
        println!("✓ Matches CPU!");
    } else if result == 0x5694e7110ab9f4d3 {
        println!("✓ Matches GPU!");
    } else {
        println!("✗ Matches neither - something else is wrong!");
    }
}

