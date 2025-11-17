// Phase 7: Dynamic Batch Sizing
// This version tests different batch sizes to find optimal performance

use ashmaize::{Rom, RomGenerationType};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Instant;

#[cfg(feature = "cuda")]
use ashmaize_solver::gpu::CudaAshmaize;

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * MB;

fn main() {
    #[cfg(not(feature = "cuda"))]
    {
        eprintln!("This example requires the 'cuda' feature");
        eprintln!("Run with: cargo run --example multi_gpu_batchtuned --features cuda --release");
        std::process::exit(1);
    }

    #[cfg(feature = "cuda")]
    {
        println!("=== Multi-GPU Optimized: Batch Size Tuning ===\n");

        // Initialize CUDA
        println!("Initializing CUDA...");
        let _init = match CudaAshmaize::new() {
            Ok(_) => {},
            Err(e) => {
                eprintln!("Failed to initialize CUDA: {:?}", e);
                std::process::exit(1);
            }
        };

        // Detect GPUs
        let gpu_count = match CudaAshmaize::get_device_count() {
            Ok(count) => {
                println!("Detected {} GPU(s)\n", count);
                count
            }
            Err(e) => {
                eprintln!("Failed to detect GPUs: {:?}", e);
                std::process::exit(1);
            }
        };

        // Setup ROM
        let rom = Arc::new(Rom::new(
            b"0",
            RomGenerationType::TwoStep {
                pre_size: 16 * MB,
                mixing_numbers: 4,
            },
            1 * GB,
        ));

        // Test different batch sizes
        let batch_sizes = vec![2048, 4096, 8192, 16384];
        
        println!("Testing batch sizes: {:?}\n", batch_sizes);

        for batch_per_gpu in batch_sizes {
            println!("=== Testing batch_per_gpu = {} ===", batch_per_gpu);
            
            let total_batch_size = batch_per_gpu * gpu_count;
            let duration_secs = 15; // Shorter tests

            // Generate test salts
            let mut all_salts = Vec::new();
            for i in 0..total_batch_size {
                all_salts.push(format!("test_{}", i).into_bytes());
            }
            let all_salts = Arc::new(all_salts);

            // Initialize GPUs
            let mut gpus = Vec::new();
            for gpu_id in 0..gpu_count {
                match CudaAshmaize::new_with_device(gpu_id) {
                    Ok(c) => gpus.push(Arc::new(c)),
                    Err(e) => {
                        eprintln!("GPU {} failed: {:?}", gpu_id, e);
                        std::process::exit(1);
                    }
                }
            }

            let start_time = Instant::now();
            let total_hashes = Arc::new(std::sync::atomic::AtomicU64::new(0));
            let running = Arc::new(std::sync::atomic::AtomicBool::new(true));

            let (result_tx, result_rx) = mpsc::channel::<(usize, Result<Vec<[u8; 64]>, String>)>();

            // Spawn GPU workers
            let mut handles = Vec::new();
            for (gpu_id, cuda) in gpus.iter().enumerate() {
                let cuda = Arc::clone(cuda);
                let rom = Arc::clone(&rom);
                let all_salts = Arc::clone(&all_salts);
                let result_tx = result_tx.clone();
                let running = Arc::clone(&running);

                let handle = thread::spawn(move || {
                    let start_idx = gpu_id * batch_per_gpu;
                    let end_idx = std::cmp::min(start_idx + batch_per_gpu, total_batch_size);
                    
                    while running.load(std::sync::atomic::Ordering::Relaxed) {
                        let gpu_salts: Vec<Vec<u8>> = all_salts[start_idx..end_idx]
                            .iter()
                            .cloned()
                            .collect();

                        let salt_refs: Vec<&[u8]> = gpu_salts.iter().map(|s| s.as_slice()).collect();
                        let result = cuda.hash_parallel(&salt_refs, &rom, 8, 256)
                            .map_err(|e| format!("{:?}", e));
                        
                        result_tx.send((gpu_id, result)).ok();
                    }
                });
                handles.push(handle);
            }
            drop(result_tx);

            // Result collector
            let total_hashes_clone = Arc::clone(&total_hashes);
            let collector = thread::spawn(move || {
                while let Ok((gpu_id, result)) = result_rx.recv() {
                    if let Err(e) = result {
                        eprintln!("GPU {} error: {}", gpu_id, e);
                    } else {
                        total_hashes_clone.fetch_add(batch_per_gpu as u64, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            });

            // Wait for duration
            thread::sleep(std::time::Duration::from_secs(duration_secs));

            // Stop workers
            running.store(false, std::sync::atomic::Ordering::Relaxed);
            for handle in handles {
                handle.join().ok();
            }
            collector.join().ok();

            let elapsed = start_time.elapsed();
            let final_hashes = total_hashes.load(std::sync::atomic::Ordering::Relaxed);
            let hashrate = final_hashes as f64 / elapsed.as_secs_f64();

            println!("  Hashrate: {:.2} H/s", hashrate);
            println!("  Per GPU: {:.2} H/s\n", hashrate / gpu_count as f64);
        }

        println!("=== Batch Size Tuning Complete ===");
    }
}

