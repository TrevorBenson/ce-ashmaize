// Verify ROM chunk 14149808 contents to ensure CPU and GPU see the same ROM data

use ashmaize::{Rom, RomGenerationType};

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * 1_024 * 1_024;

fn main() {
    println!("=== Verify ROM Chunk Contents ===\n");

    let rom = Rom::new(
        b"0",
        RomGenerationType::TwoStep {
            pre_size: 16 * MB,
            mixing_numbers: 4,
        },
        1 * GB,
    );

    println!("ROM size: {} bytes ({} MB)", rom.data.len(), rom.data.len() / (1024 * 1024));
    println!("ROM digest: {}", hex::encode(rom.digest.as_bytes()));
    
    // Check chunk 14149808
    let chunk_index = 14149808usize;
    let chunk_start = chunk_index * 64;
    let chunk_end = chunk_start + 64;
    
    println!("\nChunk {} details:", chunk_index);
    println!("  Start byte: {}", chunk_start);
    println!("  End byte: {}", chunk_end);
    
    if chunk_end <= rom.data.len() {
        let chunk = &rom.data[chunk_start..chunk_end];
        
        println!("\n  Full chunk (64 bytes):");
        for (i, byte) in chunk.iter().enumerate() {
            if i % 16 == 0 {
                print!("    ");
            }
            print!("{:02x}", byte);
            if (i + 1) % 16 == 0 {
                println!();
            }
        }
        
        println!("\n  First 16 bytes: {}", hex::encode(&chunk[..16]));
        println!("  Bytes 8-15: {}", hex::encode(&chunk[8..16]));
        
        // Extract as little-endian u64
        let bytes_8_15 = u64::from_le_bytes(*<&[u8; 8]>::try_from(&chunk[8..16]).unwrap());
        println!("  Bytes 8-15 as u64: 0x{:016x}", bytes_8_15);
        
        println!("\n=== COMPARISON ===");
        println!("GPU saw first 16 bytes: c9d497336afdd8fb96ea65b0feb6bce8");
        println!("CPU chunk first 16 bytes: {}", hex::encode(&chunk[..16]));
        
        if hex::encode(&chunk[..16]) == "c9d497336afdd8fb96ea65b0feb6bce8" {
            println!("✓ MATCH - ROM data is identical!");
        } else {
            println!("✗ MISMATCH - ROM data is DIFFERENT!");
        }
        
        println!("\nGPU extracted: 0xe8bcb6feb065ea96");
        println!("CPU should get: 0x{:016x}", bytes_8_15);
        
        if bytes_8_15 == 0xe8bcb6feb065ea96 {
            println!("✓ MATCH - Extraction is identical!");
        } else {
            println!("✗ MISMATCH - Extraction is DIFFERENT!");
        }
    } else {
        println!("ERROR: Chunk {} is out of bounds!", chunk_index);
    }
}

