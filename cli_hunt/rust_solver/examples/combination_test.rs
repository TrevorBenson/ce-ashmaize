// Systematic Combination Testing
// Tests all combinations of optimizations with optimal batch size

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
        std::process::exit(1);
    }

    #[cfg(feature = "cuda")]
    {
        println!("=== Systematic Combination Testing ===\n");

        // Initialize CUDA
        let _init = CudaAshmaize::new().expect("CUDA init failed");
        let gpu_count = CudaAshmaize::get_device_count().expect("Failed to get GPU count");
        
        println!("Detected {} GPU(s)\n", gpu_count);

        // Setup ROM
        let rom = Arc::new(Rom::new(
            b"0",
            RomGenerationType::TwoStep {
                pre_size: 16 * MB,
                mixing_numbers: 4,
            },
            1 * GB,
        ));

        // Optimal batch size from previous tests
        let batch_per_gpu = 131_072;
        let duration = 20; // seconds

        println!("Configuration:");
        println!("  Batch per GPU: {}", batch_per_gpu);
        println!("  Test duration: {}s\n", duration);
        println!("{}", "=".repeat(70));

        // Test 1: Synchronous + Clone (baseline with large batch)
        println!("\nTest 1: Synchronous + Clone (baseline, large batch)");
        let r1 = test_sync_clone(gpu_count, batch_per_gpu, Arc::clone(&rom), duration);
        println!("  Result: {:.2} H/s\n", r1);

        // Test 2: Async + Clone (current best known)
        println!("Test 2: Async + Clone");
        let r2 = test_async_clone(gpu_count, batch_per_gpu, Arc::clone(&rom), duration);
        println!("  Result: {:.2} H/s\n", r2);

        // Test 3: Async + Zero-Copy (Arc shared)
        println!("Test 3: Async + Zero-Copy");
        let r3 = test_async_zerocopy(gpu_count, batch_per_gpu, Arc::clone(&rom), duration);
        println!("  Result: {:.2} H/s\n", r3);

        // Test 4: Persistent Workers + Clone
        println!("Test 4: Persistent Workers + Clone");
        let r4 = test_persistent_clone(gpu_count, batch_per_gpu, Arc::clone(&rom), duration);
        println!("  Result: {:.2} H/s\n", r4);

        println!("{}", "=".repeat(70));
        println!("\n=== Results Summary ===\n");
        
        let results = vec![
            ("Sync + Clone", r1),
            ("Async + Clone", r2),
            ("Async + Zero-Copy", r3),
            ("Persistent + Clone", r4),
        ];

        let max_hashrate = results.iter().map(|(_, h)| *h).fold(0.0f64, f64::max);
        
        for (name, hashrate) in &results {
            let pct = (hashrate / max_hashrate) * 100.0;
            let marker = if *hashrate == max_hashrate { " ✅ BEST" } else { "" };
            println!("{:<25} {:>12.2} H/s  ({:>5.1}%){}",
                name, hashrate, pct, marker);
        }

        let (best_name, best_hashrate) = results.iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();

        println!("\n🎯 Optimal Configuration: {}", best_name);
        println!("   Performance: {:.2} H/s", best_hashrate);
        println!("   Per GPU: {:.2} H/s", best_hashrate / gpu_count as f64);
    }
}

#[cfg(feature = "cuda")]
fn test_sync_clone(gpu_count: usize, batch_per_gpu: usize, rom: Arc<Rom>, duration: u64) -> f64 {
    let total_batch = batch_per_gpu * gpu_count;
    
    let mut all_salts = Vec::new();
    for i in 0..total_batch {
        all_salts.push(format!("test_{}", i).into_bytes());
    }
    
    let mut gpus = Vec::new();
    for gpu_id in 0..gpu_count {
        gpus.push(Arc::new(CudaAshmaize::new_with_device(gpu_id).unwrap()));
    }

    let start = Instant::now();
    let mut total_hashes = 0u64;

    while start.elapsed().as_secs() < duration {
        let mut handles = Vec::new();
        
        for (gpu_id, cuda) in gpus.iter().enumerate() {
            let cuda = Arc::clone(cuda);
            let rom = Arc::clone(&rom);
            let start_idx = gpu_id * batch_per_gpu;
            let end_idx = start_idx + batch_per_gpu;
            let gpu_salts: Vec<Vec<u8>> = all_salts[start_idx..end_idx].iter().cloned().collect();
            
            let handle = thread::spawn(move || {
                let salt_refs: Vec<&[u8]> = gpu_salts.iter().map(|s| s.as_slice()).collect();
                cuda.hash_parallel(&salt_refs, &rom, 8, 256)
            });
            handles.push(handle);
        }
        
        // SYNCHRONOUS: Wait for all GPUs
        for handle in handles {
            handle.join().ok();
        }
        
        total_hashes += total_batch as u64;
    }

    total_hashes as f64 / start.elapsed().as_secs_f64()
}

#[cfg(feature = "cuda")]
fn test_async_clone(gpu_count: usize, batch_per_gpu: usize, rom: Arc<Rom>, duration: u64) -> f64 {
    let total_batch = batch_per_gpu * gpu_count;
    
    let mut all_salts = Vec::new();
    for i in 0..total_batch {
        all_salts.push(format!("test_{}", i).into_bytes());
    }
    let all_salts = Arc::new(all_salts);
    
    let mut gpus = Vec::new();
    for gpu_id in 0..gpu_count {
        gpus.push(Arc::new(CudaAshmaize::new_with_device(gpu_id).unwrap()));
    }

    let start = Instant::now();
    let total_hashes = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let running = Arc::new(std::sync::atomic::AtomicBool::new(true));

    let (result_tx, result_rx) = mpsc::channel();

    let mut handles = Vec::new();
    for (gpu_id, cuda) in gpus.iter().enumerate() {
        let cuda = Arc::clone(cuda);
        let rom = Arc::clone(&rom);
        let all_salts = Arc::clone(&all_salts);
        let result_tx = result_tx.clone();
        let running = Arc::clone(&running);

        let handle = thread::spawn(move || {
            let start_idx = gpu_id * batch_per_gpu;
            let end_idx = start_idx + batch_per_gpu;
            
            while running.load(std::sync::atomic::Ordering::Relaxed) {
                let gpu_salts: Vec<Vec<u8>> = all_salts[start_idx..end_idx].iter().cloned().collect();
                let salt_refs: Vec<&[u8]> = gpu_salts.iter().map(|s| s.as_slice()).collect();
                let result = cuda.hash_parallel(&salt_refs, &rom, 8, 256);
                result_tx.send((gpu_id, result)).ok();
            }
        });
        handles.push(handle);
    }
    drop(result_tx);

    let total_hashes_clone = Arc::clone(&total_hashes);
    let collector = thread::spawn(move || {
        while let Ok((_, result)) = result_rx.recv() {
            if result.is_ok() {
                total_hashes_clone.fetch_add(batch_per_gpu as u64, std::sync::atomic::Ordering::Relaxed);
            }
        }
    });

    thread::sleep(std::time::Duration::from_secs(duration));
    running.store(false, std::sync::atomic::Ordering::Relaxed);
    
    for handle in handles {
        handle.join().ok();
    }
    collector.join().ok();

    total_hashes.load(std::sync::atomic::Ordering::Relaxed) as f64 / start.elapsed().as_secs_f64()
}

#[cfg(feature = "cuda")]
fn test_async_zerocopy(gpu_count: usize, batch_per_gpu: usize, rom: Arc<Rom>, duration: u64) -> f64 {
    let total_batch = batch_per_gpu * gpu_count;
    
    let mut all_salts = Vec::new();
    for i in 0..total_batch {
        all_salts.push(format!("test_{}", i).into_bytes());
    }
    let all_salts = Arc::new(all_salts);
    
    let mut gpus = Vec::new();
    for gpu_id in 0..gpu_count {
        gpus.push(Arc::new(CudaAshmaize::new_with_device(gpu_id).unwrap()));
    }

    let start = Instant::now();
    let total_hashes = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let running = Arc::new(std::sync::atomic::AtomicBool::new(true));

    let (result_tx, result_rx) = mpsc::channel();

    let mut handles = Vec::new();
    for (gpu_id, cuda) in gpus.iter().enumerate() {
        let cuda = Arc::clone(cuda);
        let rom = Arc::clone(&rom);
        let all_salts = Arc::clone(&all_salts);
        let result_tx = result_tx.clone();
        let running = Arc::clone(&running);

        let handle = thread::spawn(move || {
            let start_idx = gpu_id * batch_per_gpu;
            let end_idx = start_idx + batch_per_gpu;
            
            while running.load(std::sync::atomic::Ordering::Relaxed) {
                // Zero-copy: just create references, no clone
                let salt_refs: Vec<&[u8]> = all_salts[start_idx..end_idx]
                    .iter()
                    .map(|s| s.as_slice())
                    .collect();
                let result = cuda.hash_parallel(&salt_refs, &rom, 8, 256);
                result_tx.send((gpu_id, result)).ok();
            }
        });
        handles.push(handle);
    }
    drop(result_tx);

    let total_hashes_clone = Arc::clone(&total_hashes);
    let collector = thread::spawn(move || {
        while let Ok((_, result)) = result_rx.recv() {
            if result.is_ok() {
                total_hashes_clone.fetch_add(batch_per_gpu as u64, std::sync::atomic::Ordering::Relaxed);
            }
        }
    });

    thread::sleep(std::time::Duration::from_secs(duration));
    running.store(false, std::sync::atomic::Ordering::Relaxed);
    
    for handle in handles {
        handle.join().ok();
    }
    collector.join().ok();

    total_hashes.load(std::sync::atomic::Ordering::Relaxed) as f64 / start.elapsed().as_secs_f64()
}

#[cfg(feature = "cuda")]
fn test_persistent_clone(gpu_count: usize, batch_per_gpu: usize, rom: Arc<Rom>, duration: u64) -> f64 {
    let total_batch = batch_per_gpu * gpu_count;
    
    let mut all_salts = Vec::new();
    for i in 0..total_batch {
        all_salts.push(format!("test_{}", i).into_bytes());
    }
    let all_salts = Arc::new(all_salts);
    
    let mut gpus = Vec::new();
    for gpu_id in 0..gpu_count {
        gpus.push(Arc::new(CudaAshmaize::new_with_device(gpu_id).unwrap()));
    }

    struct WorkItem {
        salts: Vec<Vec<u8>>,
        rom: Arc<Rom>,
    }

    let (work_tx, work_rx) = mpsc::channel::<WorkItem>();
    let work_rx = Arc::new(std::sync::Mutex::new(work_rx));
    let (result_tx, result_rx) = mpsc::channel::<(usize, Result<Vec<[u8; 64]>, String>)>();

    // Spawn persistent workers
    let mut handles = Vec::new();
    for (gpu_id, cuda) in gpus.iter().enumerate() {
        let cuda = Arc::clone(cuda);
        let work_rx = Arc::clone(&work_rx);
        let result_tx = result_tx.clone();

        let handle = thread::spawn(move || {
            loop {
                let work = {
                    let rx = work_rx.lock().unwrap();
                    rx.recv()
                };

                match work {
                    Ok(work_item) => {
                        let salt_refs: Vec<&[u8]> = work_item.salts.iter().map(|s| s.as_slice()).collect();
                        let result = cuda.hash_parallel(&salt_refs, &work_item.rom, 8, 256)
                            .map_err(|e| format!("{:?}", e));
                        result_tx.send((gpu_id, result)).ok();
                    }
                    Err(_) => break,
                }
            }
        });
        handles.push(handle);
    }
    drop(result_tx);

    let start = Instant::now();
    let total_hashes = Arc::new(std::sync::atomic::AtomicU64::new(0));

    let total_hashes_clone = Arc::clone(&total_hashes);
    let collector = thread::spawn(move || {
        while let Ok((_, result)) = result_rx.recv() {
            if result.is_ok() {
                total_hashes_clone.fetch_add(batch_per_gpu as u64, std::sync::atomic::Ordering::Relaxed);
            }
        }
    });

    while start.elapsed().as_secs() < duration {
        for gpu_id in 0..gpu_count {
            let start_idx = gpu_id * batch_per_gpu;
            let end_idx = start_idx + batch_per_gpu;
            let gpu_salts: Vec<Vec<u8>> = all_salts[start_idx..end_idx].iter().cloned().collect();
            
            work_tx.send(WorkItem {
                salts: gpu_salts,
                rom: Arc::clone(&rom),
            }).ok();
        }
    }

    drop(work_tx);
    for handle in handles {
        handle.join().ok();
    }
    collector.join().ok();

    total_hashes.load(std::sync::atomic::Ordering::Relaxed) as f64 / start.elapsed().as_secs_f64()
}

