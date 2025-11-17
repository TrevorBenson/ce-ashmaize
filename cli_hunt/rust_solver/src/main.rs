// use ashmaize::{hash, Rom, RomGenerationType};
use ashmaize::b2::hash; // Use the blake2 implementation (slightly faster)
use ashmaize::{Rom, RomGenerationType};
use clap::Parser;
use rayon::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

const NUM_THREADS: u64 = 5;
pub const MB: usize = 1024 * 1024;
pub const GB: usize = 1024 * MB;

#[cfg(feature = "cuda")]
#[derive(clap::ValueEnum, Clone, Debug)]
enum SolverMode {
    Cpu,
    Gpu,
    Auto,
    Mixed,
}

mod tests;

#[cfg(feature = "cuda")]
mod gpu;

#[cfg(feature = "cuda")]
mod benchmark;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
    
    #[arg(long)]
    address: Option<String>,
    #[arg(long)]
    challenge_id: Option<String>,
    #[arg(long)]
    difficulty: Option<String>,
    #[arg(long)]
    no_pre_mine: Option<String>,
    #[arg(long)]
    latest_submission: Option<String>,
    #[arg(long)]
    no_pre_mine_hour: Option<String>,
    
    #[cfg(feature = "cuda")]
    #[arg(long, default_value = "false")]
    use_gpu: bool,
    
    #[cfg(feature = "cuda")]
    #[arg(long, default_value = "0")]
    gpu_id: usize,
    
    #[cfg(feature = "cuda")]
    #[arg(long, default_value = "131072")]
    gpu_batch_size: usize,
    
    #[cfg(feature = "cuda")]
    #[arg(long, value_enum, default_value = "auto")]
    solver_mode: SolverMode,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    #[cfg(feature = "cuda")]
    Benchmark {
        #[arg(long)]
        address: String,
        #[arg(long)]
        challenge_id: String,
        #[arg(long)]
        difficulty: String,
        #[arg(long)]
        no_pre_mine: String,
        #[arg(long)]
        latest_submission: String,
        #[arg(long)]
        no_pre_mine_hour: String,
        #[arg(long, default_value = "10")]
        duration_secs: u64,
        #[arg(long, default_value = "1024")]
        batch_size: usize,
        #[arg(long, default_value = "false")]
        compare: bool,
    },
    #[cfg(feature = "cuda")]
    TestHash {
        #[arg(long)]
        address: String,
        #[arg(long)]
        challenge_id: String,
        #[arg(long)]
        difficulty: String,
        #[arg(long)]
        no_pre_mine: String,
        #[arg(long)]
        latest_submission: String,
        #[arg(long)]
        no_pre_mine_hour: String,
        #[arg(long)]
        nonce: String,
    },
}

pub fn hash_structure_good(hash: &[u8], difficulty_mask: u32) -> bool {
    if hash.len() < 4 {
        return false; // Not enough bytes to apply a u32 mask
    }

    let hash_prefix = u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]]);
    (hash_prefix & !difficulty_mask) == 0
}

pub fn init_rom(no_pre_mine_hex: &str) -> Rom {
    Rom::new(
        no_pre_mine_hex.as_bytes(),
        RomGenerationType::TwoStep {
            pre_size: 16 * MB,
            mixing_numbers: 4,
        },
        1 * GB,
    )
}

fn main() {
    let args = Args::parse();

    match args.command {
        #[cfg(feature = "cuda")]
        Some(Command::Benchmark {
            address,
            challenge_id,
            difficulty,
            no_pre_mine,
            latest_submission,
            no_pre_mine_hour,
            duration_secs,
            batch_size,
            compare,
        }) => {
            let config = benchmark::BenchmarkConfig {
                address,
                challenge_id,
                difficulty,
                no_pre_mine,
                latest_submission,
                no_pre_mine_hour,
                duration_secs,
                batch_size,
            };

            if compare {
                benchmark::benchmark_cpu_vs_gpu(&config).unwrap();
            } else {
                let result = benchmark::benchmark_gpu_hashrate(&config).unwrap();
                result.print_summary("GPU");
            }
        }
        #[cfg(feature = "cuda")]
        Some(Command::TestHash {
            address,
            challenge_id,
            difficulty,
            no_pre_mine,
            latest_submission,
            no_pre_mine_hour,
            nonce,
        }) => {
            let nonce_val = u64::from_str_radix(&nonce, 16).unwrap();
            if let Err(e) = benchmark::run_single_hash_test(
                &address,
                &challenge_id,
                &difficulty,
                &no_pre_mine,
                &latest_submission,
                &no_pre_mine_hour,
                nonce_val,
            ) {
                eprintln!("Error during hash test: {:?}", e);
                std::process::exit(1);
            }
        }
        None => {
            let address = args.address.expect("--address required");
            let challenge_id = args.challenge_id.expect("--challenge_id required");
            let difficulty = args.difficulty.expect("--difficulty required");
            let no_pre_mine = args.no_pre_mine.expect("--no_pre_mine required");
            let latest_submission = args.latest_submission.expect("--latest_submission required");
            let no_pre_mine_hour = args.no_pre_mine_hour.expect("--no_pre_mine_hour required");

            #[cfg(feature = "cuda")]
            let use_gpu = args.use_gpu;
            #[cfg(feature = "cuda")]
            let gpu_id = args.gpu_id;
            #[cfg(feature = "cuda")]
            let gpu_batch_size = args.gpu_batch_size;
            #[cfg(feature = "cuda")]
            let solver_mode = args.solver_mode;

            solve(
                &address,
                &challenge_id,
                &difficulty,
                &no_pre_mine,
                &latest_submission,
                &no_pre_mine_hour,
                #[cfg(feature = "cuda")]
                use_gpu,
                #[cfg(feature = "cuda")]
                gpu_id,
                #[cfg(feature = "cuda")]
                gpu_batch_size,
                #[cfg(feature = "cuda")]
                solver_mode,
            );
        }
    }
}

fn solve(
    address: &str,
    challenge_id: &str,
    difficulty: &str,
    no_pre_mine: &str,
    latest_submission: &str,
    no_pre_mine_hour: &str,
    #[cfg(feature = "cuda")] use_gpu: bool,
    #[cfg(feature = "cuda")] gpu_id: usize,
    #[cfg(feature = "cuda")] gpu_batch_size: usize,
    #[cfg(feature = "cuda")] solver_mode: SolverMode,
) {
    let rom = init_rom(no_pre_mine);
    let difficulty_mask = u32::from_str_radix(difficulty, 16).unwrap();

    let suffix = format!(
        "{}{}{}{}{}{}",
        address, challenge_id, difficulty, no_pre_mine, latest_submission, no_pre_mine_hour
    );

    #[cfg(feature = "cuda")]
    {
        if use_gpu {
            solve_with_gpu(&rom, &suffix, difficulty_mask, gpu_batch_size, gpu_id);
            return;
        }

        match solver_mode {
            SolverMode::Cpu => {
                eprintln!("Solver mode: CPU-only");
                solve_cpu_only(&rom, &suffix, difficulty_mask);
            }
            SolverMode::Gpu => {
                eprintln!("Solver mode: GPU-only (all GPUs)");
                solve_multi_gpu(&rom, &suffix, difficulty_mask, gpu_batch_size);
            }
            SolverMode::Auto => {
                let _init = gpu::CudaAshmaize::new();
                let gpu_count = gpu::CudaAshmaize::get_device_count().unwrap_or(0);
                if gpu_count > 0 {
                    eprintln!("Solver mode: Auto → GPU-only ({} GPUs detected)", gpu_count);
                    solve_multi_gpu(&rom, &suffix, difficulty_mask, gpu_batch_size);
                } else {
                    eprintln!("Solver mode: Auto → CPU-only (no GPUs detected)");
                    solve_cpu_only(&rom, &suffix, difficulty_mask);
                }
            }
            SolverMode::Mixed => {
                eprintln!("Solver mode: Mixed (CPU + all GPUs)");
                solve_mixed(&rom, &suffix, difficulty_mask, gpu_batch_size);
            }
        }
        return;
    }

    #[cfg(not(feature = "cuda"))]
    solve_cpu_only(&rom, &suffix, difficulty_mask);
}

fn solve_cpu_only(rom: &Rom, suffix: &str, difficulty_mask: u32) {
    let rom = Arc::new(rom.clone());
    let found = Arc::new(AtomicBool::new(false));
    let result_nonce = Arc::new(AtomicU64::new(0));
    let start_nonce = 0;

    (0..NUM_THREADS).into_par_iter().for_each(|thread_id| {
        let rom = Arc::clone(&rom);
        let mut local_nonce = start_nonce + thread_id as u64;
        let stride = NUM_THREADS as u64;

        let mut preimage = String::with_capacity(16 + suffix.len());

        while !found.load(Ordering::Relaxed) {
            preimage.clear();
            use std::fmt::Write;
            write!(&mut preimage, "{:016x}{}", local_nonce, &suffix).unwrap();

            let hash_result = hash(preimage.as_bytes(), &rom, 8, 256);

            if hash_structure_good(&hash_result, difficulty_mask) {
                found.store(true, Ordering::Relaxed);
                result_nonce.store(local_nonce, Ordering::Relaxed);
                break;
            }

            local_nonce += stride;
        }
    });

    if found.load(Ordering::Relaxed) {
        println!("{:016x}", result_nonce.load(Ordering::Relaxed));
    }
}

#[cfg(feature = "cuda")]
fn solve_with_gpu(rom: &Rom, suffix: &str, difficulty_mask: u32, batch_size: usize, gpu_id: usize) {
    use std::sync::Mutex;

    let cuda = match gpu::CudaAshmaize::new_with_device(gpu_id) {
        Ok(c) => {
            eprintln!("GPU {} initialized successfully", gpu_id);
            eprintln!("{}", c.get_device_info().unwrap());
            Arc::new(c)
        }
        Err(e) => {
            eprintln!("Failed to initialize GPU {}: {}, falling back to CPU", gpu_id, e);
            solve_cpu_only(rom, suffix, difficulty_mask);
            return;
        }
    };

    let rom = Arc::new(rom.clone());
    let found = Arc::new(AtomicBool::new(false));
    let result_nonce = Arc::new(AtomicU64::new(0));
    let nonce_counter = Arc::new(Mutex::new(0u64));

    let num_workers = 2;

    (0..num_workers).into_par_iter().for_each(|_| {
        let rom = Arc::clone(&rom);
        let cuda = Arc::clone(&cuda);
        let nonce_counter = Arc::clone(&nonce_counter);

        while !found.load(Ordering::Relaxed) {
            let base_nonce = {
                let mut counter = nonce_counter.lock().unwrap();
                let base = *counter;
                *counter += batch_size as u64;
                base
            };

            let mut salts = Vec::new();
            let mut nonces = Vec::new();

            for i in 0..batch_size {
                let nonce = base_nonce + i as u64;
                let preimage = format!("{:016x}{}", nonce, suffix);
                salts.push(preimage.into_bytes());
                nonces.push(nonce);
            }

            let salt_refs: Vec<&[u8]> = salts.iter().map(|v| v.as_slice()).collect();

            match cuda.hash_parallel(&salt_refs, &rom, 8, 256) {
                Ok(results) => {
                    for (i, hash_result) in results.iter().enumerate() {
                        if hash_structure_good(hash_result, difficulty_mask) {
                            found.store(true, Ordering::Relaxed);
                            result_nonce.store(nonces[i], Ordering::Relaxed);
                            break;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("GPU error: {}, batch skipped", e);
                }
            }
        }
    });

    if found.load(Ordering::Relaxed) {
        println!("{:016x}", result_nonce.load(Ordering::Relaxed));
    }
}

#[cfg(feature = "cuda")]
fn solve_multi_gpu(rom: &Rom, suffix: &str, difficulty_mask: u32, batch_size: usize) {
    use std::sync::mpsc;
    use std::thread;

    let _init = match gpu::CudaAshmaize::new() {
        Ok(init) => init,
        Err(e) => {
            eprintln!("Failed to initialize CUDA: {}, falling back to CPU", e);
            solve_cpu_only(rom, suffix, difficulty_mask);
            return;
        }
    };

    let gpu_count = match gpu::CudaAshmaize::get_device_count() {
        Ok(0) => {
            eprintln!("No GPUs detected, falling back to CPU");
            solve_cpu_only(rom, suffix, difficulty_mask);
            return;
        }
        Ok(count) => count,
        Err(e) => {
            eprintln!("Failed to detect GPUs: {}, falling back to CPU", e);
            solve_cpu_only(rom, suffix, difficulty_mask);
            return;
        }
    };

    eprintln!("Initializing {} GPU(s) for mining...", gpu_count);
    
    let rom = Arc::new(rom.clone());
    let found = Arc::new(AtomicBool::new(false));
    let result_nonce = Arc::new(AtomicU64::new(0));
    let nonce_counter = Arc::new(AtomicU64::new(0));

    let (result_tx, result_rx) = mpsc::channel::<(usize, u64, Result<Vec<[u8; 64]>, String>)>();

    let mut handles = Vec::new();
    for gpu_id in 0..gpu_count {
        let rom = Arc::clone(&rom);
        let found = Arc::clone(&found);
        let nonce_counter = Arc::clone(&nonce_counter);
        let result_tx = result_tx.clone();
        let suffix_owned = suffix.to_string();

        let cuda = match gpu::CudaAshmaize::new_with_device(gpu_id) {
            Ok(c) => {
                eprintln!("  GPU {}: {}", gpu_id, c.get_device_info().unwrap());
                Arc::new(c)
            }
            Err(e) => {
                eprintln!("  GPU {}: Failed to initialize ({}), skipping", gpu_id, e);
                continue;
            }
        };

        let handle = thread::spawn(move || {
            while !found.load(Ordering::Relaxed) {
                let base_nonce = nonce_counter.fetch_add(batch_size as u64, Ordering::Relaxed);

                let salts: Vec<Vec<u8>> = (0..batch_size)
                    .map(|i| {
                        let nonce = base_nonce + i as u64;
                        let preimage = format!("{:016x}{}", nonce, suffix_owned);
                        preimage.into_bytes()
                    })
                    .collect();

                let salt_refs: Vec<&[u8]> = salts.iter().map(|s| s.as_slice()).collect();
                
                let result = cuda.hash_parallel(&salt_refs, &rom, 8, 256)
                    .map_err(|e| format!("{:?}", e));
                
                result_tx.send((gpu_id, base_nonce, result)).ok();
            }
        });
        handles.push(handle);
    }
    drop(result_tx);

    let found_clone = Arc::clone(&found);
    let result_nonce_clone = Arc::clone(&result_nonce);
    let collector = thread::spawn(move || {
        while let Ok((gpu_id, base_nonce, result)) = result_rx.recv() {
            match result {
                Ok(hashes) => {
                    for (i, hash_result) in hashes.iter().enumerate() {
                        if hash_structure_good(hash_result, difficulty_mask) {
                            found_clone.store(true, Ordering::Relaxed);
                            result_nonce_clone.store(base_nonce + i as u64, Ordering::Relaxed);
                            return;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("GPU {} error: {}, batch skipped", gpu_id, e);
                }
            }
        }
    });

    for handle in handles {
        handle.join().ok();
    }
    collector.join().ok();

    if found.load(Ordering::Relaxed) {
        println!("{:016x}", result_nonce.load(Ordering::Relaxed));
    }
}

#[cfg(feature = "cuda")]
fn solve_mixed(rom: &Rom, suffix: &str, difficulty_mask: u32, gpu_batch_size: usize) {
    use std::thread;

    let _init = match gpu::CudaAshmaize::new() {
        Ok(init) => init,
        Err(e) => {
            eprintln!("Failed to initialize CUDA: {}, using CPU-only", e);
            solve_cpu_only(rom, suffix, difficulty_mask);
            return;
        }
    };

    let gpu_count = gpu::CudaAshmaize::get_device_count().unwrap_or(0);
    
    if gpu_count == 0 {
        eprintln!("No GPUs detected for mixed mode, using CPU-only");
        solve_cpu_only(rom, suffix, difficulty_mask);
        return;
    }

    eprintln!("Mixed mode: {} CPU thread(s) + {} GPU(s)", NUM_THREADS, gpu_count);
    
    let rom = Arc::new(rom.clone());
    let found = Arc::new(AtomicBool::new(false));
    let result_nonce = Arc::new(AtomicU64::new(0));
    let nonce_counter = Arc::new(AtomicU64::new(0));

    let mut handles = Vec::new();

    for _thread_id in 0..NUM_THREADS {
        let rom = Arc::clone(&rom);
        let found = Arc::clone(&found);
        let result_nonce = Arc::clone(&result_nonce);
        let nonce_counter = Arc::clone(&nonce_counter);
        let suffix_owned = suffix.to_string();

        let handle = thread::spawn(move || {
            let mut preimage = String::with_capacity(16 + suffix_owned.len());
            while !found.load(Ordering::Relaxed) {
                let nonce = nonce_counter.fetch_add(1, Ordering::Relaxed);
                
                preimage.clear();
                use std::fmt::Write;
                write!(&mut preimage, "{:016x}{}", nonce, suffix_owned).unwrap();

                let hash_result = hash(preimage.as_bytes(), &rom, 8, 256);
                
                if hash_structure_good(&hash_result, difficulty_mask) {
                    found.store(true, Ordering::Relaxed);
                    result_nonce.store(nonce, Ordering::Relaxed);
                    return;
                }
            }
        });
        handles.push(handle);
    }

    for gpu_id in 0..gpu_count {
        let rom = Arc::clone(&rom);
        let found = Arc::clone(&found);
        let nonce_counter = Arc::clone(&nonce_counter);
        let result_nonce = Arc::clone(&result_nonce);
        let suffix_owned = suffix.to_string();

        let cuda = match gpu::CudaAshmaize::new_with_device(gpu_id) {
            Ok(c) => {
                eprintln!("  GPU {}: {}", gpu_id, c.get_device_info().unwrap());
                Arc::new(c)
            }
            Err(e) => {
                eprintln!("  GPU {}: Failed to initialize ({}), skipping", gpu_id, e);
                continue;
            }
        };

        let handle = thread::spawn(move || {
            while !found.load(Ordering::Relaxed) {
                let base_nonce = nonce_counter.fetch_add(gpu_batch_size as u64, Ordering::Relaxed);

                let salts: Vec<Vec<u8>> = (0..gpu_batch_size)
                    .map(|i| {
                        let nonce = base_nonce + i as u64;
                        let preimage = format!("{:016x}{}", nonce, suffix_owned);
                        preimage.into_bytes()
                    })
                    .collect();

                let salt_refs: Vec<&[u8]> = salts.iter().map(|s| s.as_slice()).collect();
                
                match cuda.hash_parallel(&salt_refs, &rom, 8, 256) {
                    Ok(hashes) => {
                        for (i, hash_result) in hashes.iter().enumerate() {
                            if hash_structure_good(hash_result, difficulty_mask) {
                                found.store(true, Ordering::Relaxed);
                                result_nonce.store(base_nonce + i as u64, Ordering::Relaxed);
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("GPU {} error: {:?}, batch skipped", gpu_id, e);
                    }
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().ok();
    }

    if found.load(Ordering::Relaxed) {
        println!("{:016x}", result_nonce.load(Ordering::Relaxed));
    }
}


    (0..num_workers).into_par_iter().for_each(|_| {
        let rom = Arc::clone(&rom);
        let cuda = Arc::clone(&cuda);
        let nonce_counter = Arc::clone(&nonce_counter);

        while !found.load(Ordering::Relaxed) {
            let base_nonce = {
                let mut counter = nonce_counter.lock().unwrap();
                let base = *counter;
                *counter += batch_size as u64;
                base
            };

            let mut salts = Vec::new();
            let mut nonces = Vec::new();

            for i in 0..batch_size {
                let nonce = base_nonce + i as u64;
                let preimage = format!("{:016x}{}", nonce, suffix);
                salts.push(preimage.into_bytes());
                nonces.push(nonce);
            }

            let salt_refs: Vec<&[u8]> = salts.iter().map(|v| v.as_slice()).collect();

            match cuda.hash_parallel(&salt_refs, &rom, 8, 256) {
                Ok(results) => {
                    for (i, hash_result) in results.iter().enumerate() {
                        if hash_structure_good(hash_result, difficulty_mask) {
                            found.store(true, Ordering::Relaxed);
                            result_nonce.store(nonces[i], Ordering::Relaxed);
                            break;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("GPU error: {}, batch skipped", e);
                }
            }
        }
    });

    if found.load(Ordering::Relaxed) {
        println!("{:016x}", result_nonce.load(Ordering::Relaxed));
    }
}

#[cfg(feature = "cuda")]
fn solve_multi_gpu(rom: &Rom, suffix: &str, difficulty_mask: u32, batch_size: usize) {
    use std::sync::mpsc;
    use std::thread;

    let _init = match gpu::CudaAshmaize::new() {
        Ok(init) => init,
        Err(e) => {
            eprintln!("Failed to initialize CUDA: {}, falling back to CPU", e);
            solve_cpu_only(rom, suffix, difficulty_mask);
            return;
        }
    };

    let gpu_count = match gpu::CudaAshmaize::get_device_count() {
        Ok(0) => {
            eprintln!("No GPUs detected, falling back to CPU");
            solve_cpu_only(rom, suffix, difficulty_mask);
            return;
        }
        Ok(count) => count,
        Err(e) => {
            eprintln!("Failed to detect GPUs: {}, falling back to CPU", e);
            solve_cpu_only(rom, suffix, difficulty_mask);
            return;
        }
    };

    eprintln!("Initializing {} GPU(s) for mining...", gpu_count);
    
    let rom = Arc::new(rom.clone());
    let found = Arc::new(AtomicBool::new(false));
    let result_nonce = Arc::new(AtomicU64::new(0));
    let nonce_counter = Arc::new(AtomicU64::new(0));

    let (result_tx, result_rx) = mpsc::channel::<(usize, u64, Result<Vec<[u8; 64]>, String>)>();

    let mut handles = Vec::new();
    for gpu_id in 0..gpu_count {
        let rom = Arc::clone(&rom);
        let found = Arc::clone(&found);
        let nonce_counter = Arc::clone(&nonce_counter);
        let result_tx = result_tx.clone();
        let suffix_owned = suffix.to_string();

        let cuda = match gpu::CudaAshmaize::new_with_device(gpu_id) {
            Ok(c) => {
                eprintln!("  GPU {}: {}", gpu_id, c.get_device_info().unwrap());
                Arc::new(c)
            }
            Err(e) => {
                eprintln!("  GPU {}: Failed to initialize ({}), skipping", gpu_id, e);
                continue;
            }
        };

        let handle = thread::spawn(move || {
            while !found.load(Ordering::Relaxed) {
                let base_nonce = nonce_counter.fetch_add(batch_size as u64, Ordering::Relaxed);

                let salts: Vec<Vec<u8>> = (0..batch_size)
                    .map(|i| {
                        let nonce = base_nonce + i as u64;
                        let preimage = format!("{:016x}{}", nonce, suffix_owned);
                        preimage.into_bytes()
                    })
                    .collect();

                let salt_refs: Vec<&[u8]> = salts.iter().map(|s| s.as_slice()).collect();
                
                let result = cuda.hash_parallel(&salt_refs, &rom, 8, 256)
                    .map_err(|e| format!("{:?}", e));
                
                result_tx.send((gpu_id, base_nonce, result)).ok();
            }
        });
        handles.push(handle);
    }
    drop(result_tx);

    let found_clone = Arc::clone(&found);
    let result_nonce_clone = Arc::clone(&result_nonce);
    let collector = thread::spawn(move || {
        while let Ok((gpu_id, base_nonce, result)) = result_rx.recv() {
            match result {
                Ok(hashes) => {
                    for (i, hash_result) in hashes.iter().enumerate() {
                        if hash_structure_good(hash_result, difficulty_mask) {
                            found_clone.store(true, Ordering::Relaxed);
                            result_nonce_clone.store(base_nonce + i as u64, Ordering::Relaxed);
                            return;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("GPU {} error: {}, batch skipped", gpu_id, e);
                }
            }
        }
    });

    for handle in handles {
        handle.join().ok();
    }
    collector.join().ok();

    if found.load(Ordering::Relaxed) {
        println!("{:016x}", result_nonce.load(Ordering::Relaxed));
    }
}

#[cfg(feature = "cuda")]
fn solve_mixed(rom: &Rom, suffix: &str, difficulty_mask: u32, gpu_batch_size: usize) {
    use std::thread;

    let _init = match gpu::CudaAshmaize::new() {
        Ok(init) => init,
        Err(e) => {
            eprintln!("Failed to initialize CUDA: {}, using CPU-only", e);
            solve_cpu_only(rom, suffix, difficulty_mask);
            return;
        }
    };

    let gpu_count = gpu::CudaAshmaize::get_device_count().unwrap_or(0);
    
    if gpu_count == 0 {
        eprintln!("No GPUs detected for mixed mode, using CPU-only");
        solve_cpu_only(rom, suffix, difficulty_mask);
        return;
    }

    eprintln!("Mixed mode: {} CPU thread(s) + {} GPU(s)", NUM_THREADS, gpu_count);
    
    let rom = Arc::new(rom.clone());
    let found = Arc::new(AtomicBool::new(false));
    let result_nonce = Arc::new(AtomicU64::new(0));
    let nonce_counter = Arc::new(AtomicU64::new(0));

    let mut handles = Vec::new();

    for _thread_id in 0..NUM_THREADS {
        let rom = Arc::clone(&rom);
        let found = Arc::clone(&found);
        let result_nonce = Arc::clone(&result_nonce);
        let nonce_counter = Arc::clone(&nonce_counter);
        let suffix_owned = suffix.to_string();

        let handle = thread::spawn(move || {
            let mut preimage = String::with_capacity(16 + suffix_owned.len());
            while !found.load(Ordering::Relaxed) {
                let nonce = nonce_counter.fetch_add(1, Ordering::Relaxed);
                
                preimage.clear();
                use std::fmt::Write;
                write!(&mut preimage, "{:016x}{}", nonce, suffix_owned).unwrap();

                let hash_result = hash(preimage.as_bytes(), &rom, 8, 256);
                
                if hash_structure_good(&hash_result, difficulty_mask) {
                    found.store(true, Ordering::Relaxed);
                    result_nonce.store(nonce, Ordering::Relaxed);
                    return;
                }
            }
        });
        handles.push(handle);
    }

    for gpu_id in 0..gpu_count {
        let rom = Arc::clone(&rom);
        let found = Arc::clone(&found);
        let nonce_counter = Arc::clone(&nonce_counter);
        let result_nonce = Arc::clone(&result_nonce);
        let suffix_owned = suffix.to_string();

        let cuda = match gpu::CudaAshmaize::new_with_device(gpu_id) {
            Ok(c) => {
                eprintln!("  GPU {}: {}", gpu_id, c.get_device_info().unwrap());
                Arc::new(c)
            }
            Err(e) => {
                eprintln!("  GPU {}: Failed to initialize ({}), skipping", gpu_id, e);
                continue;
            }
        };

        let handle = thread::spawn(move || {
            while !found.load(Ordering::Relaxed) {
                let base_nonce = nonce_counter.fetch_add(gpu_batch_size as u64, Ordering::Relaxed);

                let salts: Vec<Vec<u8>> = (0..gpu_batch_size)
                    .map(|i| {
                        let nonce = base_nonce + i as u64;
                        let preimage = format!("{:016x}{}", nonce, suffix_owned);
                        preimage.into_bytes()
                    })
                    .collect();

                let salt_refs: Vec<&[u8]> = salts.iter().map(|s| s.as_slice()).collect();
                
                match cuda.hash_parallel(&salt_refs, &rom, 8, 256) {
                    Ok(hashes) => {
                        for (i, hash_result) in hashes.iter().enumerate() {
                            if hash_structure_good(hash_result, difficulty_mask) {
                                found.store(true, Ordering::Relaxed);
                                result_nonce.store(base_nonce + i as u64, Ordering::Relaxed);
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("GPU {} error: {:?}, batch skipped", gpu_id, e);
                    }
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().ok();
    }

    if found.load(Ordering::Relaxed) {
        println!("{:016x}", result_nonce.load(Ordering::Relaxed));
    }
}


    (0..num_workers).into_par_iter().for_each(|_| {
        let rom = Arc::clone(&rom);
        let cuda = Arc::clone(&cuda);
        let nonce_counter = Arc::clone(&nonce_counter);

        while !found.load(Ordering::Relaxed) {
            let base_nonce = {
                let mut counter = nonce_counter.lock().unwrap();
                let base = *counter;
                *counter += batch_size as u64;
                base
            };

            let mut salts = Vec::new();
            let mut nonces = Vec::new();

            for i in 0..batch_size {
                let nonce = base_nonce + i as u64;
                let preimage = format!("{:016x}{}", nonce, suffix);
                salts.push(preimage.into_bytes());
                nonces.push(nonce);
            }

            let salt_refs: Vec<&[u8]> = salts.iter().map(|v| v.as_slice()).collect();

            match cuda.hash_parallel(&salt_refs, &rom, 8, 256) {
                Ok(results) => {
                    for (i, hash_result) in results.iter().enumerate() {
                        if hash_structure_good(hash_result, difficulty_mask) {
                            found.store(true, Ordering::Relaxed);
                            result_nonce.store(nonces[i], Ordering::Relaxed);
                            break;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("GPU error: {}, batch skipped", e);
                }
            }
        }
    });

    if found.load(Ordering::Relaxed) {
        println!("{:016x}", result_nonce.load(Ordering::Relaxed));
    }
}

#[cfg(feature = "cuda")]
fn solve_multi_gpu(rom: &Rom, suffix: &str, difficulty_mask: u32, batch_size: usize) {
    use std::sync::mpsc;
    use std::thread;

    let _init = match gpu::CudaAshmaize::new() {
        Ok(init) => init,
        Err(e) => {
            eprintln!("Failed to initialize CUDA: {}, falling back to CPU", e);
            solve_cpu_only(rom, suffix, difficulty_mask);
            return;
        }
    };

    let gpu_count = match gpu::CudaAshmaize::get_device_count() {
        Ok(0) => {
            eprintln!("No GPUs detected, falling back to CPU");
            solve_cpu_only(rom, suffix, difficulty_mask);
            return;
        }
        Ok(count) => count,
        Err(e) => {
            eprintln!("Failed to detect GPUs: {}, falling back to CPU", e);
            solve_cpu_only(rom, suffix, difficulty_mask);
            return;
        }
    };

    eprintln!("Initializing {} GPU(s) for mining...", gpu_count);
    
    let rom = Arc::new(rom.clone());
    let found = Arc::new(AtomicBool::new(false));
    let result_nonce = Arc::new(AtomicU64::new(0));
    let nonce_counter = Arc::new(AtomicU64::new(0));

    let (result_tx, result_rx) = mpsc::channel::<(usize, u64, Result<Vec<[u8; 64]>, String>)>();

    let mut handles = Vec::new();
    for gpu_id in 0..gpu_count {
        let rom = Arc::clone(&rom);
        let found = Arc::clone(&found);
        let nonce_counter = Arc::clone(&nonce_counter);
        let result_tx = result_tx.clone();
        let suffix_owned = suffix.to_string();

        let cuda = match gpu::CudaAshmaize::new_with_device(gpu_id) {
            Ok(c) => {
                eprintln!("  GPU {}: {}", gpu_id, c.get_device_info().unwrap());
                Arc::new(c)
            }
            Err(e) => {
                eprintln!("  GPU {}: Failed to initialize ({}), skipping", gpu_id, e);
                continue;
            }
        };

        let handle = thread::spawn(move || {
            while !found.load(Ordering::Relaxed) {
                let base_nonce = nonce_counter.fetch_add(batch_size as u64, Ordering::Relaxed);

                let salts: Vec<Vec<u8>> = (0..batch_size)
                    .map(|i| {
                        let nonce = base_nonce + i as u64;
                        let preimage = format!("{:016x}{}", nonce, suffix_owned);
                        preimage.into_bytes()
                    })
                    .collect();

                let salt_refs: Vec<&[u8]> = salts.iter().map(|s| s.as_slice()).collect();
                
                let result = cuda.hash_parallel(&salt_refs, &rom, 8, 256)
                    .map_err(|e| format!("{:?}", e));
                
                result_tx.send((gpu_id, base_nonce, result)).ok();
            }
        });
        handles.push(handle);
    }
    drop(result_tx);

    let found_clone = Arc::clone(&found);
    let result_nonce_clone = Arc::clone(&result_nonce);
    let collector = thread::spawn(move || {
        while let Ok((gpu_id, base_nonce, result)) = result_rx.recv() {
            match result {
                Ok(hashes) => {
                    for (i, hash_result) in hashes.iter().enumerate() {
                        if hash_structure_good(hash_result, difficulty_mask) {
                            found_clone.store(true, Ordering::Relaxed);
                            result_nonce_clone.store(base_nonce + i as u64, Ordering::Relaxed);
                            return;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("GPU {} error: {}, batch skipped", gpu_id, e);
                }
            }
        }
    });

    for handle in handles {
        handle.join().ok();
    }
    collector.join().ok();

    if found.load(Ordering::Relaxed) {
        println!("{:016x}", result_nonce.load(Ordering::Relaxed));
    }
}

#[cfg(feature = "cuda")]
fn solve_mixed(rom: &Rom, suffix: &str, difficulty_mask: u32, gpu_batch_size: usize) {
    use std::thread;

    let _init = match gpu::CudaAshmaize::new() {
        Ok(init) => init,
        Err(e) => {
            eprintln!("Failed to initialize CUDA: {}, using CPU-only", e);
            solve_cpu_only(rom, suffix, difficulty_mask);
            return;
        }
    };

    let gpu_count = gpu::CudaAshmaize::get_device_count().unwrap_or(0);
    
    if gpu_count == 0 {
        eprintln!("No GPUs detected for mixed mode, using CPU-only");
        solve_cpu_only(rom, suffix, difficulty_mask);
        return;
    }

    eprintln!("Mixed mode: {} CPU thread(s) + {} GPU(s)", NUM_THREADS, gpu_count);
    
    let rom = Arc::new(rom.clone());
    let found = Arc::new(AtomicBool::new(false));
    let result_nonce = Arc::new(AtomicU64::new(0));
    let nonce_counter = Arc::new(AtomicU64::new(0));

    let mut handles = Vec::new();

    for _thread_id in 0..NUM_THREADS {
        let rom = Arc::clone(&rom);
        let found = Arc::clone(&found);
        let result_nonce = Arc::clone(&result_nonce);
        let nonce_counter = Arc::clone(&nonce_counter);
        let suffix_owned = suffix.to_string();

        let handle = thread::spawn(move || {
            let mut preimage = String::with_capacity(16 + suffix_owned.len());
            while !found.load(Ordering::Relaxed) {
                let nonce = nonce_counter.fetch_add(1, Ordering::Relaxed);
                
                preimage.clear();
                use std::fmt::Write;
                write!(&mut preimage, "{:016x}{}", nonce, suffix_owned).unwrap();

                let hash_result = hash(preimage.as_bytes(), &rom, 8, 256);
                
                if hash_structure_good(&hash_result, difficulty_mask) {
                    found.store(true, Ordering::Relaxed);
                    result_nonce.store(nonce, Ordering::Relaxed);
                    return;
                }
            }
        });
        handles.push(handle);
    }

    for gpu_id in 0..gpu_count {
        let rom = Arc::clone(&rom);
        let found = Arc::clone(&found);
        let nonce_counter = Arc::clone(&nonce_counter);
        let result_nonce = Arc::clone(&result_nonce);
        let suffix_owned = suffix.to_string();

        let cuda = match gpu::CudaAshmaize::new_with_device(gpu_id) {
            Ok(c) => {
                eprintln!("  GPU {}: {}", gpu_id, c.get_device_info().unwrap());
                Arc::new(c)
            }
            Err(e) => {
                eprintln!("  GPU {}: Failed to initialize ({}), skipping", gpu_id, e);
                continue;
            }
        };

        let handle = thread::spawn(move || {
            while !found.load(Ordering::Relaxed) {
                let base_nonce = nonce_counter.fetch_add(gpu_batch_size as u64, Ordering::Relaxed);

                let salts: Vec<Vec<u8>> = (0..gpu_batch_size)
                    .map(|i| {
                        let nonce = base_nonce + i as u64;
                        let preimage = format!("{:016x}{}", nonce, suffix_owned);
                        preimage.into_bytes()
                    })
                    .collect();

                let salt_refs: Vec<&[u8]> = salts.iter().map(|s| s.as_slice()).collect();
                
                match cuda.hash_parallel(&salt_refs, &rom, 8, 256) {
                    Ok(hashes) => {
                        for (i, hash_result) in hashes.iter().enumerate() {
                            if hash_structure_good(hash_result, difficulty_mask) {
                                found.store(true, Ordering::Relaxed);
                                result_nonce.store(base_nonce + i as u64, Ordering::Relaxed);
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("GPU {} error: {:?}, batch skipped", gpu_id, e);
                    }
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().ok();
    }

    if found.load(Ordering::Relaxed) {
        println!("{:016x}", result_nonce.load(Ordering::Relaxed));
    }
}
