// Comprehensive Batch Size Sweep
// Tests increasingly larger batch sizes to find performance plateau

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
        eprintln!("Run with: cargo run --example multi_gpu_batch_sweep --features cuda --release");
        std::process::exit(1);
    }

    #[cfg(feature = "cuda")]
    {
        println!("=== Comprehensive Batch Size Sweep ===\n");

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

        // Comprehensive batch size sweep
        let batch_sizes = vec![
            1024, 2048, 4096, 8192, 16384, 32768, 65536, 131072, 262144, 524288
        ];
        
        println!("Testing batch sizes (per GPU): {:?}\n", batch_sizes);
        println!("{:<12} {:<15} {:<15} {:<10}", "Batch/GPU", "Total Batch", "Hashrate", "Per GPU");
        println!("{}", "-".repeat(55));

        let mut results = Vec::new();

        for batch_per_gpu in batch_sizes {
            let total_batch_size = batch_per_gpu * gpu_count;
            let duration_secs = 15;

            // Generate test salts
            println!("Generating {} salts...", total_batch_size);
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
            let per_gpu = hashrate / gpu_count as f64;

            println!("{:<12} {:<15} {:<15.2} {:<10.2}", 
                batch_per_gpu, 
                total_batch_size, 
                hashrate,
                per_gpu
            );

            results.push((batch_per_gpu, hashrate, per_gpu));
        }

        println!("\n{}", "=".repeat(55));
        println!("=== Analysis ===\n");

        // Find peak performance
        let max_hashrate = results.iter().map(|(_, h, _)| *h).fold(0.0f64, f64::max);
        let (best_batch, _, best_per_gpu) = results.iter()
            .find(|(_, h, _)| *h == max_hashrate)
            .unwrap();

        println!("Peak Performance:");
        println!("  Batch per GPU: {}", best_batch);
        println!("  Aggregate: {:.2} H/s", max_hashrate);
        println!("  Per GPU: {:.2} H/s\n", best_per_gpu);

        // Calculate improvement from baseline
        let baseline = results[0].1; // First batch size
        let improvement = ((max_hashrate - baseline) / baseline) * 100.0;
        println!("Improvement over smallest batch: +{:.1}%", improvement);

        // Find plateau point (where improvement < 5%)
        let mut plateau_batch = *best_batch;
        for i in 1..results.len() {
            let prev_hashrate = results[i-1].1;
            let curr_hashrate = results[i].1;
            let improvement = ((curr_hashrate - prev_hashrate) / prev_hashrate) * 100.0;
            
            if improvement < 5.0 && curr_hashrate > max_hashrate * 0.95 {
                plateau_batch = results[i].0;
                println!("\nPerformance plateau at batch_per_gpu = {} (improvement < 5%)", plateau_batch);
                break;
            }
        }

        println!("\n=== Recommendation ===");
        println!("Optimal batch_per_gpu: {} - {}", plateau_batch, best_batch);
        println!("Use higher values for maximum throughput");
        println!("Use lower values in plateau range for lower latency");
    }
}

