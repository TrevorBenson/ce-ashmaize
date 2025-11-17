// Verify salt is being passed correctly to GPU

use ashmaize::{Rom, RomGenerationType};

#[cfg(feature = "cuda")]
use ashmaize_solver::gpu::CudaAshmaize;

const MB: usize = 1_024 * 1_024;

fn main() {
    #[cfg(not(feature = "cuda"))]
    {
        eprintln!("This example requires the 'cuda' feature");
        std::process::exit(1);
    }

    #[cfg(feature = "cuda")]
    {
        println!("=== Verify Salt Passing ===\n");

        let rom = Rom::new(
            b"0",
            RomGenerationType::TwoStep {
                pre_size: 1 * MB,
                mixing_numbers: 1,
            },
            1 * MB,
        );

        // Test with different salt lengths
        let salts: Vec<&[u8]> = vec![
            b"a",          // 1 byte
            b"test",       // 4 bytes  
            b"longer",     // 6 bytes
        ];

        println!("Input salts:");
        for (i, salt) in salts.iter().enumerate() {
            println!("  [{}] len={}, data={}", i, salt.len(), hex::encode(salt));
        }
        
        // Find max length (what GPU will use)
        let max_len = salts.iter().map(|s| s.len()).max().unwrap();
        println!("\nMax salt length: {}", max_len);
        println!("GPU will pad all salts to {} bytes\n", max_len);

        // Show what the padded buffer will look like
        println!("Padded salt buffer sent to GPU:");
        for (i, salt) in salts.iter().enumerate() {
            let mut padded = salt.to_vec();
            padded.resize(max_len, 0);
            println!("  [{}] {}", i, hex::encode(&padded));
        }

        let cuda = CudaAshmaize::new().expect("CUDA init failed");
        
        println!("\nCalling GPU hash_parallel...");
        match cuda.hash_parallel(&salts, &rom, 2, 256) {
            Ok(results) => {
                println!("Success! Got {} results", results.len());
                for (i, result) in results.iter().enumerate() {
                    println!("  [{}] {}", i, hex::encode(&result[..16]));
                }
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
            }
        }
    }
}

