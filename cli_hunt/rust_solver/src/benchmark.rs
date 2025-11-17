use crate::gpu::{CudaAshmaize, GpuResult};
use crate::{hash_structure_good, init_rom};
use ashmaize::b2::hash as cpu_hash;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub address: String,
    pub challenge_id: String,
    pub difficulty: String,
    pub no_pre_mine: String,
    pub latest_submission: String,
    pub no_pre_mine_hour: String,
    pub duration_secs: u64,
    pub batch_size: usize,
}

#[derive(Debug)]
pub struct BenchmarkResult {
    pub total_hashes: u64,
    pub duration: Duration,
    pub hashes_per_second: f64,
    pub found_solution: bool,
    pub solution_nonce: Option<u64>,
}

impl BenchmarkResult {
    pub fn print_summary(&self, label: &str) {
        println!("\n{} Benchmark Results:", label);
        println!("  Total Hashes: {}", self.total_hashes);
        println!("  Duration: {:.2}s", self.duration.as_secs_f64());
        println!("  Hash Rate: {:.2} H/s", self.hashes_per_second);
        if self.found_solution {
            println!(
                "  Solution Found: {:016x}",
                self.solution_nonce.unwrap()
            );
        } else {
            println!("  Solution Found: No");
        }
    }
}

pub fn benchmark_gpu_hashrate(config: &BenchmarkConfig) -> GpuResult<BenchmarkResult> {
    let rom = init_rom(&config.no_pre_mine);
    let cuda = CudaAshmaize::new()?;

    println!("\nGPU Device Info:");
    println!("{}", cuda.get_device_info()?);

    let difficulty_mask = u32::from_str_radix(&config.difficulty, 16).unwrap();

    let suffix = format!(
        "{}{}{}{}{}{}",
        config.address,
        config.challenge_id,
        config.difficulty,
        config.no_pre_mine,
        config.latest_submission,
        config.no_pre_mine_hour
    );

    let mut total_hashes = 0u64;
    let mut found_solution = false;
    let mut solution_nonce = None;
    let start_time = Instant::now();
    let duration_target = Duration::from_secs(config.duration_secs);

    let mut nonce_base = 0u64;

    while start_time.elapsed() < duration_target && !found_solution {
        let mut salts = Vec::new();
        let mut nonces = Vec::new();

        for i in 0..config.batch_size {
            let nonce = nonce_base + i as u64;
            let preimage = format!("{:016x}{}", nonce, suffix);
            salts.push(preimage.into_bytes());
            nonces.push(nonce);
        }

        let salt_refs: Vec<&[u8]> = salts.iter().map(|v| v.as_slice()).collect();
        
        let results = cuda.hash_parallel(&salt_refs, &rom, 8, 256)?;

        for (i, hash_result) in results.iter().enumerate() {
            total_hashes += 1;
            
            if hash_structure_good(hash_result, difficulty_mask) {
                found_solution = true;
                solution_nonce = Some(nonces[i]);
                break;
            }
        }

        nonce_base += config.batch_size as u64;
    }

    let duration = start_time.elapsed();
    let hashes_per_second = total_hashes as f64 / duration.as_secs_f64();

    Ok(BenchmarkResult {
        total_hashes,
        duration,
        hashes_per_second,
        found_solution,
        solution_nonce,
    })
}

pub fn benchmark_cpu_hashrate(config: &BenchmarkConfig) -> BenchmarkResult {
    let rom = init_rom(&config.no_pre_mine);
    let difficulty_mask = u32::from_str_radix(&config.difficulty, 16).unwrap();

    let suffix = format!(
        "{}{}{}{}{}{}",
        config.address,
        config.challenge_id,
        config.difficulty,
        config.no_pre_mine,
        config.latest_submission,
        config.no_pre_mine_hour
    );

    let mut total_hashes = 0u64;
    let mut found_solution = false;
    let mut solution_nonce = None;
    let start_time = Instant::now();
    let duration_target = Duration::from_secs(config.duration_secs);

    let mut nonce = 0u64;

    while start_time.elapsed() < duration_target && !found_solution {
        let preimage = format!("{:016x}{}", nonce, suffix);
        let hash_result = cpu_hash(preimage.as_bytes(), &rom, 8, 256);
        
        total_hashes += 1;

        if hash_structure_good(&hash_result, difficulty_mask) {
            found_solution = true;
            solution_nonce = Some(nonce);
            break;
        }

        nonce += 1;
    }

    let duration = start_time.elapsed();
    let hashes_per_second = total_hashes as f64 / duration.as_secs_f64();

    BenchmarkResult {
        total_hashes,
        duration,
        hashes_per_second,
        found_solution,
        solution_nonce,
    }
}

pub fn benchmark_cpu_vs_gpu(config: &BenchmarkConfig) -> GpuResult<()> {
    println!("\n=== CPU vs GPU Benchmark ===");
    println!("Running each benchmark for {} seconds...\n", config.duration_secs);

    println!("Running CPU benchmark...");
    let cpu_result = benchmark_cpu_hashrate(config);
    cpu_result.print_summary("CPU");

    println!("\nRunning GPU benchmark...");
    let gpu_result = benchmark_gpu_hashrate(config)?;
    gpu_result.print_summary("GPU");

    println!("\n=== Comparison ===");
    let speedup = gpu_result.hashes_per_second / cpu_result.hashes_per_second;
    println!("GPU Speedup: {:.2}x faster than CPU", speedup);

    Ok(())
}

pub fn run_single_hash_test(
    address: &str,
    challenge_id: &str,
    difficulty: &str,
    no_pre_mine: &str,
    latest_submission: &str,
    no_pre_mine_hour: &str,
    nonce: u64,
) -> GpuResult<()> {
    let rom = init_rom(no_pre_mine);
    
    // Try to initialize CUDA, but don't fail if it's not available
    let cuda_result = CudaAshmaize::new();
    let cuda = match cuda_result {
        Ok(c) => Some(c),
        Err(e) => {
            eprintln!("Warning: Could not initialize CUDA: {:?}", e);
            eprintln!("Running CPU-only test...");
            None
        }
    };

    let suffix = format!(
        "{}{}{}{}{}{}",
        address, challenge_id, difficulty, no_pre_mine, latest_submission, no_pre_mine_hour
    );

    let preimage = format!("{:016x}{}", nonce, suffix);
    
    println!("\nTesting single hash with nonce: {:016x}", nonce);
    
    let cpu_result = cpu_hash(preimage.as_bytes(), &rom, 8, 256);
    println!("CPU result: {}", hex::encode(&cpu_result[..8]));

    if let Some(cuda) = cuda {
        match cuda.hash_parallel(&[preimage.as_bytes()], &rom, 8, 256) {
            Ok(gpu_results) => {
                let gpu_result = &gpu_results[0];
                println!("GPU result: {}", hex::encode(&gpu_result[..8]));

                if cpu_result == *gpu_result {
                    println!("✓ Results match!");
                } else {
                    println!("✗ Results DO NOT match!");
                    println!("Full CPU result: {}", hex::encode(&cpu_result));
                    println!("Full GPU result: {}", hex::encode(gpu_result));
                }
            }
            Err(e) => {
                eprintln!("Error computing GPU hash: {:?}", e);
                eprintln!("CPU result only: {}", hex::encode(&cpu_result[..8]));
            }
        }
    } else {
        println!("GPU not available - showing CPU result only");
    }

    Ok(())
}

