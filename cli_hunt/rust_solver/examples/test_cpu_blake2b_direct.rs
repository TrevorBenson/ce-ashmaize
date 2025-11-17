// Test what the CPU's blake2 crate produces by calling it directly

fn main() {
    use blake2::{Blake2b512, Digest};
    
    println!("=== Direct Blake2b Test ===\n");
    
    let src1 = 0x57aaaa9e896b78ed_u64;
    let src2 = 0xe8bcb6feb065ea96_u64;
    
    let mut input = [0u8; 16];
    input[0..8].copy_from_slice(&src1.to_le_bytes());
    input[8..16].copy_from_slice(&src2.to_le_bytes());
    
    println!("Input src1: 0x{:016x}", src1);
    println!("Input src2: 0x{:016x}", src2);
    println!("Input bytes: {}", hex::encode(&input));
    
    let out = Blake2b512::digest(&input);
    
    println!("\nBlake2b512::digest() output (64 bytes total):");
    for (i, chunk) in out.chunks(8).enumerate() {
        let val = u64::from_le_bytes(*<&[u8; 8]>::try_from(chunk).unwrap());
        println!("  Chunk {}: 0x{:016x}", i, val);
    }
    
    let v = 2usize;
    let chunk_2 = if let Some(chunk) = out.chunks(8).nth(v) {
        u64::from_le_bytes(*<&[u8; 8]>::try_from(chunk).unwrap())
    } else {
        panic!("chunk doesn't exist")
    };
    
    println!("\nExtracted chunk 2 (for opcode 250, v=2): 0x{:016x}", chunk_2);
    
    println!("\n=== COMPARISON ===");
    println!("GPU Blake2b result:       0x5694e7110ab9f4d3");
    println!("b2sum result:             0x5694e7110ab9f4d3");
    println!("blake2 crate result:      0x{:016x}", chunk_2);
    println!("CPU trace instr 0 result: 0x5555698eee0fbaef");
    
    if chunk_2 == 0x5694e7110ab9f4d3 {
        println!("\n✓ blake2 crate MATCHES GPU and b2sum!");
        println!("This means the CPU VM trace result 0x5555698eee0fbaef is from a DIFFERENT instruction!");
    } else if chunk_2 == 0x5555698eee0fbaef {
        println!("\n✓ blake2 crate MATCHES CPU trace!");
        println!("GPU Blake2b needs to be fixed to match this behavior!");
    } else {
        println!("\n? blake2 crate produces THIRD value: 0x{:016x}", chunk_2);
    }
}

