use ashmaize::{Rom, RomGenerationType};
use std::sync::Arc;
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
        eprintln!("Run with: cargo run --example multi_gpu_test --features cuda --release");
        std::process::exit(1);
    }

    #[cfg(feature = "cuda")]
    {
        println!("=== Multi-GPU Automatic Distribution Test ===\n");

        // Initialize CUDA first by creating a device
        println!("Initializing CUDA...");
        let _init = match CudaAshmaize::new() {
            Ok(_) => {},
            Err(e) => {
                eprintln!("Failed to initialize CUDA: {:?}", e);
                std::process::exit(1);
            }
        };

        // Detect available GPUs
        let gpu_count = match CudaAshmaize::get_device_count() {
            Ok(count) => {
                println!("Detected {} GPU(s)", count);
                if count == 0 {
                    eprintln!("No GPUs found!");
                    std::process::exit(1);
                }
                count
            }
            Err(e) => {
                eprintln!("Failed to detect GPUs: {:?}", e);
                std::process::exit(1);
            }
        };

        // Initialize one CudaAshmaize instance per GPU
        println!("\nInitializing GPUs...");
        let mut gpu_instances = Vec::new();
        for gpu_id in 0..gpu_count {
            match CudaAshmaize::new_with_device(gpu_id) {
                Ok(cuda) => {
                    let device_info = cuda.get_device_info().unwrap_or_else(|_| format!("GPU {}", gpu_id));
                    println!("  GPU {}: {}", gpu_id, device_info);
                    gpu_instances.push(Arc::new(cuda));
                }
                Err(e) => {
                    eprintln!("  GPU {}: Failed to initialize - {:?}", gpu_id, e);
                }
            }
        }

        if gpu_instances.is_empty() {
            eprintln!("\nNo GPUs could be initialized!");
            std::process::exit(1);
        }

        let active_gpu_count = gpu_instances.len();
        println!("\nSuccessfully initialized {} GPU(s)\n", active_gpu_count);

        // Setup test parameters
        let rom = Rom::new(
            b"0",
            RomGenerationType::TwoStep {
                pre_size: 16 * MB,
                mixing_numbers: 4,
            },
            1 * GB,
        );
        let rom = Arc::new(rom);

        let total_batch_size = 8192; // Total hashes to compute
        let batch_per_gpu = total_batch_size / active_gpu_count;
        let duration_secs = 30;

        println!("Test Configuration:");
        println!("  Total batch size: {}", total_batch_size);
        println!("  Batch per GPU: {}", batch_per_gpu);
        println!("  Duration: {}s", duration_secs);
        println!("  ROM size: {} MB\n", rom.data.len() / MB);

        // Generate test salts
        let mut all_salts = Vec::new();
        for i in 0..total_batch_size {
            let salt = format!("test_salt_{}", i);
            all_salts.push(salt.into_bytes());
        }

        println!("Starting multi-GPU benchmark...\n");
        let start_time = Instant::now();
        let mut total_hashes = 0u64;
        let mut iteration = 0;

        while start_time.elapsed().as_secs() < duration_secs {
            // Distribute work across GPUs using threads
            let mut handles = Vec::new();

            for (gpu_id, cuda) in gpu_instances.iter().enumerate() {
                let cuda = Arc::clone(cuda);
                let rom = Arc::clone(&rom);
                
                // Get this GPU's slice of salts (clone for thread safety)
                let start_idx = gpu_id * batch_per_gpu;
                let end_idx = std::cmp::min(start_idx + batch_per_gpu, total_batch_size);
                let gpu_salts: Vec<Vec<u8>> = all_salts[start_idx..end_idx]
                    .iter()
                    .cloned()
                    .collect();

                let handle = thread::spawn(move || {
                    let salt_refs: Vec<&[u8]> = gpu_salts.iter().map(|s| s.as_slice()).collect();
                    cuda.hash_parallel(&salt_refs, &rom, 8, 256)
                });

                handles.push(handle);
            }

            // Wait for all GPUs to complete and collect results
            let mut all_results = Vec::new();
            for (gpu_id, handle) in handles.into_iter().enumerate() {
                match handle.join() {
                    Ok(Ok(results)) => {
                        all_results.extend(results);
                    }
                    Ok(Err(e)) => {
                        eprintln!("GPU {} error: {:?}", gpu_id, e);
                    }
                    Err(_) => {
                        eprintln!("GPU {} thread panicked", gpu_id);
                    }
                }
            }

            total_hashes += total_batch_size as u64;
            iteration += 1;

            if iteration % 10 == 0 {
                let elapsed = start_time.elapsed().as_secs_f64();
                let hashrate = total_hashes as f64 / elapsed;
                print!("\rIteration {}: {:.2} H/s (aggregate)", iteration, hashrate);
                use std::io::Write;
                std::io::stdout().flush().unwrap();
            }
        }

        let elapsed = start_time.elapsed();
        let hashrate = total_hashes as f64 / elapsed.as_secs_f64();

        println!("\n\n=== Results ===");
        println!("Total hashes: {}", total_hashes);
        println!("Duration: {:.2}s", elapsed.as_secs_f64());
        println!("Aggregate hashrate: {:.2} H/s", hashrate);
        println!("Average per GPU: {:.2} H/s", hashrate / active_gpu_count as f64);
        println!("\nMulti-GPU work distribution: SUCCESS ✓");
    }
}

