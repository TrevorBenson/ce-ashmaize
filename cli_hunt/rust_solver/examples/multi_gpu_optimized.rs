// Phase 2: Persistent GPU Workers
// This version eliminates thread spawn overhead by using persistent workers

use ashmaize::{Rom, RomGenerationType};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Instant;

#[cfg(feature = "cuda")]
use ashmaize_solver::gpu::CudaAshmaize;

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * MB;

#[cfg(feature = "cuda")]
struct WorkItem {
    salts: Vec<Vec<u8>>,
    rom: Arc<Rom>,
}

#[cfg(feature = "cuda")]
struct ResultItem {
    gpu_id: usize,
    result: Result<Vec<[u8; 64]>, String>,
}

fn main() {
    #[cfg(not(feature = "cuda"))]
    {
        eprintln!("This example requires the 'cuda' feature");
        eprintln!("Run with: cargo run --example multi_gpu_optimized --features cuda --release");
        std::process::exit(1);
    }

    #[cfg(feature = "cuda")]
    {
        println!("=== Multi-GPU Optimized: Persistent Workers ===\n");

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

        // Generate test salts
        let mut all_salts = Vec::new();
        for i in 0..total_batch_size {
            all_salts.push(format!("test_{}", i).into_bytes());
        }
        let all_salts = Arc::new(all_salts);

        // Create channels
        let (work_tx, work_rx) = mpsc::channel::<WorkItem>();
        let work_rx = Arc::new(std::sync::Mutex::new(work_rx));
        
        let (result_tx, result_rx) = mpsc::channel::<ResultItem>();

        // Spawn persistent worker threads
        println!("Starting persistent GPU workers...");
        for gpu_id in 0..gpu_count {
            let cuda = match CudaAshmaize::new_with_device(gpu_id) {
                Ok(c) => {
                    println!("  GPU {}: {}", gpu_id, c.get_device_info().unwrap_or_else(|_| format!("GPU {}", gpu_id)));
                    Arc::new(c)
                }
                Err(e) => {
                    eprintln!("  GPU {}: Failed - {:?}", gpu_id, e);
                    continue;
                }
            };

            let work_rx = Arc::clone(&work_rx);
            let result_tx = result_tx.clone();

            thread::spawn(move || {
                loop {
                    // Get work from channel
                    let work = {
                        let rx = work_rx.lock().unwrap();
                        rx.recv()
                    };

                    match work {
                        Ok(work_item) => {
                            let salt_refs: Vec<&[u8]> = work_item.salts.iter().map(|s| s.as_slice()).collect();
                            let result = cuda.hash_parallel(&salt_refs, &work_item.rom, 8, 256)
                                .map_err(|e| format!("{:?}", e));
                            
                            result_tx.send(ResultItem { gpu_id, result }).ok();
                        }
                        Err(_) => break, // Channel closed, exit worker
                    }
                }
            });
        }
        drop(result_tx); // Drop the original sender

        println!("\nRunning benchmark with persistent workers...\n");
        
        let start_time = Instant::now();
        let mut total_hashes = 0u64;
        let mut iteration = 0;

        while start_time.elapsed().as_secs() < duration_secs {
            // Distribute work to all GPUs
            for gpu_id in 0..gpu_count {
                let start_idx = gpu_id * batch_per_gpu;
                let end_idx = std::cmp::min(start_idx + batch_per_gpu, total_batch_size);
                
                let gpu_salts: Vec<Vec<u8>> = all_salts[start_idx..end_idx]
                    .iter()
                    .cloned()
                    .collect();

                work_tx.send(WorkItem {
                    salts: gpu_salts,
                    rom: Arc::clone(&rom),
                }).ok();
            }

            // Collect results from all GPUs
            for _ in 0..gpu_count {
                match result_rx.recv() {
                    Ok(result_item) => {
                        if result_item.result.is_err() {
                            eprintln!("GPU {} error: {:?}", result_item.gpu_id, result_item.result.err());
                        }
                    }
                    Err(_) => break,
                }
            }

            total_hashes += total_batch_size as u64;
            iteration += 1;

            if iteration % 10 == 0 {
                let elapsed = start_time.elapsed().as_secs_f64();
                let hashrate = total_hashes as f64 / elapsed;
                print!("\rIteration {}: {:.2} H/s", iteration, hashrate);
                use std::io::Write;
                std::io::stdout().flush().unwrap();
            }
        }

        drop(work_tx); // Signal workers to exit

        let elapsed = start_time.elapsed();
        let hashrate = total_hashes as f64 / elapsed.as_secs_f64();

        println!("\n\n=== Results ===");
        println!("Total hashes: {}", total_hashes);
        println!("Duration: {:.2}s", elapsed.as_secs_f64());
        println!("Hashrate: {:.2} H/s", hashrate);
        println!("Per GPU: {:.2} H/s", hashrate / gpu_count as f64);
        
        println!("\nOptimization: Persistent GPU Workers");
        println!("Status: COMPLETE");
    }
}

