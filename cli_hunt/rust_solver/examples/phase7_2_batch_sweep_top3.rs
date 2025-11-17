// Phase 7 Test 2: Batch Size Sweep for Top 3 Configurations
// Tests 6 batch sizes for each of the top 3 configs
// Test Duration: 30s per config

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
        println!("=== Phase 7 Test 2: Batch Size Sweep for Top 3 ===\n");
        
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
        
        let batch_sizes = vec![
            65_536,   // 65k
            98_304,   // 98k
            131_072,  // 131k (current optimal)
            163_840,  // 164k
            196_608,  // 197k
            229_376,  // 229k
        ];
        
        println!("Testing {} batch sizes for 3 configurations = {} total tests", 
            batch_sizes.len(), batch_sizes.len() * 3);
        println!("Estimated time: ~{} minutes\n", (batch_sizes.len() * 3 * TEST_DURATION as usize) / 60 + 2);
        println!("{}", "=".repeat(80));
        
        // Top 3 configs
        let configs = vec![
            ("Async+Clone", true, false),
            ("Sync+ZeroCopy", false, true),
            ("Async+ZeroCopy", true, true),
        ];
        
        let mut all_results = Vec::new();
        
        for (config_name, is_async, is_zerocopy) in configs {
            println!("\n### Testing: {} ###\n", config_name);
            
            for &batch_per_gpu in &batch_sizes {
                println!("  Batch {}: ", batch_per_gpu);
                
                let hashrate = if is_async {
                    if is_zerocopy {
                        run_async_zerocopy(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION)
                    } else {
                        run_async_clone(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION)
                    }
                } else {
                    run_sync_zerocopy(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION)
                };
                
                println!("    {:.2} H/s", hashrate);
                all_results.push((config_name, batch_per_gpu, hashrate));
                
                // Cooling period
                thread::sleep(Duration::from_secs(5));
            }
        }
        
        println!("\n{}", "=".repeat(80));
        println!("\n=== Complete Results ===\n");
        
        println!("{:<20} {:>12} {:>15}", "Configuration", "Batch/GPU", "Hashrate (H/s)");
        println!("{}", "-".repeat(50));
        
        let mut by_config: std::collections::HashMap<&str, Vec<(usize, f64)>> = std::collections::HashMap::new();
        for (config, batch, hashrate) in &all_results {
            by_config.entry(*config).or_insert_with(Vec::new).push((*batch, *hashrate));
        }
        
        for config_name in ["Async+Clone", "Sync+ZeroCopy", "Async+ZeroCopy"] {
            if let Some(results) = by_config.get(config_name) {
                println!("\n{}:", config_name);
                let max_hashrate = results.iter().map(|(_, h)| *h).fold(0.0f64, f64::max);
                for (batch, hashrate) in results {
                    let marker = if *hashrate == max_hashrate { " ✅ BEST" } else { "" };
                    println!("  {:>12}  {:>15.2}{}", batch, hashrate, marker);
                }
            }
        }
        
        println!("\n{}", "=".repeat(80));
        println!("\n=== Top 3 Overall ===\n");
        
        let mut sorted = all_results.clone();
        sorted.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
        
        for (i, (config, batch, hashrate)) in sorted.iter().take(3).enumerate() {
            println!("{}. {} @ {} batch: {:.2} H/s", i+1, config, batch, hashrate);
        }
        
        let (best_config, best_batch, best_hashrate) = &sorted[0];
        println!("\n🎯 Optimal: {} @ {} batch/GPU = {:.2} H/s", 
            best_config, best_batch, best_hashrate);
    }
}

#[cfg(feature = "cuda")]
fn run_async_clone(gpu_count: usize, batch_per_gpu: usize, rom: Arc<Rom>, duration: u64) -> f64 {
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
                
                let result = cuda.hash_parallel(&salt_refs, &rom, 8, 256).map_err(|e| format!("{:?}", e));
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

#[cfg(feature = "cuda")]
fn run_async_zerocopy(gpu_count: usize, batch_per_gpu: usize, rom: Arc<Rom>, duration: u64) -> f64 {
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
                let gpu_salts_arc: Arc<Vec<&[u8]>> = Arc::new(
                    all_salts[start_idx..end_idx].iter().map(|s| s.as_slice()).collect()
                );
                let salt_refs: Vec<&[u8]> = gpu_salts_arc.iter().copied().collect();
                
                let result = cuda.hash_parallel(&salt_refs, &rom, 8, 256).map_err(|e| format!("{:?}", e));
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

#[cfg(feature = "cuda")]
fn run_sync_zerocopy(gpu_count: usize, batch_per_gpu: usize, rom: Arc<Rom>, duration: u64) -> f64 {
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
                let gpu_salts_arc: Arc<Vec<&[u8]>> = Arc::new(
                    all_salts[start_idx..end_idx].iter().map(|s| s.as_slice()).collect()
                );
                let salt_refs: Vec<&[u8]> = gpu_salts_arc.iter().copied().collect();
                
                let result = cuda.hash_parallel(&salt_refs, &rom, 8, 256).map_err(|e| format!("{:?}", e));
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


// Test Duration: 30s per config

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
        println!("=== Phase 7 Test 2: Batch Size Sweep for Top 3 ===\n");
        
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
        
        let batch_sizes = vec![
            65_536,   // 65k
            98_304,   // 98k
            131_072,  // 131k (current optimal)
            163_840,  // 164k
            196_608,  // 197k
            229_376,  // 229k
        ];
        
        println!("Testing {} batch sizes for 3 configurations = {} total tests", 
            batch_sizes.len(), batch_sizes.len() * 3);
        println!("Estimated time: ~{} minutes\n", (batch_sizes.len() * 3 * TEST_DURATION as usize) / 60 + 2);
        println!("{}", "=".repeat(80));
        
        // Top 3 configs
        let configs = vec![
            ("Async+Clone", true, false),
            ("Sync+ZeroCopy", false, true),
            ("Async+ZeroCopy", true, true),
        ];
        
        let mut all_results = Vec::new();
        
        for (config_name, is_async, is_zerocopy) in configs {
            println!("\n### Testing: {} ###\n", config_name);
            
            for &batch_per_gpu in &batch_sizes {
                println!("  Batch {}: ", batch_per_gpu);
                
                let hashrate = if is_async {
                    if is_zerocopy {
                        run_async_zerocopy(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION)
                    } else {
                        run_async_clone(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION)
                    }
                } else {
                    run_sync_zerocopy(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION)
                };
                
                println!("    {:.2} H/s", hashrate);
                all_results.push((config_name, batch_per_gpu, hashrate));
                
                // Cooling period
                thread::sleep(Duration::from_secs(5));
            }
        }
        
        println!("\n{}", "=".repeat(80));
        println!("\n=== Complete Results ===\n");
        
        println!("{:<20} {:>12} {:>15}", "Configuration", "Batch/GPU", "Hashrate (H/s)");
        println!("{}", "-".repeat(50));
        
        let mut by_config: std::collections::HashMap<&str, Vec<(usize, f64)>> = std::collections::HashMap::new();
        for (config, batch, hashrate) in &all_results {
            by_config.entry(*config).or_insert_with(Vec::new).push((*batch, *hashrate));
        }
        
        for config_name in ["Async+Clone", "Sync+ZeroCopy", "Async+ZeroCopy"] {
            if let Some(results) = by_config.get(config_name) {
                println!("\n{}:", config_name);
                let max_hashrate = results.iter().map(|(_, h)| *h).fold(0.0f64, f64::max);
                for (batch, hashrate) in results {
                    let marker = if *hashrate == max_hashrate { " ✅ BEST" } else { "" };
                    println!("  {:>12}  {:>15.2}{}", batch, hashrate, marker);
                }
            }
        }
        
        println!("\n{}", "=".repeat(80));
        println!("\n=== Top 3 Overall ===\n");
        
        let mut sorted = all_results.clone();
        sorted.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
        
        for (i, (config, batch, hashrate)) in sorted.iter().take(3).enumerate() {
            println!("{}. {} @ {} batch: {:.2} H/s", i+1, config, batch, hashrate);
        }
        
        let (best_config, best_batch, best_hashrate) = &sorted[0];
        println!("\n🎯 Optimal: {} @ {} batch/GPU = {:.2} H/s", 
            best_config, best_batch, best_hashrate);
    }
}

#[cfg(feature = "cuda")]
fn run_async_clone(gpu_count: usize, batch_per_gpu: usize, rom: Arc<Rom>, duration: u64) -> f64 {
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
                
                let result = cuda.hash_parallel(&salt_refs, &rom, 8, 256).map_err(|e| format!("{:?}", e));
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

#[cfg(feature = "cuda")]
fn run_async_zerocopy(gpu_count: usize, batch_per_gpu: usize, rom: Arc<Rom>, duration: u64) -> f64 {
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
                let gpu_salts_arc: Arc<Vec<&[u8]>> = Arc::new(
                    all_salts[start_idx..end_idx].iter().map(|s| s.as_slice()).collect()
                );
                let salt_refs: Vec<&[u8]> = gpu_salts_arc.iter().copied().collect();
                
                let result = cuda.hash_parallel(&salt_refs, &rom, 8, 256).map_err(|e| format!("{:?}", e));
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

#[cfg(feature = "cuda")]
fn run_sync_zerocopy(gpu_count: usize, batch_per_gpu: usize, rom: Arc<Rom>, duration: u64) -> f64 {
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
                let gpu_salts_arc: Arc<Vec<&[u8]>> = Arc::new(
                    all_salts[start_idx..end_idx].iter().map(|s| s.as_slice()).collect()
                );
                let salt_refs: Vec<&[u8]> = gpu_salts_arc.iter().copied().collect();
                
                let result = cuda.hash_parallel(&salt_refs, &rom, 8, 256).map_err(|e| format!("{:?}", e));
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


// Test Duration: 30s per config

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
        println!("=== Phase 7 Test 2: Batch Size Sweep for Top 3 ===\n");
        
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
        
        let batch_sizes = vec![
            65_536,   // 65k
            98_304,   // 98k
            131_072,  // 131k (current optimal)
            163_840,  // 164k
            196_608,  // 197k
            229_376,  // 229k
        ];
        
        println!("Testing {} batch sizes for 3 configurations = {} total tests", 
            batch_sizes.len(), batch_sizes.len() * 3);
        println!("Estimated time: ~{} minutes\n", (batch_sizes.len() * 3 * TEST_DURATION as usize) / 60 + 2);
        println!("{}", "=".repeat(80));
        
        // Top 3 configs
        let configs = vec![
            ("Async+Clone", true, false),
            ("Sync+ZeroCopy", false, true),
            ("Async+ZeroCopy", true, true),
        ];
        
        let mut all_results = Vec::new();
        
        for (config_name, is_async, is_zerocopy) in configs {
            println!("\n### Testing: {} ###\n", config_name);
            
            for &batch_per_gpu in &batch_sizes {
                println!("  Batch {}: ", batch_per_gpu);
                
                let hashrate = if is_async {
                    if is_zerocopy {
                        run_async_zerocopy(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION)
                    } else {
                        run_async_clone(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION)
                    }
                } else {
                    run_sync_zerocopy(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION)
                };
                
                println!("    {:.2} H/s", hashrate);
                all_results.push((config_name, batch_per_gpu, hashrate));
                
                // Cooling period
                thread::sleep(Duration::from_secs(5));
            }
        }
        
        println!("\n{}", "=".repeat(80));
        println!("\n=== Complete Results ===\n");
        
        println!("{:<20} {:>12} {:>15}", "Configuration", "Batch/GPU", "Hashrate (H/s)");
        println!("{}", "-".repeat(50));
        
        let mut by_config: std::collections::HashMap<&str, Vec<(usize, f64)>> = std::collections::HashMap::new();
        for (config, batch, hashrate) in &all_results {
            by_config.entry(*config).or_insert_with(Vec::new).push((*batch, *hashrate));
        }
        
        for config_name in ["Async+Clone", "Sync+ZeroCopy", "Async+ZeroCopy"] {
            if let Some(results) = by_config.get(config_name) {
                println!("\n{}:", config_name);
                let max_hashrate = results.iter().map(|(_, h)| *h).fold(0.0f64, f64::max);
                for (batch, hashrate) in results {
                    let marker = if *hashrate == max_hashrate { " ✅ BEST" } else { "" };
                    println!("  {:>12}  {:>15.2}{}", batch, hashrate, marker);
                }
            }
        }
        
        println!("\n{}", "=".repeat(80));
        println!("\n=== Top 3 Overall ===\n");
        
        let mut sorted = all_results.clone();
        sorted.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
        
        for (i, (config, batch, hashrate)) in sorted.iter().take(3).enumerate() {
            println!("{}. {} @ {} batch: {:.2} H/s", i+1, config, batch, hashrate);
        }
        
        let (best_config, best_batch, best_hashrate) = &sorted[0];
        println!("\n🎯 Optimal: {} @ {} batch/GPU = {:.2} H/s", 
            best_config, best_batch, best_hashrate);
    }
}

#[cfg(feature = "cuda")]
fn run_async_clone(gpu_count: usize, batch_per_gpu: usize, rom: Arc<Rom>, duration: u64) -> f64 {
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
                
                let result = cuda.hash_parallel(&salt_refs, &rom, 8, 256).map_err(|e| format!("{:?}", e));
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

#[cfg(feature = "cuda")]
fn run_async_zerocopy(gpu_count: usize, batch_per_gpu: usize, rom: Arc<Rom>, duration: u64) -> f64 {
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
                let gpu_salts_arc: Arc<Vec<&[u8]>> = Arc::new(
                    all_salts[start_idx..end_idx].iter().map(|s| s.as_slice()).collect()
                );
                let salt_refs: Vec<&[u8]> = gpu_salts_arc.iter().copied().collect();
                
                let result = cuda.hash_parallel(&salt_refs, &rom, 8, 256).map_err(|e| format!("{:?}", e));
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

#[cfg(feature = "cuda")]
fn run_sync_zerocopy(gpu_count: usize, batch_per_gpu: usize, rom: Arc<Rom>, duration: u64) -> f64 {
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
                let gpu_salts_arc: Arc<Vec<&[u8]>> = Arc::new(
                    all_salts[start_idx..end_idx].iter().map(|s| s.as_slice()).collect()
                );
                let salt_refs: Vec<&[u8]> = gpu_salts_arc.iter().copied().collect();
                
                let result = cuda.hash_parallel(&salt_refs, &rom, 8, 256).map_err(|e| format!("{:?}", e));
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

