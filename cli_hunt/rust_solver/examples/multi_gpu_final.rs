// Final Optimized: Async + Large Batches
// Combines async execution with optimal batch size

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
        eprintln!("Run with: cargo run --example multi_gpu_final --features cuda --release");
        std::process::exit(1);
    }

    #[cfg(feature = "cuda")]
    {
        println!("=== Multi-GPU Final Optimization ===\n");

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

        // Optimal batch size from tuning
        let batch_per_gpu = 16384;
        let total_batch_size = batch_per_gpu * gpu_count;
        let duration_secs = 30;

        println!("Configuration:");
        println!("  GPUs: {}", gpu_count);
        println!("  Batch per GPU: {} (optimal)", batch_per_gpu);
        println!("  Total batch: {}", total_batch_size);
        println!("  Duration: {}s\n", duration_secs);

        // Generate test salts
        let mut all_salts = Vec::new();
        for i in 0..total_batch_size {
            all_salts.push(format!("test_{}", i).into_bytes());
        }
        let all_salts = Arc::new(all_salts);

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

        println!("\nRunning final optimized benchmark...\n");
        
        let start_time = Instant::now();
        let total_hashes = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let running = Arc::new(std::sync::atomic::AtomicBool::new(true));

        // Result channel
        let (result_tx, result_rx) = mpsc::channel::<(usize, Result<Vec<[u8; 64]>, String>)>();

        // Spawn GPU workers with async execution and large batches
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

        println!("\n\n=== FINAL RESULTS ===");
        println!("Total hashes: {}", final_hashes);
        println!("Duration: {:.2}s", elapsed.as_secs_f64());
        println!("Aggregate hashrate: {:.2} H/s", hashrate);
        println!("Per GPU: {:.2} H/s", hashrate / gpu_count as f64);
        
        println!("\n=== Performance Comparison ===");
        let baseline = 21605.0;
        let multi_process = 54074.0;
        let target_80 = 43000.0;
        let target_90 = 48600.0;
        
        println!("vs Baseline (small batch):  {:.1}x", hashrate / baseline);
        println!("vs Multi-Process Target:    {:.1}x", hashrate / multi_process);
        println!("vs 80% Target (43k H/s):    {:.1}%", (hashrate / target_80) * 100.0);
        println!("vs 90% Target (48.6k H/s):  {:.1}%", (hashrate / target_90) * 100.0);
        
        if hashrate >= target_90 {
            println!("\n✅ SUCCESS: Exceeded 90% efficiency target!");
        } else if hashrate >= target_80 {
            println!("\n✅ SUCCESS: Achieved 80-90% efficiency target!");
        } else {
            println!("\n⚠️  Below 80% target, but massive improvement over baseline");
        }
    }
}

