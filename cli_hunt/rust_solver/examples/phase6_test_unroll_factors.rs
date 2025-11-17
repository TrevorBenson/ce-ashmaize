// Phase 6: Test Different Unroll Factors
// Quickly test unroll 4, 8, 16, 32 to find optimal

use ashmaize::{Rom, RomGenerationType};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::sync::mpsc;
use std::thread;
use std::time::{Instant, Duration};

#[cfg(feature = "cuda")]
use ashmaize_solver::gpu::CudaAshmaize;

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * MB;
const TEST_DURATION: u64 = 20;

fn main() {
    println!("This test requires manually changing #pragma unroll in ashmaize.cu");
    println!("Test unroll factors: 4, 8, 16, 32");
    println!("Current: unroll 8 = 277.1k H/s");
    println!();
    println!("Run multi_gpu_final for each factor and record results.");
}

// Quickly test unroll 4, 8, 16, 32 to find optimal

use ashmaize::{Rom, RomGenerationType};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::sync::mpsc;
use std::thread;
use std::time::{Instant, Duration};

#[cfg(feature = "cuda")]
use ashmaize_solver::gpu::CudaAshmaize;

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * MB;
const TEST_DURATION: u64 = 20;

fn main() {
    println!("This test requires manually changing #pragma unroll in ashmaize.cu");
    println!("Test unroll factors: 4, 8, 16, 32");
    println!("Current: unroll 8 = 277.1k H/s");
    println!();
    println!("Run multi_gpu_final for each factor and record results.");
}

// Quickly test unroll 4, 8, 16, 32 to find optimal

use ashmaize::{Rom, RomGenerationType};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::sync::mpsc;
use std::thread;
use std::time::{Instant, Duration};

#[cfg(feature = "cuda")]
use ashmaize_solver::gpu::CudaAshmaize;

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * MB;
const TEST_DURATION: u64 = 20;

fn main() {
    println!("This test requires manually changing #pragma unroll in ashmaize.cu");
    println!("Test unroll factors: 4, 8, 16, 32");
    println!("Current: unroll 8 = 277.1k H/s");
    println!();
    println!("Run multi_gpu_final for each factor and record results.");
}




