// Phase 6: Final Verification - 131k vs 180k
// Longer 60s tests for stable comparison
// Tests both batch sizes back-to-back

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
        println!("=== Phase 6: Final Verification (60s tests) ===\n");
        
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
        
        let configs = vec![
            (131_072, "131k (Phase 5 optimal)"),
            (180_224, "180k (Phase 6 candidate)"),
            (131_072, "131k (repeat)"),
            (180_224, "180k (repeat)"),
        ];
        
        println!("Running 4 tests x 60s each = 240s total...\n");
        println!("{}", "=".repeat(70));
        
        let mut results = Vec::new();
        
        for (i, (batch_per_gpu, label)) in configs.iter().enumerate() {
            println!("\n[Test {}/4] batch_per_gpu = {} ({})...", i+1, batch_per_gpu, label);
            
            let hashrate = run_batch_test(gpu_count, *batch_per_gpu, Arc::clone(&rom), TEST_DURATION);
            
            println!("  Result: {:.2} H/s", hashrate);
            results.push((*batch_per_gpu, label, hashrate));
            
            if i < configs.len() - 1 {
                println!("  Cooling down for 5s...");
                thread::sleep(Duration::from_secs(5));
            }
        }
        
        println!("\n{}", "=".repeat(70));
        println!("\n=== Final Comparison ===\n");
        
        let mut batch_131k: Vec<f64> = Vec::new();
        let mut batch_180k: Vec<f64> = Vec::new();
        
        for (batch, _label, hashrate) in &results {
            if *batch == 131_072 {
                batch_131k.push(*hashrate);
            } else {
                batch_180k.push(*hashrate);
            }
        }
        
        let avg_131k = batch_131k.iter().sum::<f64>() / batch_131k.len() as f64;
        let avg_180k = batch_180k.iter().sum::<f64>() / batch_180k.len() as f64;
        
        println!("131k batch/GPU:");
        for (i, hr) in batch_131k.iter().enumerate() {
            println!("  Run {}: {:.2} H/s", i+1, hr);
        }
        println!("  Average: {:.2} H/s\n", avg_131k);
        
        println!("180k batch/GPU:");
        for (i, hr) in batch_180k.iter().enumerate() {
            println!("  Run {}: {:.2} H/s", i+1, hr);
        }
        println!("  Average: {:.2} H/s\n", avg_180k);
        
        let improvement = ((avg_180k - avg_131k) / avg_131k) * 100.0;
        
        println!("Improvement: {:+.2}% ({:+.2} H/s)", improvement, avg_180k - avg_131k);
        
        if improvement > 2.0 {
            println!("\n✅ SIGNIFICANT IMPROVEMENT - Use 180k batch/GPU");
        } else if improvement > 0.5 {
            println!("\n✅ MINOR IMPROVEMENT - Use 180k batch/GPU");
        } else if improvement > -0.5 {
            println!("\n⚠️  NEUTRAL - Either batch size acceptable");
        } else {
            println!("\n❌ NO IMPROVEMENT - Keep 131k batch/GPU");
        }
    }
}

#[cfg(feature = "cuda")]
fn run_batch_test(
    gpu_count: usize,
    batch_per_gpu: usize,
    rom: Arc<Rom>,
    duration: u64
) -> f64 {
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

// Longer 60s tests for stable comparison
// Tests both batch sizes back-to-back

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
        println!("=== Phase 6: Final Verification (60s tests) ===\n");
        
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
        
        let configs = vec![
            (131_072, "131k (Phase 5 optimal)"),
            (180_224, "180k (Phase 6 candidate)"),
            (131_072, "131k (repeat)"),
            (180_224, "180k (repeat)"),
        ];
        
        println!("Running 4 tests x 60s each = 240s total...\n");
        println!("{}", "=".repeat(70));
        
        let mut results = Vec::new();
        
        for (i, (batch_per_gpu, label)) in configs.iter().enumerate() {
            println!("\n[Test {}/4] batch_per_gpu = {} ({})...", i+1, batch_per_gpu, label);
            
            let hashrate = run_batch_test(gpu_count, *batch_per_gpu, Arc::clone(&rom), TEST_DURATION);
            
            println!("  Result: {:.2} H/s", hashrate);
            results.push((*batch_per_gpu, label, hashrate));
            
            if i < configs.len() - 1 {
                println!("  Cooling down for 5s...");
                thread::sleep(Duration::from_secs(5));
            }
        }
        
        println!("\n{}", "=".repeat(70));
        println!("\n=== Final Comparison ===\n");
        
        let mut batch_131k: Vec<f64> = Vec::new();
        let mut batch_180k: Vec<f64> = Vec::new();
        
        for (batch, _label, hashrate) in &results {
            if *batch == 131_072 {
                batch_131k.push(*hashrate);
            } else {
                batch_180k.push(*hashrate);
            }
        }
        
        let avg_131k = batch_131k.iter().sum::<f64>() / batch_131k.len() as f64;
        let avg_180k = batch_180k.iter().sum::<f64>() / batch_180k.len() as f64;
        
        println!("131k batch/GPU:");
        for (i, hr) in batch_131k.iter().enumerate() {
            println!("  Run {}: {:.2} H/s", i+1, hr);
        }
        println!("  Average: {:.2} H/s\n", avg_131k);
        
        println!("180k batch/GPU:");
        for (i, hr) in batch_180k.iter().enumerate() {
            println!("  Run {}: {:.2} H/s", i+1, hr);
        }
        println!("  Average: {:.2} H/s\n", avg_180k);
        
        let improvement = ((avg_180k - avg_131k) / avg_131k) * 100.0;
        
        println!("Improvement: {:+.2}% ({:+.2} H/s)", improvement, avg_180k - avg_131k);
        
        if improvement > 2.0 {
            println!("\n✅ SIGNIFICANT IMPROVEMENT - Use 180k batch/GPU");
        } else if improvement > 0.5 {
            println!("\n✅ MINOR IMPROVEMENT - Use 180k batch/GPU");
        } else if improvement > -0.5 {
            println!("\n⚠️  NEUTRAL - Either batch size acceptable");
        } else {
            println!("\n❌ NO IMPROVEMENT - Keep 131k batch/GPU");
        }
    }
}

#[cfg(feature = "cuda")]
fn run_batch_test(
    gpu_count: usize,
    batch_per_gpu: usize,
    rom: Arc<Rom>,
    duration: u64
) -> f64 {
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

// Longer 60s tests for stable comparison
// Tests both batch sizes back-to-back

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
        println!("=== Phase 6: Final Verification (60s tests) ===\n");
        
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
        
        let configs = vec![
            (131_072, "131k (Phase 5 optimal)"),
            (180_224, "180k (Phase 6 candidate)"),
            (131_072, "131k (repeat)"),
            (180_224, "180k (repeat)"),
        ];
        
        println!("Running 4 tests x 60s each = 240s total...\n");
        println!("{}", "=".repeat(70));
        
        let mut results = Vec::new();
        
        for (i, (batch_per_gpu, label)) in configs.iter().enumerate() {
            println!("\n[Test {}/4] batch_per_gpu = {} ({})...", i+1, batch_per_gpu, label);
            
            let hashrate = run_batch_test(gpu_count, *batch_per_gpu, Arc::clone(&rom), TEST_DURATION);
            
            println!("  Result: {:.2} H/s", hashrate);
            results.push((*batch_per_gpu, label, hashrate));
            
            if i < configs.len() - 1 {
                println!("  Cooling down for 5s...");
                thread::sleep(Duration::from_secs(5));
            }
        }
        
        println!("\n{}", "=".repeat(70));
        println!("\n=== Final Comparison ===\n");
        
        let mut batch_131k: Vec<f64> = Vec::new();
        let mut batch_180k: Vec<f64> = Vec::new();
        
        for (batch, _label, hashrate) in &results {
            if *batch == 131_072 {
                batch_131k.push(*hashrate);
            } else {
                batch_180k.push(*hashrate);
            }
        }
        
        let avg_131k = batch_131k.iter().sum::<f64>() / batch_131k.len() as f64;
        let avg_180k = batch_180k.iter().sum::<f64>() / batch_180k.len() as f64;
        
        println!("131k batch/GPU:");
        for (i, hr) in batch_131k.iter().enumerate() {
            println!("  Run {}: {:.2} H/s", i+1, hr);
        }
        println!("  Average: {:.2} H/s\n", avg_131k);
        
        println!("180k batch/GPU:");
        for (i, hr) in batch_180k.iter().enumerate() {
            println!("  Run {}: {:.2} H/s", i+1, hr);
        }
        println!("  Average: {:.2} H/s\n", avg_180k);
        
        let improvement = ((avg_180k - avg_131k) / avg_131k) * 100.0;
        
        println!("Improvement: {:+.2}% ({:+.2} H/s)", improvement, avg_180k - avg_131k);
        
        if improvement > 2.0 {
            println!("\n✅ SIGNIFICANT IMPROVEMENT - Use 180k batch/GPU");
        } else if improvement > 0.5 {
            println!("\n✅ MINOR IMPROVEMENT - Use 180k batch/GPU");
        } else if improvement > -0.5 {
            println!("\n⚠️  NEUTRAL - Either batch size acceptable");
        } else {
            println!("\n❌ NO IMPROVEMENT - Keep 131k batch/GPU");
        }
    }
}

#[cfg(feature = "cuda")]
fn run_batch_test(
    gpu_count: usize,
    batch_per_gpu: usize,
    rom: Arc<Rom>,
    duration: u64
) -> f64 {
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




