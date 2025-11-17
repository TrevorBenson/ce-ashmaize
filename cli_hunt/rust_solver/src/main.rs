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
    #[arg(long, default_value = "1024")]
    gpu_batch_size: usize,
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
) {
    let rom = init_rom(no_pre_mine);
    let difficulty_mask = u32::from_str_radix(difficulty, 16).unwrap();

    let suffix = format!(
        "{}{}{}{}{}{}",
        address, challenge_id, difficulty, no_pre_mine, latest_submission, no_pre_mine_hour
    );

    #[cfg(feature = "cuda")]
    if use_gpu {
        solve_with_gpu(&rom, &suffix, difficulty_mask, gpu_batch_size, gpu_id);
        return;
    }

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
