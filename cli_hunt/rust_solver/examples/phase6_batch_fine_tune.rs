// Phase 6: Fine-Tune Batch Size Around 197k
// Explore 180k-230k range to find precise optimal
// Test Duration: 25s per config for more accuracy

use ashmaize::{Rom, RomGenerationType};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::sync::mpsc;
use std::thread;
use std::time::{Instant, Duration};

#[cfg(feature = "cuda")]
use ashmaize_solver::gpu::CudaAshmaize;

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * MB;
const TEST_DURATION: u64 = 25;

fn main() {
    #[cfg(not(feature = "cuda"))]
    {
        eprintln!("This example requires the 'cuda' feature");
        std::process::exit(1);
    }

    #[cfg(feature = "cuda")]
    {
        println!("=== Phase 6: Fine-Tune Batch Size (Around 197k) ===\n");
        
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
        
        // Fine-tune around 197k
        let batch_sizes = vec![
            (180_224, "180k"),
            (196_608, "197k (best from prev test)"),
            (229_376, "229k"),
            (245_760, "246k"),
        ];
        
        println!("Testing {} configurations (25s each)...\n", batch_sizes.len());
        println!("{}", "=".repeat(70));
        
        let mut results = Vec::new();
        
        for (batch_per_gpu, label) in batch_sizes {
            println!("\nTesting batch_per_gpu = {} ({})...", batch_per_gpu, label);
            
            let hashrate = run_batch_test(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION);
            
            println!("  Result: {:.2} H/s", hashrate);
            results.push((batch_per_gpu, label, hashrate));
        }
        
        println!("\n{}", "=".repeat(70));
        println!("\n=== Fine-Tuned Results ===\n");
        
        let max_hashrate = results.iter().map(|(_, _, h)| *h).fold(0.0f64, f64::max);
        let baseline_131k = 281_694.0;  // From previous test
        
        println!("{:<12} {:<35} {:>15} {:>12}",
            "Batch/GPU", "Label", "Hashrate (H/s)", "vs 131k");
        println!("{}", "-".repeat(80));
        
        for (batch, label, hashrate) in &results {
            let vs_131k = ((hashrate - baseline_131k) / baseline_131k) * 100.0;
            let marker = if *hashrate == max_hashrate { " ✅ BEST" } else { "" };
            
            println!("{:<12} {:<35} {:>15.2} {:>11.1}%{}",
                batch, label, hashrate, vs_131k, marker);
        }
        
        let (best_batch, best_label, best_hashrate) = results.iter()
            .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap())
            .unwrap();
        
        println!("\n🎯 Optimal Configuration: {} batch/GPU ({})",
            best_batch, best_label);
        println!("   Performance: {:.2} H/s aggregate", best_hashrate);
        println!("   Per GPU: {:.2} H/s", best_hashrate / gpu_count as f64);
        
        let improvement_vs_131k = ((best_hashrate - baseline_131k) / baseline_131k) * 100.0;
        println!("   Improvement over 131k: {:+.2}%", improvement_vs_131k);
        
        println!("\n✅ New optimal batch size confirmed!");
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

// Explore 180k-230k range to find precise optimal
// Test Duration: 25s per config for more accuracy

use ashmaize::{Rom, RomGenerationType};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::sync::mpsc;
use std::thread;
use std::time::{Instant, Duration};

#[cfg(feature = "cuda")]
use ashmaize_solver::gpu::CudaAshmaize;

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * MB;
const TEST_DURATION: u64 = 25;

fn main() {
    #[cfg(not(feature = "cuda"))]
    {
        eprintln!("This example requires the 'cuda' feature");
        std::process::exit(1);
    }

    #[cfg(feature = "cuda")]
    {
        println!("=== Phase 6: Fine-Tune Batch Size (Around 197k) ===\n");
        
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
        
        // Fine-tune around 197k
        let batch_sizes = vec![
            (180_224, "180k"),
            (196_608, "197k (best from prev test)"),
            (229_376, "229k"),
            (245_760, "246k"),
        ];
        
        println!("Testing {} configurations (25s each)...\n", batch_sizes.len());
        println!("{}", "=".repeat(70));
        
        let mut results = Vec::new();
        
        for (batch_per_gpu, label) in batch_sizes {
            println!("\nTesting batch_per_gpu = {} ({})...", batch_per_gpu, label);
            
            let hashrate = run_batch_test(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION);
            
            println!("  Result: {:.2} H/s", hashrate);
            results.push((batch_per_gpu, label, hashrate));
        }
        
        println!("\n{}", "=".repeat(70));
        println!("\n=== Fine-Tuned Results ===\n");
        
        let max_hashrate = results.iter().map(|(_, _, h)| *h).fold(0.0f64, f64::max);
        let baseline_131k = 281_694.0;  // From previous test
        
        println!("{:<12} {:<35} {:>15} {:>12}",
            "Batch/GPU", "Label", "Hashrate (H/s)", "vs 131k");
        println!("{}", "-".repeat(80));
        
        for (batch, label, hashrate) in &results {
            let vs_131k = ((hashrate - baseline_131k) / baseline_131k) * 100.0;
            let marker = if *hashrate == max_hashrate { " ✅ BEST" } else { "" };
            
            println!("{:<12} {:<35} {:>15.2} {:>11.1}%{}",
                batch, label, hashrate, vs_131k, marker);
        }
        
        let (best_batch, best_label, best_hashrate) = results.iter()
            .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap())
            .unwrap();
        
        println!("\n🎯 Optimal Configuration: {} batch/GPU ({})",
            best_batch, best_label);
        println!("   Performance: {:.2} H/s aggregate", best_hashrate);
        println!("   Per GPU: {:.2} H/s", best_hashrate / gpu_count as f64);
        
        let improvement_vs_131k = ((best_hashrate - baseline_131k) / baseline_131k) * 100.0;
        println!("   Improvement over 131k: {:+.2}%", improvement_vs_131k);
        
        println!("\n✅ New optimal batch size confirmed!");
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

// Explore 180k-230k range to find precise optimal
// Test Duration: 25s per config for more accuracy

use ashmaize::{Rom, RomGenerationType};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::sync::mpsc;
use std::thread;
use std::time::{Instant, Duration};

#[cfg(feature = "cuda")]
use ashmaize_solver::gpu::CudaAshmaize;

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * MB;
const TEST_DURATION: u64 = 25;

fn main() {
    #[cfg(not(feature = "cuda"))]
    {
        eprintln!("This example requires the 'cuda' feature");
        std::process::exit(1);
    }

    #[cfg(feature = "cuda")]
    {
        println!("=== Phase 6: Fine-Tune Batch Size (Around 197k) ===\n");
        
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
        
        // Fine-tune around 197k
        let batch_sizes = vec![
            (180_224, "180k"),
            (196_608, "197k (best from prev test)"),
            (229_376, "229k"),
            (245_760, "246k"),
        ];
        
        println!("Testing {} configurations (25s each)...\n", batch_sizes.len());
        println!("{}", "=".repeat(70));
        
        let mut results = Vec::new();
        
        for (batch_per_gpu, label) in batch_sizes {
            println!("\nTesting batch_per_gpu = {} ({})...", batch_per_gpu, label);
            
            let hashrate = run_batch_test(gpu_count, batch_per_gpu, Arc::clone(&rom), TEST_DURATION);
            
            println!("  Result: {:.2} H/s", hashrate);
            results.push((batch_per_gpu, label, hashrate));
        }
        
        println!("\n{}", "=".repeat(70));
        println!("\n=== Fine-Tuned Results ===\n");
        
        let max_hashrate = results.iter().map(|(_, _, h)| *h).fold(0.0f64, f64::max);
        let baseline_131k = 281_694.0;  // From previous test
        
        println!("{:<12} {:<35} {:>15} {:>12}",
            "Batch/GPU", "Label", "Hashrate (H/s)", "vs 131k");
        println!("{}", "-".repeat(80));
        
        for (batch, label, hashrate) in &results {
            let vs_131k = ((hashrate - baseline_131k) / baseline_131k) * 100.0;
            let marker = if *hashrate == max_hashrate { " ✅ BEST" } else { "" };
            
            println!("{:<12} {:<35} {:>15.2} {:>11.1}%{}",
                batch, label, hashrate, vs_131k, marker);
        }
        
        let (best_batch, best_label, best_hashrate) = results.iter()
            .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap())
            .unwrap();
        
        println!("\n🎯 Optimal Configuration: {} batch/GPU ({})",
            best_batch, best_label);
        println!("   Performance: {:.2} H/s aggregate", best_hashrate);
        println!("   Per GPU: {:.2} H/s", best_hashrate / gpu_count as f64);
        
        let improvement_vs_131k = ((best_hashrate - baseline_131k) / baseline_131k) * 100.0;
        println!("   Improvement over 131k: {:+.2}%", improvement_vs_131k);
        
        println!("\n✅ New optimal batch size confirmed!");
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




