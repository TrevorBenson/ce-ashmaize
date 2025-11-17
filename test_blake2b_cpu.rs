use blake2::{Blake2b512, Digest};

fn main() {
    println!("=== Compare Blake2b Implementations ===\n");
    
    let src1 = 0x57aaaa9e896b78ed_u64;
    let src2 = 0xe8bcb6feb065ea96_u64;
    
    let mut input = [0u8; 16];
    input[0..8].copy_from_slice(&src1.to_le_bytes());
    input[8..16].copy_from_slice(&src2.to_le_bytes());
    
    println!("Input: {}", hex::encode(&input));
    
    let out = Blake2b512::digest(&input);
    
    for (i, chunk) in out.chunks(8).enumerate() {
        let val = u64::from_le_bytes(*<&[u8; 8]>::try_from(chunk).unwrap());
        println!("Chunk {}: 0x{:016x}", i, val);
    }
    
    let v = 2usize;
    let chunk_2 = if let Some(chunk) = out.chunks(8).nth(v) {
        u64::from_le_bytes(*<&[u8; 8]>::try_from(chunk).unwrap())
    } else {
        panic!("chunk doesn't exist")
    };
    
    println!("\nExtracted chunk 2: 0x{:016x}", chunk_2);
    println!("GPU result:        0x5694e7110ab9f4d3");
    println!("CPU trace result:  0x5555698eee0fbaef");
    
    if chunk_2 == 0x5694e7110ab9f4d3 {
        println!("\n✓ blake2 crate matches GPU - CPU VM must be doing something different!");
    } else if chunk_2 == 0x5555698eee0fbaef {
        println!("\n✓ blake2 crate matches CPU trace - GPU needs to match this!");
    } else {
        println!("\n✗ blake2 crate is different: 0x{:016x}", chunk_2);
    }
}
