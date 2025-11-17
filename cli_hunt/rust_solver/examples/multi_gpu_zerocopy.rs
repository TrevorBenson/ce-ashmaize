// Phase 5: Zero-Copy Data
// This version uses Arc to share salt data across threads without cloning

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
        eprintln!("Run with: cargo run --example multi_gpu_zerocopy --features cuda --release");
        std::process::exit(1);
    }

    #[cfg(feature = "cuda")]
    {
        println!("=== Multi-GPU Optimized: Zero-Copy + Async ===\n");

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

        let total_batch_size = 8192;
        let batch_per_gpu = total_batch_size / gpu_count;
        let duration_secs = 30;

        println!("Test Configuration:");
        println!("  GPUs: {}", gpu_count);
        println!("  Total batch: {}", total_batch_size);
        println!("  Per GPU: {}", batch_per_gpu);
        println!("  Duration: {}s\n", duration_secs);

        // Generate test salts - ONCE, shared via Arc (zero-copy)
        let mut all_salts = Vec::new();
        for i in 0..total_batch_size {
            all_salts.push(format!("test_{}", i).into_bytes());
        }
        let all_salts = Arc::new(all_salts);

        // Pre-compute salt ranges for each GPU (zero allocation in hot path)
        let salt_ranges: Vec<(usize, usize)> = (0..gpu_count)
            .map(|gpu_id| {
                let start = gpu_id * batch_per_gpu;
                let end = std::cmp::min(start + batch_per_gpu, total_batch_size);
                (start, end)
            })
            .collect();
        let salt_ranges = Arc::new(salt_ranges);

        // Initialize GPUs
        let mut gpus = Vec::new();
        println!("Initializing GPUs...");
        for gpu_id in 0..gpu_count {
            match CudaAshmaize::new_with_device(gpu_id) {
                Ok(c) => {
                    println!("  GPU {}: {}", gpu_id, c.get_device_info().unwrap_or_else(|_| format!("GPU {}", gpu_id)));
                    gpus.push(Arc::new(c));
                }
                Err(e) => {
                    eprintln!("  GPU {}: Failed - {:?}", gpu_id, e);
                    std::process::exit(1);
                }
            }
        }

        println!("\nRunning benchmark with zero-copy data sharing...\n");
        
        let start_time = Instant::now();
        let total_hashes = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let running = Arc::new(std::sync::atomic::AtomicBool::new(true));

        // Result channel
        let (result_tx, result_rx) = mpsc::channel::<(usize, Result<Vec<[u8; 64]>, String>)>();

        // Spawn GPU workers with zero-copy shared data
        let mut handles = Vec::new();
        for (gpu_id, cuda) in gpus.iter().enumerate() {
            let cuda = Arc::clone(cuda);
            let rom = Arc::clone(&rom);
            let all_salts = Arc::clone(&all_salts);  // Zero-copy: just increment Arc refcount
            let salt_ranges = Arc::clone(&salt_ranges);
            let result_tx = result_tx.clone();
            let running = Arc::clone(&running);

            let handle = thread::spawn(move || {
                let (start_idx, end_idx) = salt_ranges[gpu_id];
                
                while running.load(std::sync::atomic::Ordering::Relaxed) {
                    // Zero-copy: just create slice references
                    let salt_refs: Vec<&[u8]> = all_salts[start_idx..end_idx]
                        .iter()
                        .map(|s| s.as_slice())
                        .collect();

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

        // Progress reporter
        let mut last_report = Instant::now();
        while start_time.elapsed().as_secs() < duration_secs {
            thread::sleep(std::time::Duration::from_millis(500));
            
            if last_report.elapsed().as_millis() >= 1000 {
                let elapsed = start_time.elapsed().as_secs_f64();
                let hashes = total_hashes.load(std::sync::atomic::Ordering::Relaxed);
                let hashrate = hashes as f64 / elapsed;
                print!("\rHashrate: {:.2} H/s ({}s elapsed)", hashrate, elapsed as u64);
                use std::io::Write;
                std::io::stdout().flush().unwrap();
                last_report = Instant::now();
            }
        }

        // Stop workers
        running.store(false, std::sync::atomic::Ordering::Relaxed);
        for handle in handles {
            handle.join().ok();
        }
        collector.join().ok();

        let elapsed = start_time.elapsed();
        let final_hashes = total_hashes.load(std::sync::atomic::Ordering::Relaxed);
        let hashrate = final_hashes as f64 / elapsed.as_secs_f64();

        println!("\n\n=== Results ===");
        println!("Total hashes: {}", final_hashes);
        println!("Duration: {:.2}s", elapsed.as_secs_f64());
        println!("Hashrate: {:.2} H/s", hashrate);
        println!("Per GPU: {:.2} H/s", hashrate / gpu_count as f64);
        
        println!("\nOptimization: Zero-Copy Data + Async Execution");
        println!("Status: COMPLETE");
    }
}

