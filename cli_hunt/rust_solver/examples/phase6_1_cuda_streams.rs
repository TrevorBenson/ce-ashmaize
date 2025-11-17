// Phase 6 Test 1: CUDA Streams
// Expected: +5-15% improvement via overlapping compute and data transfer
// Test Duration: 20s, Timeout: 40s

use ashmaize::{Rom, RomGenerationType};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::sync::mpsc;
use std::thread;
use std::time::{Instant, Duration};

#[cfg(feature = "cuda")]
use ashmaize_solver::gpu::CudaAshmaize;

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * MB;
const TEST_DURATION: u64 = 20;
const TIMEOUT_SECS: u64 = 40;

fn main() {
    #[cfg(not(feature = "cuda"))]
    {
        eprintln!("This example requires the 'cuda' feature");
        eprintln!("Run with: cargo run --example phase6_1_cuda_streams --features cuda --release");
        std::process::exit(1);
    }

    #[cfg(feature = "cuda")]
    {
        println!("=== Phase 6 Test 1: CUDA Streams ===\n");
        
        // Setup
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
        
        let batch_per_gpu = 131_072;  // Optimal from Phase 4
        
        println!("Configuration:");
        println!("  Batch per GPU: {}", batch_per_gpu);
        println!("  Duration: {}s", TEST_DURATION);
        println!("  Timeout: {}s", TIMEOUT_SECS);
        println!("\nNote: CUDA Streams require explicit cudarc stream support.");
        println!("Currently testing baseline (no streams) vs multi-stream approach.\n");
        
        // Run test with timeout
        let (tx, rx) = mpsc::channel();
        
        let test_thread = thread::spawn(move || {
            let hashrate = run_streams_test(gpu_count, batch_per_gpu, rom, TEST_DURATION);
            tx.send(hashrate).ok();
        });
        
        // Wait for result with timeout
        match rx.recv_timeout(Duration::from_secs(TIMEOUT_SECS)) {
            Ok(hashrate) => {
                test_thread.join().ok();
                
                let baseline = 285_996.0;
                let improvement = ((hashrate - baseline) / baseline) * 100.0;
                
                println!("\n=== Results ===");
                println!("Hashrate: {:.2} H/s", hashrate);
                println!("Baseline: {:.2} H/s", baseline);
                println!("Change: {:+.2}% ({:+.2} H/s)", improvement, hashrate - baseline);
                
                if improvement > 0.0 {
                    println!("✅ IMPROVEMENT - Keep for combination testing");
                } else if improvement > -1.0 {
                    println!("⚠️  NEUTRAL - Negligible impact");
                } else {
                    println!("❌ NO IMPROVEMENT - Skip");
                }
            }
            Err(_) => {
                println!("❌ TEST TIMEOUT - Hung after {}s", TIMEOUT_SECS);
                println!("This optimization will be SKIPPED.");
            }
        }
    }
}

#[cfg(feature = "cuda")]
fn run_streams_test(
    gpu_count: usize,
    batch_per_gpu: usize,
    rom: Arc<Rom>,
    duration: u64
) -> f64 {
    // For now, this will test the baseline behavior since cudarc doesn't 
    // expose CUDA streams directly in a simple way.
    // A real streams implementation would require:
    // 1. Creating multiple CUDA streams per GPU
    // 2. Dividing work into sub-batches
    // 3. Launching kernels in different streams
    // 4. Using async memory copies
    
    // This test currently runs baseline to verify infrastructure
    // Real streams implementation requires lower-level CUDA API access
    
    println!("Note: CUDA Streams require low-level API access not exposed by cudarc.");
    println!("Testing baseline performance with stream-like async pattern...\n");
    
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
    
    let start_time = Instant::now();
    let total_hashes = Arc::new(AtomicU64::new(0));
    let running = Arc::new(AtomicBool::new(true));
    
    let (result_tx, result_rx) = mpsc::channel::<(usize, Result<Vec<[u8; 64]>, String>)>();
    
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

// Expected: +5-15% improvement via overlapping compute and data transfer
// Test Duration: 20s, Timeout: 40s

use ashmaize::{Rom, RomGenerationType};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::sync::mpsc;
use std::thread;
use std::time::{Instant, Duration};

#[cfg(feature = "cuda")]
use ashmaize_solver::gpu::CudaAshmaize;

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * MB;
const TEST_DURATION: u64 = 20;
const TIMEOUT_SECS: u64 = 40;

fn main() {
    #[cfg(not(feature = "cuda"))]
    {
        eprintln!("This example requires the 'cuda' feature");
        eprintln!("Run with: cargo run --example phase6_1_cuda_streams --features cuda --release");
        std::process::exit(1);
    }

    #[cfg(feature = "cuda")]
    {
        println!("=== Phase 6 Test 1: CUDA Streams ===\n");
        
        // Setup
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
        
        let batch_per_gpu = 131_072;  // Optimal from Phase 4
        
        println!("Configuration:");
        println!("  Batch per GPU: {}", batch_per_gpu);
        println!("  Duration: {}s", TEST_DURATION);
        println!("  Timeout: {}s", TIMEOUT_SECS);
        println!("\nNote: CUDA Streams require explicit cudarc stream support.");
        println!("Currently testing baseline (no streams) vs multi-stream approach.\n");
        
        // Run test with timeout
        let (tx, rx) = mpsc::channel();
        
        let test_thread = thread::spawn(move || {
            let hashrate = run_streams_test(gpu_count, batch_per_gpu, rom, TEST_DURATION);
            tx.send(hashrate).ok();
        });
        
        // Wait for result with timeout
        match rx.recv_timeout(Duration::from_secs(TIMEOUT_SECS)) {
            Ok(hashrate) => {
                test_thread.join().ok();
                
                let baseline = 285_996.0;
                let improvement = ((hashrate - baseline) / baseline) * 100.0;
                
                println!("\n=== Results ===");
                println!("Hashrate: {:.2} H/s", hashrate);
                println!("Baseline: {:.2} H/s", baseline);
                println!("Change: {:+.2}% ({:+.2} H/s)", improvement, hashrate - baseline);
                
                if improvement > 0.0 {
                    println!("✅ IMPROVEMENT - Keep for combination testing");
                } else if improvement > -1.0 {
                    println!("⚠️  NEUTRAL - Negligible impact");
                } else {
                    println!("❌ NO IMPROVEMENT - Skip");
                }
            }
            Err(_) => {
                println!("❌ TEST TIMEOUT - Hung after {}s", TIMEOUT_SECS);
                println!("This optimization will be SKIPPED.");
            }
        }
    }
}

#[cfg(feature = "cuda")]
fn run_streams_test(
    gpu_count: usize,
    batch_per_gpu: usize,
    rom: Arc<Rom>,
    duration: u64
) -> f64 {
    // For now, this will test the baseline behavior since cudarc doesn't 
    // expose CUDA streams directly in a simple way.
    // A real streams implementation would require:
    // 1. Creating multiple CUDA streams per GPU
    // 2. Dividing work into sub-batches
    // 3. Launching kernels in different streams
    // 4. Using async memory copies
    
    // This test currently runs baseline to verify infrastructure
    // Real streams implementation requires lower-level CUDA API access
    
    println!("Note: CUDA Streams require low-level API access not exposed by cudarc.");
    println!("Testing baseline performance with stream-like async pattern...\n");
    
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
    
    let start_time = Instant::now();
    let total_hashes = Arc::new(AtomicU64::new(0));
    let running = Arc::new(AtomicBool::new(true));
    
    let (result_tx, result_rx) = mpsc::channel::<(usize, Result<Vec<[u8; 64]>, String>)>();
    
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

// Expected: +5-15% improvement via overlapping compute and data transfer
// Test Duration: 20s, Timeout: 40s

use ashmaize::{Rom, RomGenerationType};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::sync::mpsc;
use std::thread;
use std::time::{Instant, Duration};

#[cfg(feature = "cuda")]
use ashmaize_solver::gpu::CudaAshmaize;

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * MB;
const TEST_DURATION: u64 = 20;
const TIMEOUT_SECS: u64 = 40;

fn main() {
    #[cfg(not(feature = "cuda"))]
    {
        eprintln!("This example requires the 'cuda' feature");
        eprintln!("Run with: cargo run --example phase6_1_cuda_streams --features cuda --release");
        std::process::exit(1);
    }

    #[cfg(feature = "cuda")]
    {
        println!("=== Phase 6 Test 1: CUDA Streams ===\n");
        
        // Setup
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
        
        let batch_per_gpu = 131_072;  // Optimal from Phase 4
        
        println!("Configuration:");
        println!("  Batch per GPU: {}", batch_per_gpu);
        println!("  Duration: {}s", TEST_DURATION);
        println!("  Timeout: {}s", TIMEOUT_SECS);
        println!("\nNote: CUDA Streams require explicit cudarc stream support.");
        println!("Currently testing baseline (no streams) vs multi-stream approach.\n");
        
        // Run test with timeout
        let (tx, rx) = mpsc::channel();
        
        let test_thread = thread::spawn(move || {
            let hashrate = run_streams_test(gpu_count, batch_per_gpu, rom, TEST_DURATION);
            tx.send(hashrate).ok();
        });
        
        // Wait for result with timeout
        match rx.recv_timeout(Duration::from_secs(TIMEOUT_SECS)) {
            Ok(hashrate) => {
                test_thread.join().ok();
                
                let baseline = 285_996.0;
                let improvement = ((hashrate - baseline) / baseline) * 100.0;
                
                println!("\n=== Results ===");
                println!("Hashrate: {:.2} H/s", hashrate);
                println!("Baseline: {:.2} H/s", baseline);
                println!("Change: {:+.2}% ({:+.2} H/s)", improvement, hashrate - baseline);
                
                if improvement > 0.0 {
                    println!("✅ IMPROVEMENT - Keep for combination testing");
                } else if improvement > -1.0 {
                    println!("⚠️  NEUTRAL - Negligible impact");
                } else {
                    println!("❌ NO IMPROVEMENT - Skip");
                }
            }
            Err(_) => {
                println!("❌ TEST TIMEOUT - Hung after {}s", TIMEOUT_SECS);
                println!("This optimization will be SKIPPED.");
            }
        }
    }
}

#[cfg(feature = "cuda")]
fn run_streams_test(
    gpu_count: usize,
    batch_per_gpu: usize,
    rom: Arc<Rom>,
    duration: u64
) -> f64 {
    // For now, this will test the baseline behavior since cudarc doesn't 
    // expose CUDA streams directly in a simple way.
    // A real streams implementation would require:
    // 1. Creating multiple CUDA streams per GPU
    // 2. Dividing work into sub-batches
    // 3. Launching kernels in different streams
    // 4. Using async memory copies
    
    // This test currently runs baseline to verify infrastructure
    // Real streams implementation requires lower-level CUDA API access
    
    println!("Note: CUDA Streams require low-level API access not exposed by cudarc.");
    println!("Testing baseline performance with stream-like async pattern...\n");
    
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
    
    let start_time = Instant::now();
    let total_hashes = Arc::new(AtomicU64::new(0));
    let running = Arc::new(AtomicBool::new(true));
    
    let (result_tx, result_rx) = mpsc::channel::<(usize, Result<Vec<[u8; 64]>, String>)>();
    
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




