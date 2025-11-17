// Minimal test to isolate CPU vs GPU discrepancy

use ashmaize::{Rom, RomGenerationType};
use ashmaize::b2::hash as cpu_hash;

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
        println!("=== Minimal Hash Test - CPU vs GPU ===\n");

        // Create SMALLEST possible ROM to minimize variables
        let rom = Rom::new(
            b"0",
            RomGenerationType::TwoStep {
                pre_size: 1 * MB,  // Minimum
                mixing_numbers: 1,  // Minimum
            },
            1 * MB,  // Minimum
        );

        println!("ROM size: {} bytes", rom.data.len());
        println!("ROM digest: {}\n", hex::encode(&rom.digest.as_bytes()[..16]));

        let cuda = match CudaAshmaize::new() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to initialize CUDA: {:?}", e);
                std::process::exit(1);
            }
        };

        // Test with minimal parameters
        let test_cases = vec![
            ("empty", b"" as &[u8]),
            ("a", b"a"),
            ("test", b"test"),
        ];

        // Use MINIMUM loop and instruction counts
        let nb_loops = 2;  // Minimum allowed
        let nb_instrs = 256;  // Minimum allowed

        println!("Parameters: nb_loops={}, nb_instrs={}", nb_loops, nb_instrs);
        println!();

        for (name, salt) in test_cases {
            println!("Test: '{}'", name);
            
            // CPU
            let cpu_result = cpu_hash(salt, &rom, nb_loops, nb_instrs);
            println!("  CPU: {}", hex::encode(&cpu_result[..16]));
            
            // GPU
            match cuda.hash_parallel(&[salt], &rom, nb_loops, nb_instrs) {
                Ok(gpu_results) => {
                    let gpu_result = &gpu_results[0];
                    println!("  GPU: {}", hex::encode(&gpu_result[..16]));
                    
                    if cpu_result == *gpu_result {
                        println!("  ✓ MATCH\n");
                    } else {
                        println!("  ✗ MISMATCH");
                        
                        // Find first byte that differs
                        for i in 0..64 {
                            if cpu_result[i] != gpu_result[i] {
                                println!("  First diff at byte {}: CPU={:02x}, GPU={:02x}\n",
                                    i, cpu_result[i], gpu_result[i]);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("  GPU Error: {:?}\n", e);
                }
            }
        }
    }
}

