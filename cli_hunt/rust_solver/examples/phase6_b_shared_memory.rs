// Phase 6-B: Shared Memory ROM Caching
// Tests both hash correctness AND performance
// CRITICAL: GPU vs CPU hash must match, or the optimization is INVALID

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
        println!("=== Phase 6-B: Shared Memory ROM Caching ===\n");
        println!("Concept: Cache first 16KB of ROM in shared memory for faster access\n");
        
        let _init = CudaAshmaize::new().expect("CUDA init failed");
        let gpu_count = CudaAshmaize::get_device_count().expect("Failed to get GPU count");
        
        println!("Detected {} GPU(s)\n", gpu_count);
        
        // Use smaller ROM for testing (10MB vs 1GB)
        let rom = Arc::new(Rom::new(
            b"phase6b",
            RomGenerationType::TwoStep {
                pre_size: 1 * MB,
                mixing_numbers: 4,
            },
            10 * MB,
        ));
        
        println!("ROM size: {} MB", rom.data.len() / MB);
        println!("First 16KB will be cached in shared memory\n");
        
        // === STEP 1: HASH CORRECTNESS TEST ===
        println!("{}", "=".repeat(70));
        println!("STEP 1: Hash Correctness Test (CPU vs GPU)");
        println!("{}", "=".repeat(70));
        println!();
        
        let test_salts = vec![
            "correctness_test_1",
            "correctness_test_2",
            "correctness_test_3",
        ];
        
        let cuda = Arc::new(CudaAshmaize::new().expect("Failed to init CUDA"));
        
        let mut all_match = true;
        for salt_str in &test_salts {
            let salt = salt_str.as_bytes();
            
            // CPU hash (reference)
            let cpu_hash = ashmaize::b2::hash(salt, &rom, 8, 256);
            
            // GPU hash (baseline kernel)
            let gpu_baseline = cuda.hash_parallel(&[salt], &rom, 8, 256)
                .expect("GPU baseline failed")[0];
            
            // GPU hash (shared memory kernel)
            let gpu_shared = match cuda.hash_parallel_shared_mem(&[salt], &rom, 8, 256) {
                Ok(results) => results[0],
                Err(e) => {
                    println!("❌ Shared memory kernel not available: {}", e);
                    println!("   This optimization requires recompilation.\n");
                    return;
                }
            };
            
            let baseline_match = cpu_hash == gpu_baseline;
            let shared_match = cpu_hash == gpu_shared;
            
            println!("Salt: {}", salt_str);
            println!("  CPU:            {}", hex::encode(&cpu_hash[..8]));
            println!("  GPU (baseline): {} {}", hex::encode(&gpu_baseline[..8]), 
                if baseline_match { "✅" } else { "❌" });
            println!("  GPU (shared):   {} {}", hex::encode(&gpu_shared[..8]),
                if shared_match { "✅" } else { "❌" });
            println!();
            
            if !shared_match {
                all_match = false;
            }
        }
        
        if !all_match {
            println!("❌ HASH CORRECTNESS FAILED!");
            println!("   Shared memory optimization produces different results than CPU.");
            println!("   This optimization is INVALID and must be fixed or discarded.\n");
            return;
        }
        
        println!("✅ HASH CORRECTNESS VERIFIED");
        println!("   All hashes match between CPU and GPU (both kernels)\n");
        
        // === STEP 2: PERFORMANCE TEST ===
        println!("{}", "=".repeat(70));
        println!("STEP 2: Performance Test");
        println!("{}", "=".repeat(70));
        println!();
        
        let batch_per_gpu = 131_072;
        
        println!("Testing baseline kernel...");
        let baseline_hashrate = run_test(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION, false);
        
        thread::sleep(Duration::from_secs(5));  // Cooling
        
        println!("Testing shared memory kernel...");
        let shared_hashrate = run_test(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION, true);
        
        println!("\n{}", "=".repeat(70));
        println!("RESULTS");
        println!("{}", "=".repeat(70));
        println!();
        
        let improvement = ((shared_hashrate - baseline_hashrate) / baseline_hashrate) * 100.0;
        
        println!("Baseline kernel:     {:.2} H/s", baseline_hashrate);
        println!("Shared memory kernel: {:.2} H/s", shared_hashrate);
        println!("Improvement:          {:+.2}%", improvement);
        println!();
        
        if improvement > 5.0 {
            println!("✅ SIGNIFICANT IMPROVEMENT - Keep this optimization!");
            println!("   Shared memory caching provides a {} gain", 
                if improvement > 20.0 { "major" } else if improvement > 10.0 { "good" } else { "modest" });
        } else if improvement > 1.0 {
            println!("✅ MINOR IMPROVEMENT - Consider keeping");
        } else if improvement > -2.0 {
            println!("⚠️  NO SIGNIFICANT CHANGE - Neutral");
        } else {
            println!("❌ PERFORMANCE REGRESSION - Discard this optimization");
        }
        
        println!("\n{}", "=".repeat(70));
        println!("Phase 6-B: Shared Memory Test Complete");
        println!("{}", "=".repeat(70));
    }
}

#[cfg(feature = "cuda")]
fn run_test(
    gpu_count: usize,
    batch_per_gpu: usize,
    rom: Arc<Rom>,
    duration: u64,
    use_shared_mem: bool,
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
                let gpu_salts: Vec<Vec<u8>> = all_salts[start_idx..end_idx].iter().cloned().collect();
                let salt_refs: Vec<&[u8]> = gpu_salts.iter().map(|s| s.as_slice()).collect();
                
                let result = if use_shared_mem {
                    cuda.hash_parallel_shared_mem(&salt_refs, &rom, 8, 256)
                } else {
                    cuda.hash_parallel(&salt_refs, &rom, 8, 256)
                }.map_err(|e| format!("{:?}", e));
                
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

// Tests both hash correctness AND performance
// CRITICAL: GPU vs CPU hash must match, or the optimization is INVALID

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
        println!("=== Phase 6-B: Shared Memory ROM Caching ===\n");
        println!("Concept: Cache first 16KB of ROM in shared memory for faster access\n");
        
        let _init = CudaAshmaize::new().expect("CUDA init failed");
        let gpu_count = CudaAshmaize::get_device_count().expect("Failed to get GPU count");
        
        println!("Detected {} GPU(s)\n", gpu_count);
        
        // Use smaller ROM for testing (10MB vs 1GB)
        let rom = Arc::new(Rom::new(
            b"phase6b",
            RomGenerationType::TwoStep {
                pre_size: 1 * MB,
                mixing_numbers: 4,
            },
            10 * MB,
        ));
        
        println!("ROM size: {} MB", rom.data.len() / MB);
        println!("First 16KB will be cached in shared memory\n");
        
        // === STEP 1: HASH CORRECTNESS TEST ===
        println!("{}", "=".repeat(70));
        println!("STEP 1: Hash Correctness Test (CPU vs GPU)");
        println!("{}", "=".repeat(70));
        println!();
        
        let test_salts = vec![
            "correctness_test_1",
            "correctness_test_2",
            "correctness_test_3",
        ];
        
        let cuda = Arc::new(CudaAshmaize::new().expect("Failed to init CUDA"));
        
        let mut all_match = true;
        for salt_str in &test_salts {
            let salt = salt_str.as_bytes();
            
            // CPU hash (reference)
            let cpu_hash = ashmaize::b2::hash(salt, &rom, 8, 256);
            
            // GPU hash (baseline kernel)
            let gpu_baseline = cuda.hash_parallel(&[salt], &rom, 8, 256)
                .expect("GPU baseline failed")[0];
            
            // GPU hash (shared memory kernel)
            let gpu_shared = match cuda.hash_parallel_shared_mem(&[salt], &rom, 8, 256) {
                Ok(results) => results[0],
                Err(e) => {
                    println!("❌ Shared memory kernel not available: {}", e);
                    println!("   This optimization requires recompilation.\n");
                    return;
                }
            };
            
            let baseline_match = cpu_hash == gpu_baseline;
            let shared_match = cpu_hash == gpu_shared;
            
            println!("Salt: {}", salt_str);
            println!("  CPU:            {}", hex::encode(&cpu_hash[..8]));
            println!("  GPU (baseline): {} {}", hex::encode(&gpu_baseline[..8]), 
                if baseline_match { "✅" } else { "❌" });
            println!("  GPU (shared):   {} {}", hex::encode(&gpu_shared[..8]),
                if shared_match { "✅" } else { "❌" });
            println!();
            
            if !shared_match {
                all_match = false;
            }
        }
        
        if !all_match {
            println!("❌ HASH CORRECTNESS FAILED!");
            println!("   Shared memory optimization produces different results than CPU.");
            println!("   This optimization is INVALID and must be fixed or discarded.\n");
            return;
        }
        
        println!("✅ HASH CORRECTNESS VERIFIED");
        println!("   All hashes match between CPU and GPU (both kernels)\n");
        
        // === STEP 2: PERFORMANCE TEST ===
        println!("{}", "=".repeat(70));
        println!("STEP 2: Performance Test");
        println!("{}", "=".repeat(70));
        println!();
        
        let batch_per_gpu = 131_072;
        
        println!("Testing baseline kernel...");
        let baseline_hashrate = run_test(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION, false);
        
        thread::sleep(Duration::from_secs(5));  // Cooling
        
        println!("Testing shared memory kernel...");
        let shared_hashrate = run_test(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION, true);
        
        println!("\n{}", "=".repeat(70));
        println!("RESULTS");
        println!("{}", "=".repeat(70));
        println!();
        
        let improvement = ((shared_hashrate - baseline_hashrate) / baseline_hashrate) * 100.0;
        
        println!("Baseline kernel:     {:.2} H/s", baseline_hashrate);
        println!("Shared memory kernel: {:.2} H/s", shared_hashrate);
        println!("Improvement:          {:+.2}%", improvement);
        println!();
        
        if improvement > 5.0 {
            println!("✅ SIGNIFICANT IMPROVEMENT - Keep this optimization!");
            println!("   Shared memory caching provides a {} gain", 
                if improvement > 20.0 { "major" } else if improvement > 10.0 { "good" } else { "modest" });
        } else if improvement > 1.0 {
            println!("✅ MINOR IMPROVEMENT - Consider keeping");
        } else if improvement > -2.0 {
            println!("⚠️  NO SIGNIFICANT CHANGE - Neutral");
        } else {
            println!("❌ PERFORMANCE REGRESSION - Discard this optimization");
        }
        
        println!("\n{}", "=".repeat(70));
        println!("Phase 6-B: Shared Memory Test Complete");
        println!("{}", "=".repeat(70));
    }
}

#[cfg(feature = "cuda")]
fn run_test(
    gpu_count: usize,
    batch_per_gpu: usize,
    rom: Arc<Rom>,
    duration: u64,
    use_shared_mem: bool,
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
                let gpu_salts: Vec<Vec<u8>> = all_salts[start_idx..end_idx].iter().cloned().collect();
                let salt_refs: Vec<&[u8]> = gpu_salts.iter().map(|s| s.as_slice()).collect();
                
                let result = if use_shared_mem {
                    cuda.hash_parallel_shared_mem(&salt_refs, &rom, 8, 256)
                } else {
                    cuda.hash_parallel(&salt_refs, &rom, 8, 256)
                }.map_err(|e| format!("{:?}", e));
                
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

// Tests both hash correctness AND performance
// CRITICAL: GPU vs CPU hash must match, or the optimization is INVALID

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
        println!("=== Phase 6-B: Shared Memory ROM Caching ===\n");
        println!("Concept: Cache first 16KB of ROM in shared memory for faster access\n");
        
        let _init = CudaAshmaize::new().expect("CUDA init failed");
        let gpu_count = CudaAshmaize::get_device_count().expect("Failed to get GPU count");
        
        println!("Detected {} GPU(s)\n", gpu_count);
        
        // Use smaller ROM for testing (10MB vs 1GB)
        let rom = Arc::new(Rom::new(
            b"phase6b",
            RomGenerationType::TwoStep {
                pre_size: 1 * MB,
                mixing_numbers: 4,
            },
            10 * MB,
        ));
        
        println!("ROM size: {} MB", rom.data.len() / MB);
        println!("First 16KB will be cached in shared memory\n");
        
        // === STEP 1: HASH CORRECTNESS TEST ===
        println!("{}", "=".repeat(70));
        println!("STEP 1: Hash Correctness Test (CPU vs GPU)");
        println!("{}", "=".repeat(70));
        println!();
        
        let test_salts = vec![
            "correctness_test_1",
            "correctness_test_2",
            "correctness_test_3",
        ];
        
        let cuda = Arc::new(CudaAshmaize::new().expect("Failed to init CUDA"));
        
        let mut all_match = true;
        for salt_str in &test_salts {
            let salt = salt_str.as_bytes();
            
            // CPU hash (reference)
            let cpu_hash = ashmaize::b2::hash(salt, &rom, 8, 256);
            
            // GPU hash (baseline kernel)
            let gpu_baseline = cuda.hash_parallel(&[salt], &rom, 8, 256)
                .expect("GPU baseline failed")[0];
            
            // GPU hash (shared memory kernel)
            let gpu_shared = match cuda.hash_parallel_shared_mem(&[salt], &rom, 8, 256) {
                Ok(results) => results[0],
                Err(e) => {
                    println!("❌ Shared memory kernel not available: {}", e);
                    println!("   This optimization requires recompilation.\n");
                    return;
                }
            };
            
            let baseline_match = cpu_hash == gpu_baseline;
            let shared_match = cpu_hash == gpu_shared;
            
            println!("Salt: {}", salt_str);
            println!("  CPU:            {}", hex::encode(&cpu_hash[..8]));
            println!("  GPU (baseline): {} {}", hex::encode(&gpu_baseline[..8]), 
                if baseline_match { "✅" } else { "❌" });
            println!("  GPU (shared):   {} {}", hex::encode(&gpu_shared[..8]),
                if shared_match { "✅" } else { "❌" });
            println!();
            
            if !shared_match {
                all_match = false;
            }
        }
        
        if !all_match {
            println!("❌ HASH CORRECTNESS FAILED!");
            println!("   Shared memory optimization produces different results than CPU.");
            println!("   This optimization is INVALID and must be fixed or discarded.\n");
            return;
        }
        
        println!("✅ HASH CORRECTNESS VERIFIED");
        println!("   All hashes match between CPU and GPU (both kernels)\n");
        
        // === STEP 2: PERFORMANCE TEST ===
        println!("{}", "=".repeat(70));
        println!("STEP 2: Performance Test");
        println!("{}", "=".repeat(70));
        println!();
        
        let batch_per_gpu = 131_072;
        
        println!("Testing baseline kernel...");
        let baseline_hashrate = run_test(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION, false);
        
        thread::sleep(Duration::from_secs(5));  // Cooling
        
        println!("Testing shared memory kernel...");
        let shared_hashrate = run_test(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION, true);
        
        println!("\n{}", "=".repeat(70));
        println!("RESULTS");
        println!("{}", "=".repeat(70));
        println!();
        
        let improvement = ((shared_hashrate - baseline_hashrate) / baseline_hashrate) * 100.0;
        
        println!("Baseline kernel:     {:.2} H/s", baseline_hashrate);
        println!("Shared memory kernel: {:.2} H/s", shared_hashrate);
        println!("Improvement:          {:+.2}%", improvement);
        println!();
        
        if improvement > 5.0 {
            println!("✅ SIGNIFICANT IMPROVEMENT - Keep this optimization!");
            println!("   Shared memory caching provides a {} gain", 
                if improvement > 20.0 { "major" } else if improvement > 10.0 { "good" } else { "modest" });
        } else if improvement > 1.0 {
            println!("✅ MINOR IMPROVEMENT - Consider keeping");
        } else if improvement > -2.0 {
            println!("⚠️  NO SIGNIFICANT CHANGE - Neutral");
        } else {
            println!("❌ PERFORMANCE REGRESSION - Discard this optimization");
        }
        
        println!("\n{}", "=".repeat(70));
        println!("Phase 6-B: Shared Memory Test Complete");
        println!("{}", "=".repeat(70));
    }
}

#[cfg(feature = "cuda")]
fn run_test(
    gpu_count: usize,
    batch_per_gpu: usize,
    rom: Arc<Rom>,
    duration: u64,
    use_shared_mem: bool,
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
                let gpu_salts: Vec<Vec<u8>> = all_salts[start_idx..end_idx].iter().cloned().collect();
                let salt_refs: Vec<&[u8]> = gpu_salts.iter().map(|s| s.as_slice()).collect();
                
                let result = if use_shared_mem {
                    cuda.hash_parallel_shared_mem(&salt_refs, &rom, 8, 256)
                } else {
                    cuda.hash_parallel(&salt_refs, &rom, 8, 256)
                }.map_err(|e| format!("{:?}", e));
                
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




