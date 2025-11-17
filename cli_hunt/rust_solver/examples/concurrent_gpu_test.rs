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
        eprintln!("Run with: cargo run --example concurrent_gpu_test --features cuda --release");
        std::process::exit(1);
    }

    #[cfg(feature = "cuda")]
    {
        println!("=== Concurrent GPU Task Test ===\n");
        println!("Testing if multiple tasks on same GPU run in parallel or serially\n");

        // Initialize GPU 0
        let cuda = match CudaAshmaize::new_with_device(0) {
            Ok(c) => {
                println!("GPU 0 initialized: {}", c.get_device_info().unwrap());
                Arc::new(c)
            }
            Err(e) => {
                eprintln!("Failed to initialize GPU 0: {:?}", e);
                std::process::exit(1);
            }
        };

        // Setup ROM
        let rom = Arc::new(Rom::new(
            b"0",
            RomGenerationType::TwoStep {
                pre_size: 16 * MB,
                mixing_numbers: 4,
            },
            1 * GB,
        ));

        let batch_size = 4096;
        let test_duration = 20; // seconds

        // Generate test salts
        let mut salts = Vec::new();
        for i in 0..batch_size {
            salts.push(format!("test_{}", i).into_bytes());
        }
        let salts = Arc::new(salts);

        println!("Test Configuration:");
        println!("  Batch size: {}", batch_size);
        println!("  Duration: {}s\n", test_duration);

        // Test 1: Single task on GPU 0
        println!("Test 1: Single task on GPU 0...");
        let single_hashrate = test_single_task(
            Arc::clone(&cuda),
            Arc::clone(&rom),
            Arc::clone(&salts),
            batch_size,
            test_duration,
        );
        println!("  Single task hashrate: {:.2} H/s\n", single_hashrate);

        // Test 2: Two concurrent tasks on same GPU 0
        println!("Test 2: Two concurrent tasks on GPU 0...");
        let concurrent_hashrate = test_concurrent_tasks(
            Arc::clone(&cuda),
            Arc::clone(&rom),
            Arc::clone(&salts),
            batch_size,
            test_duration,
        );
        println!("  Concurrent aggregate hashrate: {:.2} H/s", concurrent_hashrate.0);
        println!("  Task 1: {:.2} H/s", concurrent_hashrate.1);
        println!("  Task 2: {:.2} H/s\n", concurrent_hashrate.2);

        // Analysis
        println!("=== Analysis ===");
        let efficiency = (concurrent_hashrate.0 / single_hashrate) * 100.0;
        println!("Concurrent vs Single: {:.1}%", efficiency);
        
        if efficiency > 95.0 {
            println!("Result: Tasks run in PARALLEL (GPU has concurrent execution)");
        } else if efficiency > 80.0 {
            println!("Result: Tasks run MOSTLY in parallel (some serialization)");
        } else if efficiency > 60.0 {
            println!("Result: Tasks run PARTIALLY serialized (significant overhead)");
        } else {
            println!("Result: Tasks run SERIALLY (one waits for other)");
        }
        
        println!("\nConclusion:");
        if efficiency < 80.0 {
            println!("  Running multiple tasks on same GPU is INEFFICIENT");
            println!("  Recommendation: Use separate GPUs for separate tasks");
        } else {
            println!("  Running multiple tasks on same GPU is VIABLE");
            println!("  Recommendation: Can use for work subdivision");
        }
    }
}

#[cfg(feature = "cuda")]
fn test_single_task(
    cuda: Arc<CudaAshmaize>,
    rom: Arc<Rom>,
    salts: Arc<Vec<Vec<u8>>>,
    batch_size: usize,
    duration: u64,
) -> f64 {
    let start = Instant::now();
    let mut total_hashes = 0u64;

    while start.elapsed().as_secs() < duration {
        let salt_refs: Vec<&[u8]> = salts.iter().map(|s| s.as_slice()).collect();
        match cuda.hash_parallel(&salt_refs, &rom, 8, 256) {
            Ok(_) => total_hashes += batch_size as u64,
            Err(e) => {
                eprintln!("  Error: {:?}", e);
                break;
            }
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    total_hashes as f64 / elapsed
}

#[cfg(feature = "cuda")]
fn test_concurrent_tasks(
    cuda: Arc<CudaAshmaize>,
    rom: Arc<Rom>,
    salts: Arc<Vec<Vec<u8>>>,
    batch_size: usize,
    duration: u64,
) -> (f64, f64, f64) {
    use std::sync::atomic::{AtomicU64, Ordering};
    
    let total_hashes_1 = Arc::new(AtomicU64::new(0));
    let total_hashes_2 = Arc::new(AtomicU64::new(0));
    let start = Arc::new(Instant::now());

    let cuda1 = Arc::clone(&cuda);
    let rom1 = Arc::clone(&rom);
    let salts1 = Arc::clone(&salts);
    let start1 = Arc::clone(&start);
    let hashes1 = Arc::clone(&total_hashes_1);

    let cuda2 = Arc::clone(&cuda);
    let rom2 = Arc::clone(&rom);
    let salts2 = Arc::clone(&salts);
    let start2 = Arc::clone(&start);
    let hashes2 = Arc::clone(&total_hashes_2);

    // Task 1
    let handle1 = thread::spawn(move || {
        while start1.elapsed().as_secs() < duration {
            let salt_refs: Vec<&[u8]> = salts1.iter().map(|s| s.as_slice()).collect();
            match cuda1.hash_parallel(&salt_refs, &rom1, 8, 256) {
                Ok(_) => hashes1.fetch_add(batch_size as u64, Ordering::Relaxed),
                Err(_) => break,
            };
        }
    });

    // Task 2
    let handle2 = thread::spawn(move || {
        while start2.elapsed().as_secs() < duration {
            let salt_refs: Vec<&[u8]> = salts2.iter().map(|s| s.as_slice()).collect();
            match cuda2.hash_parallel(&salt_refs, &rom2, 8, 256) {
                Ok(_) => hashes2.fetch_add(batch_size as u64, Ordering::Relaxed),
                Err(_) => break,
            };
        }
    });

    handle1.join().unwrap();
    handle2.join().unwrap();

    let elapsed = start.elapsed().as_secs_f64();
    let h1 = total_hashes_1.load(Ordering::Relaxed);
    let h2 = total_hashes_2.load(Ordering::Relaxed);
    
    let hashrate_1 = h1 as f64 / elapsed;
    let hashrate_2 = h2 as f64 / elapsed;
    let aggregate = (h1 + h2) as f64 / elapsed;

    (aggregate, hashrate_1, hashrate_2)
}

