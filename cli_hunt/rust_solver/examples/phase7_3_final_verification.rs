// Phase 7 Test 3: Final Verification (60s tests)
// Verify top candidates with rigorous 60s duration
// Tests the apparent top performers from 30s sweep

use ashmaize::{Rom, RomGenerationType};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::sync::mpsc;
use std::thread;
use std::time::{Instant, Duration};

#[cfg(feature = "cuda")]
use ashmaize_solver::gpu::CudaAshmaize;

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * MB;
const TEST_DURATION: u64 = 60;

fn main() {
    #[cfg(not(feature = "cuda"))]
    {
        eprintln!("This example requires the 'cuda' feature");
        std::process::exit(1);
    }

    #[cfg(feature = "cuda")]
    {
        println!("=== Phase 7 Test 3: Final Verification (60s tests) ===\n");
        
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
        
        // Test candidates from 30s sweep + known good 131k
        let test_configs = vec![
            ("Async+Clone", 131_072, true, false),   // Known good from Phase 6
            ("Async+Clone", 229_376, true, false),   // Best from 30s test
            ("Async+ZeroCopy", 98_304, true, true),  // Second from 30s test
            ("Sync+ZeroCopy", 163_840, false, true), // Third from 30s test
        ];
        
        println!("Testing {} configurations x 60s each = {} total", 
            test_configs.len(), test_configs.len() * TEST_DURATION as usize);
        println!("Estimated time: ~{} minutes\n", test_configs.len() * (TEST_DURATION as usize + 5) / 60);
        println!("{}", "=".repeat(80));
        
        let mut results = Vec::new();
        
        for (config_name, batch_per_gpu, is_async, is_zerocopy) in test_configs {
            println!("\n[Test] {} @ {} batch/GPU (60s)...", config_name, batch_per_gpu);
            
            let hashrate = if is_async {
                if is_zerocopy {
                    run_async_zerocopy(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION)
                } else {
                    run_async_clone(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION)
                }
            } else {
                run_sync_zerocopy(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION)
            };
            
            println!("  Result: {:.2} H/s", hashrate);
            results.push((config_name, batch_per_gpu, hashrate));
            
            // Cooling period
            println!("  Cooling for 5s...");
            thread::sleep(Duration::from_secs(5));
        }
        
        println!("\n{}", "=".repeat(80));
        println!("\n=== Final Verification Results (60s tests) ===\n");
        
        println!("{:<20} {:>12} {:>15}", "Configuration", "Batch/GPU", "Hashrate (H/s)");
        println!("{}", "-".repeat(50));
        
        let mut sorted = results.clone();
        sorted.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
        
        let max_hashrate = sorted[0].2;
        
        for (config, batch, hashrate) in &sorted {
            let marker = if *hashrate == max_hashrate { " ✅ BEST" } else { "" };
            let vs_best = ((hashrate - max_hashrate) / max_hashrate) * 100.0;
            println!("{:<20} {:>12}  {:>15.2} ({:>+6.2}%){}", 
                config, batch, hashrate, vs_best, marker);
        }
        
        println!("\n{}", "=".repeat(80));
        
        let (best_config, best_batch, best_hashrate) = &sorted[0];
        
        println!("\n🎯 FINAL OPTIMAL CONFIGURATION:");
        println!("   {} @ {} batch/GPU", best_config, best_batch);
        println!("   Performance: {:.2} H/s", best_hashrate);
        println!("   Per GPU: {:.2} H/s", best_hashrate / gpu_count as f64);
        
        // Compare to Phase 6 baseline
        let phase6_baseline = 294_000.0;
        let change = ((best_hashrate - phase6_baseline) / phase6_baseline) * 100.0;
        
        println!("\n📊 vs Phase 6 (Async+Clone @ 131k, 60s test):");
        println!("   Phase 6: {:.0} H/s", phase6_baseline);
        println!("   Phase 7: {:.2} H/s", best_hashrate);
        println!("   Change: {:+.2}% ({:+.2} H/s)", change, best_hashrate - phase6_baseline);
        
        if change.abs() < 2.0 {
            println!("\n✅ CONCLUSION: No significant change. Phase 6 optimal configuration confirmed.");
            println!("   Proceed with Async+Clone @ 131k batch/GPU for production.");
        } else if change > 2.0 {
            println!("\n⚠️  CONCLUSION: Significant improvement found!");
            println!("   Update implementation to use {} @ {} batch/GPU.", best_config, best_batch);
        } else {
            println!("\n⚠️  CONCLUSION: Degradation detected.");
            println!("   Keep Phase 6 configuration (Async+Clone @ 131k).");
        }
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


// Tests the apparent top performers from 30s sweep

use ashmaize::{Rom, RomGenerationType};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::sync::mpsc;
use std::thread;
use std::time::{Instant, Duration};

#[cfg(feature = "cuda")]
use ashmaize_solver::gpu::CudaAshmaize;

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * MB;
const TEST_DURATION: u64 = 60;

fn main() {
    #[cfg(not(feature = "cuda"))]
    {
        eprintln!("This example requires the 'cuda' feature");
        std::process::exit(1);
    }

    #[cfg(feature = "cuda")]
    {
        println!("=== Phase 7 Test 3: Final Verification (60s tests) ===\n");
        
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
        
        // Test candidates from 30s sweep + known good 131k
        let test_configs = vec![
            ("Async+Clone", 131_072, true, false),   // Known good from Phase 6
            ("Async+Clone", 229_376, true, false),   // Best from 30s test
            ("Async+ZeroCopy", 98_304, true, true),  // Second from 30s test
            ("Sync+ZeroCopy", 163_840, false, true), // Third from 30s test
        ];
        
        println!("Testing {} configurations x 60s each = {} total", 
            test_configs.len(), test_configs.len() * TEST_DURATION as usize);
        println!("Estimated time: ~{} minutes\n", test_configs.len() * (TEST_DURATION as usize + 5) / 60);
        println!("{}", "=".repeat(80));
        
        let mut results = Vec::new();
        
        for (config_name, batch_per_gpu, is_async, is_zerocopy) in test_configs {
            println!("\n[Test] {} @ {} batch/GPU (60s)...", config_name, batch_per_gpu);
            
            let hashrate = if is_async {
                if is_zerocopy {
                    run_async_zerocopy(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION)
                } else {
                    run_async_clone(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION)
                }
            } else {
                run_sync_zerocopy(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION)
            };
            
            println!("  Result: {:.2} H/s", hashrate);
            results.push((config_name, batch_per_gpu, hashrate));
            
            // Cooling period
            println!("  Cooling for 5s...");
            thread::sleep(Duration::from_secs(5));
        }
        
        println!("\n{}", "=".repeat(80));
        println!("\n=== Final Verification Results (60s tests) ===\n");
        
        println!("{:<20} {:>12} {:>15}", "Configuration", "Batch/GPU", "Hashrate (H/s)");
        println!("{}", "-".repeat(50));
        
        let mut sorted = results.clone();
        sorted.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
        
        let max_hashrate = sorted[0].2;
        
        for (config, batch, hashrate) in &sorted {
            let marker = if *hashrate == max_hashrate { " ✅ BEST" } else { "" };
            let vs_best = ((hashrate - max_hashrate) / max_hashrate) * 100.0;
            println!("{:<20} {:>12}  {:>15.2} ({:>+6.2}%){}", 
                config, batch, hashrate, vs_best, marker);
        }
        
        println!("\n{}", "=".repeat(80));
        
        let (best_config, best_batch, best_hashrate) = &sorted[0];
        
        println!("\n🎯 FINAL OPTIMAL CONFIGURATION:");
        println!("   {} @ {} batch/GPU", best_config, best_batch);
        println!("   Performance: {:.2} H/s", best_hashrate);
        println!("   Per GPU: {:.2} H/s", best_hashrate / gpu_count as f64);
        
        // Compare to Phase 6 baseline
        let phase6_baseline = 294_000.0;
        let change = ((best_hashrate - phase6_baseline) / phase6_baseline) * 100.0;
        
        println!("\n📊 vs Phase 6 (Async+Clone @ 131k, 60s test):");
        println!("   Phase 6: {:.0} H/s", phase6_baseline);
        println!("   Phase 7: {:.2} H/s", best_hashrate);
        println!("   Change: {:+.2}% ({:+.2} H/s)", change, best_hashrate - phase6_baseline);
        
        if change.abs() < 2.0 {
            println!("\n✅ CONCLUSION: No significant change. Phase 6 optimal configuration confirmed.");
            println!("   Proceed with Async+Clone @ 131k batch/GPU for production.");
        } else if change > 2.0 {
            println!("\n⚠️  CONCLUSION: Significant improvement found!");
            println!("   Update implementation to use {} @ {} batch/GPU.", best_config, best_batch);
        } else {
            println!("\n⚠️  CONCLUSION: Degradation detected.");
            println!("   Keep Phase 6 configuration (Async+Clone @ 131k).");
        }
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


// Tests the apparent top performers from 30s sweep

use ashmaize::{Rom, RomGenerationType};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::sync::mpsc;
use std::thread;
use std::time::{Instant, Duration};

#[cfg(feature = "cuda")]
use ashmaize_solver::gpu::CudaAshmaize;

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * MB;
const TEST_DURATION: u64 = 60;

fn main() {
    #[cfg(not(feature = "cuda"))]
    {
        eprintln!("This example requires the 'cuda' feature");
        std::process::exit(1);
    }

    #[cfg(feature = "cuda")]
    {
        println!("=== Phase 7 Test 3: Final Verification (60s tests) ===\n");
        
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
        
        // Test candidates from 30s sweep + known good 131k
        let test_configs = vec![
            ("Async+Clone", 131_072, true, false),   // Known good from Phase 6
            ("Async+Clone", 229_376, true, false),   // Best from 30s test
            ("Async+ZeroCopy", 98_304, true, true),  // Second from 30s test
            ("Sync+ZeroCopy", 163_840, false, true), // Third from 30s test
        ];
        
        println!("Testing {} configurations x 60s each = {} total", 
            test_configs.len(), test_configs.len() * TEST_DURATION as usize);
        println!("Estimated time: ~{} minutes\n", test_configs.len() * (TEST_DURATION as usize + 5) / 60);
        println!("{}", "=".repeat(80));
        
        let mut results = Vec::new();
        
        for (config_name, batch_per_gpu, is_async, is_zerocopy) in test_configs {
            println!("\n[Test] {} @ {} batch/GPU (60s)...", config_name, batch_per_gpu);
            
            let hashrate = if is_async {
                if is_zerocopy {
                    run_async_zerocopy(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION)
                } else {
                    run_async_clone(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION)
                }
            } else {
                run_sync_zerocopy(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION)
            };
            
            println!("  Result: {:.2} H/s", hashrate);
            results.push((config_name, batch_per_gpu, hashrate));
            
            // Cooling period
            println!("  Cooling for 5s...");
            thread::sleep(Duration::from_secs(5));
        }
        
        println!("\n{}", "=".repeat(80));
        println!("\n=== Final Verification Results (60s tests) ===\n");
        
        println!("{:<20} {:>12} {:>15}", "Configuration", "Batch/GPU", "Hashrate (H/s)");
        println!("{}", "-".repeat(50));
        
        let mut sorted = results.clone();
        sorted.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
        
        let max_hashrate = sorted[0].2;
        
        for (config, batch, hashrate) in &sorted {
            let marker = if *hashrate == max_hashrate { " ✅ BEST" } else { "" };
            let vs_best = ((hashrate - max_hashrate) / max_hashrate) * 100.0;
            println!("{:<20} {:>12}  {:>15.2} ({:>+6.2}%){}", 
                config, batch, hashrate, vs_best, marker);
        }
        
        println!("\n{}", "=".repeat(80));
        
        let (best_config, best_batch, best_hashrate) = &sorted[0];
        
        println!("\n🎯 FINAL OPTIMAL CONFIGURATION:");
        println!("   {} @ {} batch/GPU", best_config, best_batch);
        println!("   Performance: {:.2} H/s", best_hashrate);
        println!("   Per GPU: {:.2} H/s", best_hashrate / gpu_count as f64);
        
        // Compare to Phase 6 baseline
        let phase6_baseline = 294_000.0;
        let change = ((best_hashrate - phase6_baseline) / phase6_baseline) * 100.0;
        
        println!("\n📊 vs Phase 6 (Async+Clone @ 131k, 60s test):");
        println!("   Phase 6: {:.0} H/s", phase6_baseline);
        println!("   Phase 7: {:.2} H/s", best_hashrate);
        println!("   Change: {:+.2}% ({:+.2} H/s)", change, best_hashrate - phase6_baseline);
        
        if change.abs() < 2.0 {
            println!("\n✅ CONCLUSION: No significant change. Phase 6 optimal configuration confirmed.");
            println!("   Proceed with Async+Clone @ 131k batch/GPU for production.");
        } else if change > 2.0 {
            println!("\n⚠️  CONCLUSION: Significant improvement found!");
            println!("   Update implementation to use {} @ {} batch/GPU.", best_config, best_batch);
        } else {
            println!("\n⚠️  CONCLUSION: Degradation detected.");
            println!("   Keep Phase 6 configuration (Async+Clone @ 131k).");
        }
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

