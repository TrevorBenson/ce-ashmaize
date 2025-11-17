// Compare CPU's blake2 crate with external reference

use blake2::{Blake2b512, Digest};

fn main() {
    println!("=== Compare Blake2b Implementations ===\n");
    
    // The exact input we traced
    let src1 = 0x57aaaa9e896b78ed_u64;
    let src2 = 0xe8bcb6feb065ea96_u64;
    
    let mut input = [0u8; 16];
    input[0..8].copy_from_slice(&src1.to_le_bytes());
    input[8..16].copy_from_slice(&src2.to_le_bytes());
    
    println!("Input (16 bytes):");
    println!("  src1: 0x{:016x}", src1);
    println!("  src2: 0x{:016x}", src2);
    println!("  Combined: {}", hex::encode(&input));
    
    // Use the EXACT same method as CPU VM (src/b2.rs line 408)
    let out = Blake2b512::digest(&input);
    
    println!("\nblake2 crate Blake2b512::digest() output:");
    for (i, chunk) in out.chunks(8).enumerate() {
        let val = u64::from_le_bytes(*<&[u8; 8]>::try_from(chunk).unwrap());
        println!("  Chunk {}: 0x{:016x}", i, val);
    }
    
    // Extract chunk 2 (v=2 for opcode 250)
    let v = 2usize;
    let chunk_2 = if let Some(chunk) = out.chunks(8).nth(v) {
        u64::from_le_bytes(*<&[u8; 8]>::try_from(chunk).unwrap())
    } else {
        panic!("chunk doesn't exist")
    };
    
    println!("\nExtracted chunk {} (opcode 250, v=2): 0x{:016x}", v, chunk_2);
    
    println!("\n=== COMPARISON ===");
    println!("GPU result:          0x5694e7110ab9f4d3");
    println!("b2sum result:        0x5694e7110ab9f4d3");
    println!("blake2 crate result: 0x{:016x}", chunk_2);
    println!("CPU instr 0 result:  0x5555698eee0fbaef (from trace_instructions)");
    
    if chunk_2 == 0x5694e7110ab9f4d3 {
        println!("\n✓ blake2 crate matches GPU/b2sum!");
        println!("But CPU trace showed 0x5555698eee0fbaef...");
        println!("This means the CPU VM is NOT executing the Blake2b we think it is!");
    } else if chunk_2 == 0x5555698eee0fbaef {
        println!("\n✓ blake2 crate matches CPU VM trace!");
        println!("GPU needs to be fixed to match this behavior!");
    } else {
        println!("\n✗ blake2 crate produces DIFFERENT result: 0x{:016x}", chunk_2);
    }
}

