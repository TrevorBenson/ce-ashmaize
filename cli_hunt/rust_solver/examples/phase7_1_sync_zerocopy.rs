// Phase 7 Test 1: Sync + Zero-Copy + 131k batch
// Complete the 2x2 matrix at optimal batch size
// Test Duration: 30s

use ashmaize::{Rom, RomGenerationType};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::sync::mpsc;
use std::thread;
use std::time::{Instant, Duration};

#[cfg(feature = "cuda")]
use ashmaize_solver::gpu::CudaAshmaize;

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * MB;
const TEST_DURATION: u64 = 30;

fn main() {
    #[cfg(not(feature = "cuda"))]
    {
        eprintln!("This example requires the 'cuda' feature");
        std::process::exit(1);
    }

    #[cfg(feature = "cuda")]
    {
        println!("=== Phase 7 Test 1: Sync + Zero-Copy + 131k ===\n");
        
        let _init = CudaAshmaize::new().expect("CUDA init failed");
        let gpu_count = CudaAshmaize::get_device_count().expect("Failed to get GPU count");
        
        println!("Detected {} GPU(s)\n", gpu_count);
        
        let rom = Arc::new(Rom::new(
            b"0",
            RomGenerationType::TwoStep {
                pre_size: 16 * MB,
                mixing_numbers: 4,
            },
            1 * GB,
        ));
        
        let batch_per_gpu = 131_072;
        
        println!("Configuration:");
        println!("  Execution: SYNC (blocking)");
        println!("  Distribution: ZERO-COPY (Arc-wrapped)");
        println!("  Batch per GPU: {}", batch_per_gpu);
        println!("  Duration: {}s\n", TEST_DURATION);
        
        let hashrate = run_sync_zerocopy(gpu_count, batch_per_gpu, rom, TEST_DURATION);
        
        let async_clone_baseline = 294_000.0;
        let sync_clone_baseline = 241_272.0;
        let async_zerocopy_baseline = 267_660.0;
        
        println!("\n=== Results ===");
        println!("Configuration: Sync + Zero-Copy + 131k");
        println!("Hashrate: {:.2} H/s\n", hashrate);
        
        println!("Comparison:");
        println!("  vs Async+Clone:     {:+.2}% ({:.2} H/s diff)", 
            ((hashrate - async_clone_baseline) / async_clone_baseline) * 100.0,
            hashrate - async_clone_baseline);
        println!("  vs Sync+Clone:      {:+.2}% ({:.2} H/s diff)", 
            ((hashrate - sync_clone_baseline) / sync_clone_baseline) * 100.0,
            hashrate - sync_clone_baseline);
        println!("  vs Async+ZeroCopy:  {:+.2}% ({:.2} H/s diff)", 
            ((hashrate - async_zerocopy_baseline) / async_zerocopy_baseline) * 100.0,
            hashrate - async_zerocopy_baseline);
        
        println!("\n=== Matrix Status ===");
        println!("Async + Clone:     {:.0} H/s ✅", async_clone_baseline);
        println!("Async + Zero-Copy: {:.0} H/s ✅", async_zerocopy_baseline);
        println!("Sync + Clone:      {:.0} H/s ✅", sync_clone_baseline);
        println!("Sync + Zero-Copy:  {:.0} H/s ✅ (just tested)", hashrate);
    }
}

#[cfg(feature = "cuda")]
fn run_sync_zerocopy(
    gpu_count: usize,
    batch_per_gpu: usize,
    rom: Arc<Rom>,
    duration: u64
) -> f64 {
    let total_batch = batch_per_gpu * gpu_count;
    let all_salts: Vec<Vec<u8>> = (0..total_batch)
        .map(|i| format!("test_{}", i).into_bytes())
        .collect();
    let all_salts = Arc::new(all_salts);
    
    let mut gpus = Vec::new();
    for gpu_id in 0..gpu_count {
        gpus.push(Arc::new(CudaAshmaize::new_with_device(gpu_id).unwrap()));
    }
    
    let start_time = Instant::now();
    let total_hashes = Arc::new(AtomicU64::new(0));
    let running = Arc::new(AtomicBool::new(true));
    
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
            
            while running.load(Ordering::Relaxed) {
                // Zero-copy: Create Arc-wrapped references (no data clone)
                let gpu_salts_arc: Arc<Vec<&[u8]>> = Arc::new(
                    all_salts[start_idx..end_idx]
                        .iter()
                        .map(|s| s.as_slice())
                        .collect()
                );
                
                let salt_refs: Vec<&[u8]> = gpu_salts_arc.iter().copied().collect();
                
                let result = cuda.hash_parallel(&salt_refs, &rom, 8, 256)
                    .map_err(|e| format!("{:?}", e));
                
                result_tx.send((gpu_id, result)).ok();
            }
        });
        handles.push(handle);
    }
    drop(result_tx);
    
    let total_hashes_clone = Arc::clone(&total_hashes);
    let collector = thread::spawn(move || {
        while let Ok((_gpu_id, result)) = result_rx.recv() {
            if result.is_ok() {
                total_hashes_clone.fetch_add(batch_per_gpu as u64, Ordering::Relaxed);
            }
        }
    });
    
    thread::sleep(Duration::from_secs(duration));
    
    running.store(false, Ordering::Relaxed);
    for handle in handles {
        handle.join().ok();
    }
    collector.join().ok();
    
    let elapsed = start_time.elapsed();
    let final_hashes = total_hashes.load(Ordering::Relaxed);
    final_hashes as f64 / elapsed.as_secs_f64()
}

// Complete the 2x2 matrix at optimal batch size
// Test Duration: 30s

use ashmaize::{Rom, RomGenerationType};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::sync::mpsc;
use std::thread;
use std::time::{Instant, Duration};

#[cfg(feature = "cuda")]
use ashmaize_solver::gpu::CudaAshmaize;

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * MB;
const TEST_DURATION: u64 = 30;

fn main() {
    #[cfg(not(feature = "cuda"))]
    {
        eprintln!("This example requires the 'cuda' feature");
        std::process::exit(1);
    }

    #[cfg(feature = "cuda")]
    {
        println!("=== Phase 7 Test 1: Sync + Zero-Copy + 131k ===\n");
        
        let _init = CudaAshmaize::new().expect("CUDA init failed");
        let gpu_count = CudaAshmaize::get_device_count().expect("Failed to get GPU count");
        
        println!("Detected {} GPU(s)\n", gpu_count);
        
        let rom = Arc::new(Rom::new(
            b"0",
            RomGenerationType::TwoStep {
                pre_size: 16 * MB,
                mixing_numbers: 4,
            },
            1 * GB,
        ));
        
        let batch_per_gpu = 131_072;
        
        println!("Configuration:");
        println!("  Execution: SYNC (blocking)");
        println!("  Distribution: ZERO-COPY (Arc-wrapped)");
        println!("  Batch per GPU: {}", batch_per_gpu);
        println!("  Duration: {}s\n", TEST_DURATION);
        
        let hashrate = run_sync_zerocopy(gpu_count, batch_per_gpu, rom, TEST_DURATION);
        
        let async_clone_baseline = 294_000.0;
        let sync_clone_baseline = 241_272.0;
        let async_zerocopy_baseline = 267_660.0;
        
        println!("\n=== Results ===");
        println!("Configuration: Sync + Zero-Copy + 131k");
        println!("Hashrate: {:.2} H/s\n", hashrate);
        
        println!("Comparison:");
        println!("  vs Async+Clone:     {:+.2}% ({:.2} H/s diff)", 
            ((hashrate - async_clone_baseline) / async_clone_baseline) * 100.0,
            hashrate - async_clone_baseline);
        println!("  vs Sync+Clone:      {:+.2}% ({:.2} H/s diff)", 
            ((hashrate - sync_clone_baseline) / sync_clone_baseline) * 100.0,
            hashrate - sync_clone_baseline);
        println!("  vs Async+ZeroCopy:  {:+.2}% ({:.2} H/s diff)", 
            ((hashrate - async_zerocopy_baseline) / async_zerocopy_baseline) * 100.0,
            hashrate - async_zerocopy_baseline);
        
        println!("\n=== Matrix Status ===");
        println!("Async + Clone:     {:.0} H/s ✅", async_clone_baseline);
        println!("Async + Zero-Copy: {:.0} H/s ✅", async_zerocopy_baseline);
        println!("Sync + Clone:      {:.0} H/s ✅", sync_clone_baseline);
        println!("Sync + Zero-Copy:  {:.0} H/s ✅ (just tested)", hashrate);
    }
}

#[cfg(feature = "cuda")]
fn run_sync_zerocopy(
    gpu_count: usize,
    batch_per_gpu: usize,
    rom: Arc<Rom>,
    duration: u64
) -> f64 {
    let total_batch = batch_per_gpu * gpu_count;
    let all_salts: Vec<Vec<u8>> = (0..total_batch)
        .map(|i| format!("test_{}", i).into_bytes())
        .collect();
    let all_salts = Arc::new(all_salts);
    
    let mut gpus = Vec::new();
    for gpu_id in 0..gpu_count {
        gpus.push(Arc::new(CudaAshmaize::new_with_device(gpu_id).unwrap()));
    }
    
    let start_time = Instant::now();
    let total_hashes = Arc::new(AtomicU64::new(0));
    let running = Arc::new(AtomicBool::new(true));
    
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
            
            while running.load(Ordering::Relaxed) {
                // Zero-copy: Create Arc-wrapped references (no data clone)
                let gpu_salts_arc: Arc<Vec<&[u8]>> = Arc::new(
                    all_salts[start_idx..end_idx]
                        .iter()
                        .map(|s| s.as_slice())
                        .collect()
                );
                
                let salt_refs: Vec<&[u8]> = gpu_salts_arc.iter().copied().collect();
                
                let result = cuda.hash_parallel(&salt_refs, &rom, 8, 256)
                    .map_err(|e| format!("{:?}", e));
                
                result_tx.send((gpu_id, result)).ok();
            }
        });
        handles.push(handle);
    }
    drop(result_tx);
    
    let total_hashes_clone = Arc::clone(&total_hashes);
    let collector = thread::spawn(move || {
        while let Ok((_gpu_id, result)) = result_rx.recv() {
            if result.is_ok() {
                total_hashes_clone.fetch_add(batch_per_gpu as u64, Ordering::Relaxed);
            }
        }
    });
    
    thread::sleep(Duration::from_secs(duration));
    
    running.store(false, Ordering::Relaxed);
    for handle in handles {
        handle.join().ok();
    }
    collector.join().ok();
    
    let elapsed = start_time.elapsed();
    let final_hashes = total_hashes.load(Ordering::Relaxed);
    final_hashes as f64 / elapsed.as_secs_f64()
}

// Complete the 2x2 matrix at optimal batch size
// Test Duration: 30s

use ashmaize::{Rom, RomGenerationType};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::sync::mpsc;
use std::thread;
use std::time::{Instant, Duration};

#[cfg(feature = "cuda")]
use ashmaize_solver::gpu::CudaAshmaize;

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * MB;
const TEST_DURATION: u64 = 30;

fn main() {
    #[cfg(not(feature = "cuda"))]
    {
        eprintln!("This example requires the 'cuda' feature");
        std::process::exit(1);
    }

    #[cfg(feature = "cuda")]
    {
        println!("=== Phase 7 Test 1: Sync + Zero-Copy + 131k ===\n");
        
        let _init = CudaAshmaize::new().expect("CUDA init failed");
        let gpu_count = CudaAshmaize::get_device_count().expect("Failed to get GPU count");
        
        println!("Detected {} GPU(s)\n", gpu_count);
        
        let rom = Arc::new(Rom::new(
            b"0",
            RomGenerationType::TwoStep {
                pre_size: 16 * MB,
                mixing_numbers: 4,
            },
            1 * GB,
        ));
        
        let batch_per_gpu = 131_072;
        
        println!("Configuration:");
        println!("  Execution: SYNC (blocking)");
        println!("  Distribution: ZERO-COPY (Arc-wrapped)");
        println!("  Batch per GPU: {}", batch_per_gpu);
        println!("  Duration: {}s\n", TEST_DURATION);
        
        let hashrate = run_sync_zerocopy(gpu_count, batch_per_gpu, rom, TEST_DURATION);
        
        let async_clone_baseline = 294_000.0;
        let sync_clone_baseline = 241_272.0;
        let async_zerocopy_baseline = 267_660.0;
        
        println!("\n=== Results ===");
        println!("Configuration: Sync + Zero-Copy + 131k");
        println!("Hashrate: {:.2} H/s\n", hashrate);
        
        println!("Comparison:");
        println!("  vs Async+Clone:     {:+.2}% ({:.2} H/s diff)", 
            ((hashrate - async_clone_baseline) / async_clone_baseline) * 100.0,
            hashrate - async_clone_baseline);
        println!("  vs Sync+Clone:      {:+.2}% ({:.2} H/s diff)", 
            ((hashrate - sync_clone_baseline) / sync_clone_baseline) * 100.0,
            hashrate - sync_clone_baseline);
        println!("  vs Async+ZeroCopy:  {:+.2}% ({:.2} H/s diff)", 
            ((hashrate - async_zerocopy_baseline) / async_zerocopy_baseline) * 100.0,
            hashrate - async_zerocopy_baseline);
        
        println!("\n=== Matrix Status ===");
        println!("Async + Clone:     {:.0} H/s ✅", async_clone_baseline);
        println!("Async + Zero-Copy: {:.0} H/s ✅", async_zerocopy_baseline);
        println!("Sync + Clone:      {:.0} H/s ✅", sync_clone_baseline);
        println!("Sync + Zero-Copy:  {:.0} H/s ✅ (just tested)", hashrate);
    }
}

#[cfg(feature = "cuda")]
fn run_sync_zerocopy(
    gpu_count: usize,
    batch_per_gpu: usize,
    rom: Arc<Rom>,
    duration: u64
) -> f64 {
    let total_batch = batch_per_gpu * gpu_count;
    let all_salts: Vec<Vec<u8>> = (0..total_batch)
        .map(|i| format!("test_{}", i).into_bytes())
        .collect();
    let all_salts = Arc::new(all_salts);
    
    let mut gpus = Vec::new();
    for gpu_id in 0..gpu_count {
        gpus.push(Arc::new(CudaAshmaize::new_with_device(gpu_id).unwrap()));
    }
    
    let start_time = Instant::now();
    let total_hashes = Arc::new(AtomicU64::new(0));
    let running = Arc::new(AtomicBool::new(true));
    
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
            
            while running.load(Ordering::Relaxed) {
                // Zero-copy: Create Arc-wrapped references (no data clone)
                let gpu_salts_arc: Arc<Vec<&[u8]>> = Arc::new(
                    all_salts[start_idx..end_idx]
                        .iter()
                        .map(|s| s.as_slice())
                        .collect()
                );
                
                let salt_refs: Vec<&[u8]> = gpu_salts_arc.iter().copied().collect();
                
                let result = cuda.hash_parallel(&salt_refs, &rom, 8, 256)
                    .map_err(|e| format!("{:?}", e));
                
                result_tx.send((gpu_id, result)).ok();
            }
        });
        handles.push(handle);
    }
    drop(result_tx);
    
    let total_hashes_clone = Arc::clone(&total_hashes);
    let collector = thread::spawn(move || {
        while let Ok((_gpu_id, result)) = result_rx.recv() {
            if result.is_ok() {
                total_hashes_clone.fetch_add(batch_per_gpu as u64, Ordering::Relaxed);
            }
        }
    });
    
    thread::sleep(Duration::from_secs(duration));
    
    running.store(false, Ordering::Relaxed);
    for handle in handles {
        handle.join().ok();
    }
    collector.join().ok();
    
    let elapsed = start_time.elapsed();
    let final_hashes = total_hashes.load(Ordering::Relaxed);
    final_hashes as f64 / elapsed.as_secs_f64()
}




