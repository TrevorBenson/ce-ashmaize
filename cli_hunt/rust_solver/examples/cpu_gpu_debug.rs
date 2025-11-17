// CPU vs GPU Debug Tool
// Compares intermediate values to find where divergence occurs

use ashmaize::{Rom, RomGenerationType};
use ashmaize::b2::hash as cpu_hash;

#[cfg(feature = "cuda")]
use ashmaize_solver::gpu::CudaAshmaize;

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * MB;

fn main() {
    #[cfg(not(feature = "cuda"))]
    {
        eprintln!("This example requires the 'cuda' feature");
        std::process::exit(1);
    }

    #[cfg(feature = "cuda")]
    {
        println!("=== CPU vs GPU Detailed Debug ===\n");

        // Simple ROM for testing
        let rom = Rom::new(
            b"0",
            RomGenerationType::TwoStep {
                pre_size: 16 * MB,
                mixing_numbers: 4,
            },
            1 * GB,
        );

        let cuda = match CudaAshmaize::new() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to initialize CUDA: {:?}", e);
                std::process::exit(1);
            }
        };

        println!("GPU initialized: {}\n", cuda.get_device_info().unwrap());

        // Test cases
        let test_cases = vec![
            ("Simple", b"test" as &[u8]),
            ("Numeric", b"12345"),
            ("Hex-like", b"0000000000012345test1000000010011"),
            ("Long", b"0000000000012345test1000000010011aaaa"),
        ];

        for (name, salt) in test_cases {
            println!("Test Case: {}", name);
            println!("  Salt: {} ({} bytes)", String::from_utf8_lossy(salt), salt.len());
            
            // CPU hash
            let cpu_result = cpu_hash(salt, &rom, 8, 256);
            println!("  CPU: {}", hex::encode(&cpu_result[..16]));
            
            // GPU hash
            match cuda.hash_parallel(&[salt], &rom, 8, 256) {
                Ok(gpu_results) => {
                    let gpu_result = &gpu_results[0];
                    println!("  GPU: {}", hex::encode(&gpu_result[..16]));
                    
                    if cpu_result == *gpu_result {
                        println!("  ✓ MATCH\n");
                    } else {
                        println!("  ✗ MISMATCH");
                        println!("  Full CPU: {}", hex::encode(&cpu_result));
                        println!("  Full GPU: {}", hex::encode(gpu_result));
                        
                        // Find first differing byte
                        for i in 0..64 {
                            if cpu_result[i] != gpu_result[i] {
                                println!("  First difference at byte {}: CPU={:02x}, GPU={:02x}\n",
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

        // Test with varying salt lengths
        println!("\n=== Salt Length Variation Test ===\n");
        for len in [1, 2, 4, 8, 16, 32, 64, 128] {
            let salt: Vec<u8> = (0..len).map(|i| (i % 256) as u8).collect();
            
            let cpu_result = cpu_hash(&salt, &rom, 8, 256);
            match cuda.hash_parallel(&[salt.as_slice()], &rom, 8, 256) {
                Ok(gpu_results) => {
                    let gpu_result = &gpu_results[0];
                    let matches = cpu_result == *gpu_result;
                    let marker = if matches { "✓" } else { "✗" };
                    println!("  Length {:3}: {} CPU={} GPU={}",
                        len, marker,
                        hex::encode(&cpu_result[..8]),
                        hex::encode(&gpu_result[..8]));
                }
                Err(e) => {
                    println!("  Length {:3}: GPU Error: {:?}", len, e);
                }
            }
        }
    }
}

