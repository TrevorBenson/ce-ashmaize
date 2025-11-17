// Phase 6 Optimization Test 1: Block Size Tuning (WORKING TEST)
// Test different CUDA thread block sizes for optimal occupancy

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
        println!("=== Phase 6 Test 1: Block Size Tuning ===\n");
        
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
        let block_sizes = vec![128, 256, 384, 512, 768, 1024];
        
        println!("Configuration:");
        println!("  Batch per GPU: {}", batch_per_gpu);
        println!("  Test duration: {}s per block size", TEST_DURATION);
        println!("  Block sizes: {:?}\n", block_sizes);
        println!("Total time: ~{} minutes\n", (block_sizes.len() * TEST_DURATION as usize) / 60 + 1);
        
        println!("{}", "=".repeat(70));
        
        let mut results = Vec::new();
        
        for block_size in block_sizes {
            println!("\n Testing block_size = {}...", block_size);
            
            let hashrate = run_test_with_block_size(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION, block_size);
            
            println!("  Result: {:.2} H/s", hashrate);
            results.push((block_size, hashrate));
            
            // Cooling period
            thread::sleep(Duration::from_secs(5));
        }
        
        println!("\n{}", "=".repeat(70));
        println!("\n=== Block Size Results ===\n");
        
        let baseline_256 = results.iter().find(|(bs, _)| *bs == 256).map(|(_, hr)| *hr).unwrap_or(294_000.0);
        let max_hashrate = results.iter().map(|(_, h)| *h).fold(0.0f64, f64::max);
        
        println!("{:<12} {:>15} {:>12} {:>12}",
            "Block Size", "Hashrate (H/s)", "vs 256", "vs Best");
        println!("{}", "-".repeat(55));
        
        for (block_size, hashrate) in &results {
            let vs_256 = ((hashrate - baseline_256) / baseline_256) * 100.0;
            let vs_best = (hashrate / max_hashrate) * 100.0;
            let marker = if *hashrate == max_hashrate { " ✅" } else { "" };
            
            println!("{:<12} {:>15.2} {:>11.1}% {:>11.1}%{}",
                block_size, hashrate, vs_256, vs_best, marker);
        }
        
        let (best_block_size, best_hashrate) = results.iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();
        
        println!("\n🎯 Optimal Block Size: {}", best_block_size);
        println!("   Performance: {:.2} H/s", best_hashrate);
        println!("   Per GPU: {:.2} H/s", best_hashrate / gpu_count as f64);
        
        let improvement = ((best_hashrate - baseline_256) / baseline_256) * 100.0;
        println!("   Improvement over baseline (256): {:+.2}%", improvement);
        
        if improvement > 2.0 {
            println!("\n✅ SIGNIFICANT IMPROVEMENT - Update default block size to {}", best_block_size);
        } else if improvement > 0.5 {
            println!("\n✅ MINOR IMPROVEMENT - Consider updating to {}", best_block_size);
        } else {
            println!("\n⚠️  NO SIGNIFICANT IMPROVEMENT - Keep block size 256");
        }
    }
}

#[cfg(feature = "cuda")]
fn run_test_with_block_size(
    gpu_count: usize,
    batch_per_gpu: usize,
    rom: Arc<Rom>,
    duration: u64,
    block_size: u32,
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
                
                // Use new method with configurable block size
                let result = cuda.hash_parallel_with_block_size(&salt_refs, &rom, 8, 256, block_size)
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

// Test different CUDA thread block sizes for optimal occupancy

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
        println!("=== Phase 6 Test 1: Block Size Tuning ===\n");
        
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
        let block_sizes = vec![128, 256, 384, 512, 768, 1024];
        
        println!("Configuration:");
        println!("  Batch per GPU: {}", batch_per_gpu);
        println!("  Test duration: {}s per block size", TEST_DURATION);
        println!("  Block sizes: {:?}\n", block_sizes);
        println!("Total time: ~{} minutes\n", (block_sizes.len() * TEST_DURATION as usize) / 60 + 1);
        
        println!("{}", "=".repeat(70));
        
        let mut results = Vec::new();
        
        for block_size in block_sizes {
            println!("\n Testing block_size = {}...", block_size);
            
            let hashrate = run_test_with_block_size(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION, block_size);
            
            println!("  Result: {:.2} H/s", hashrate);
            results.push((block_size, hashrate));
            
            // Cooling period
            thread::sleep(Duration::from_secs(5));
        }
        
        println!("\n{}", "=".repeat(70));
        println!("\n=== Block Size Results ===\n");
        
        let baseline_256 = results.iter().find(|(bs, _)| *bs == 256).map(|(_, hr)| *hr).unwrap_or(294_000.0);
        let max_hashrate = results.iter().map(|(_, h)| *h).fold(0.0f64, f64::max);
        
        println!("{:<12} {:>15} {:>12} {:>12}",
            "Block Size", "Hashrate (H/s)", "vs 256", "vs Best");
        println!("{}", "-".repeat(55));
        
        for (block_size, hashrate) in &results {
            let vs_256 = ((hashrate - baseline_256) / baseline_256) * 100.0;
            let vs_best = (hashrate / max_hashrate) * 100.0;
            let marker = if *hashrate == max_hashrate { " ✅" } else { "" };
            
            println!("{:<12} {:>15.2} {:>11.1}% {:>11.1}%{}",
                block_size, hashrate, vs_256, vs_best, marker);
        }
        
        let (best_block_size, best_hashrate) = results.iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();
        
        println!("\n🎯 Optimal Block Size: {}", best_block_size);
        println!("   Performance: {:.2} H/s", best_hashrate);
        println!("   Per GPU: {:.2} H/s", best_hashrate / gpu_count as f64);
        
        let improvement = ((best_hashrate - baseline_256) / baseline_256) * 100.0;
        println!("   Improvement over baseline (256): {:+.2}%", improvement);
        
        if improvement > 2.0 {
            println!("\n✅ SIGNIFICANT IMPROVEMENT - Update default block size to {}", best_block_size);
        } else if improvement > 0.5 {
            println!("\n✅ MINOR IMPROVEMENT - Consider updating to {}", best_block_size);
        } else {
            println!("\n⚠️  NO SIGNIFICANT IMPROVEMENT - Keep block size 256");
        }
    }
}

#[cfg(feature = "cuda")]
fn run_test_with_block_size(
    gpu_count: usize,
    batch_per_gpu: usize,
    rom: Arc<Rom>,
    duration: u64,
    block_size: u32,
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
                
                // Use new method with configurable block size
                let result = cuda.hash_parallel_with_block_size(&salt_refs, &rom, 8, 256, block_size)
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

// Test different CUDA thread block sizes for optimal occupancy

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
        println!("=== Phase 6 Test 1: Block Size Tuning ===\n");
        
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
        let block_sizes = vec![128, 256, 384, 512, 768, 1024];
        
        println!("Configuration:");
        println!("  Batch per GPU: {}", batch_per_gpu);
        println!("  Test duration: {}s per block size", TEST_DURATION);
        println!("  Block sizes: {:?}\n", block_sizes);
        println!("Total time: ~{} minutes\n", (block_sizes.len() * TEST_DURATION as usize) / 60 + 1);
        
        println!("{}", "=".repeat(70));
        
        let mut results = Vec::new();
        
        for block_size in block_sizes {
            println!("\n Testing block_size = {}...", block_size);
            
            let hashrate = run_test_with_block_size(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION, block_size);
            
            println!("  Result: {:.2} H/s", hashrate);
            results.push((block_size, hashrate));
            
            // Cooling period
            thread::sleep(Duration::from_secs(5));
        }
        
        println!("\n{}", "=".repeat(70));
        println!("\n=== Block Size Results ===\n");
        
        let baseline_256 = results.iter().find(|(bs, _)| *bs == 256).map(|(_, hr)| *hr).unwrap_or(294_000.0);
        let max_hashrate = results.iter().map(|(_, h)| *h).fold(0.0f64, f64::max);
        
        println!("{:<12} {:>15} {:>12} {:>12}",
            "Block Size", "Hashrate (H/s)", "vs 256", "vs Best");
        println!("{}", "-".repeat(55));
        
        for (block_size, hashrate) in &results {
            let vs_256 = ((hashrate - baseline_256) / baseline_256) * 100.0;
            let vs_best = (hashrate / max_hashrate) * 100.0;
            let marker = if *hashrate == max_hashrate { " ✅" } else { "" };
            
            println!("{:<12} {:>15.2} {:>11.1}% {:>11.1}%{}",
                block_size, hashrate, vs_256, vs_best, marker);
        }
        
        let (best_block_size, best_hashrate) = results.iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();
        
        println!("\n🎯 Optimal Block Size: {}", best_block_size);
        println!("   Performance: {:.2} H/s", best_hashrate);
        println!("   Per GPU: {:.2} H/s", best_hashrate / gpu_count as f64);
        
        let improvement = ((best_hashrate - baseline_256) / baseline_256) * 100.0;
        println!("   Improvement over baseline (256): {:+.2}%", improvement);
        
        if improvement > 2.0 {
            println!("\n✅ SIGNIFICANT IMPROVEMENT - Update default block size to {}", best_block_size);
        } else if improvement > 0.5 {
            println!("\n✅ MINOR IMPROVEMENT - Consider updating to {}", best_block_size);
        } else {
            println!("\n⚠️  NO SIGNIFICANT IMPROVEMENT - Keep block size 256");
        }
    }
}

#[cfg(feature = "cuda")]
fn run_test_with_block_size(
    gpu_count: usize,
    batch_per_gpu: usize,
    rom: Arc<Rom>,
    duration: u64,
    block_size: u32,
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
                
                // Use new method with configurable block size
                let result = cuda.hash_parallel_with_block_size(&salt_refs, &rom, 8, 256, block_size)
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




