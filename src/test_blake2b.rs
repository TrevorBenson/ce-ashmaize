#[cfg(test)]
mod tests {
    use blake2::{Blake2b512, Digest};

    #[test]
    fn test_blake2b_direct() {
        println!("\n=== Testing blake2 crate directly ===");
        
        let src1 = 0x57aaaa9e896b78ed_u64;
        let src2 = 0xe8bcb6feb065ea96_u64;
        
        let mut input = [0u8; 16];
        input[0..8].copy_from_slice(&src1.to_le_bytes());
        input[8..16].copy_from_slice(&src2.to_le_bytes());
        
        println!("Input (hex): {:02x?}", input);
        
        let out = Blake2b512::digest(&input);
        
        println!("\nBlake2b512::digest() output:");
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
        
        println!("\nExtracted chunk 2 (v=2): 0x{:016x}", chunk_2);
        println!("GPU result:              0x5694e7110ab9f4d3");
        println!("CPU trace showed:        0x5555698eee0fbaef");
        
        if chunk_2 == 0x5694e7110ab9f4d3 {
            println!("\n✓ blake2 crate MATCHES GPU!");
        } else if chunk_2 == 0x5555698eee0fbaef {
            println!("\n✓ blake2 crate MATCHES CPU trace!");
        } else {
            println!("\n? blake2 crate produces: 0x{:016x}", chunk_2);
        }
    }
}

