// Test Argon2 H' implementation CPU vs GPU

use ashmaize::b2::argon2;

#[cfg(feature = "cuda")]
use cudarc::driver::*;
#[cfg(feature = "cuda")]
use std::io::Write;

fn main() {
    #[cfg(not(feature = "cuda"))]
    {
        eprintln!("This example requires the 'cuda' feature");
        std::process::exit(1);
    }

    #[cfg(feature = "cuda")]
    {
        println!("=== Argon2 H' (hprime) Test ===\n");

        // Test cases with varying input/output sizes
        let test_cases = vec![
            ("Empty input, 64 output", b"" as &[u8], 64),
            ("Short input, 64 output", b"test", 64),
            ("Long input, 64 output", b"this is a longer test input", 64),
            ("Short input, 448 output", b"test", 448), // 256*8 + 3*64 for VM init
        ];

        println!("CPU Results:");
        for (name, input, output_len) in &test_cases {
            let mut cpu_output = vec![0u8; *output_len];
            argon2::hprime(&mut cpu_output, input);
            println!("  {}", name);
            println!("    First 16 bytes: {}", hex::encode(&cpu_output[..16]));
        }

        println!("\nGPU Results:");
        let device = CudaDevice::new(0).expect("Failed to create device");
        
        for (name, input, output_len) in &test_cases {
            // Compile kernel
            let ptx = compile_hprime_test();
            device.load_ptx(ptx, "test_hprime", &["test_hprime_kernel"]).expect("Load PTX failed");
            let kernel = device.get_func("test_hprime", "test_hprime_kernel").expect("Get func failed");
            
            // Prepare buffers
            let input_buffer = device.htod_sync_copy(input).expect("Input copy failed");
            let mut output_buffer = device.alloc_zeros::<u8>(*output_len).expect("Alloc failed");
            
            let cfg = LaunchConfig {
                grid_dim: (1, 1, 1),
                block_dim: (1, 1, 1),
                shared_mem_bytes: 0,
            };
            
            unsafe {
                kernel.launch(cfg, (
                    &input_buffer,
                    input.len() as u32,
                    &mut output_buffer,
                    *output_len as u32,
                )).expect("Launch failed");
            }
            
            device.synchronize().expect("Sync failed");
            let gpu_output = device.dtoh_sync_copy(&output_buffer).expect("Copy back failed");
            
            println!("  {}", name);
            println!("    First 16 bytes: {}", hex::encode(&gpu_output[..16]));
        }

        println!("\nComparison:");
        for (name, input, output_len) in &test_cases {
            let mut cpu_output = vec![0u8; *output_len];
            argon2::hprime(&mut cpu_output, input);
            
            // Re-run GPU
            let ptx = compile_hprime_test();
            device.load_ptx(ptx, "test_hprime", &["test_hprime_kernel"]).expect("Load PTX failed");
            let kernel = device.get_func("test_hprime", "test_hprime_kernel").expect("Get func failed");
            
            let input_buffer = device.htod_sync_copy(input).expect("Input copy failed");
            let mut output_buffer = device.alloc_zeros::<u8>(*output_len).expect("Alloc failed");
            
            let cfg = LaunchConfig {
                grid_dim: (1, 1, 1),
                block_dim: (1, 1, 1),
                shared_mem_bytes: 0,
            };
            
            unsafe {
                kernel.launch(cfg, (
                    &input_buffer,
                    input.len() as u32,
                    &mut output_buffer,
                    *output_len as u32,
                )).expect("Launch failed");
            }
            
            device.synchronize().expect("Sync failed");
            let gpu_output = device.dtoh_sync_copy(&output_buffer).expect("Copy back failed");
            
            let matches = cpu_output == gpu_output;
            let marker = if matches { "✓" } else { "✗" };
            println!("  {} {}", marker, name);
            
            if !matches {
                // Find first difference
                for i in 0..cpu_output.len() {
                    if cpu_output[i] != gpu_output[i] {
                        println!("    First diff at byte {}: CPU={:02x}, GPU={:02x}",
                            i, cpu_output[i], gpu_output[i]);
                        break;
                    }
                }
            }
        }
    }
}

#[cfg(feature = "cuda")]
fn compile_hprime_test() -> cudarc::nvrtc::Ptx {
    let test_cu = include_str!("../cuda/test_hprime.cu");
    let blake2b_cuh = include_str!("../cuda/blake2b.cuh");
    let argon2_cuh = include_str!("../cuda/argon2.cuh");
    
    // Remove the #include from argon2.cuh since we're combining them
    let argon2_cuh_cleaned = argon2_cuh.replace("#include \"blake2b.cuh\"", "// blake2b.cuh already included");
    
    let full_source = format!("{}\n{}\n{}", blake2b_cuh, argon2_cuh_cleaned, test_cu);
    
    let out_dir = std::env::var("OUT_DIR").unwrap_or_else(|_| "/tmp".to_string());
    let source_path = format!("{}/test_hprime_combined.cu", out_dir);
    let mut file = std::fs::File::create(&source_path).expect("Failed to create source");
    file.write_all(full_source.as_bytes()).expect("Failed to write source");
    
    let ptx_path = format!("{}/test_hprime.ptx", out_dir);
    let status = std::process::Command::new("nvcc")
        .args(&[
            "--ptx",
            "-arch=sm_89",
            "-o", &ptx_path,
            &source_path,
        ])
        .status()
        .expect("Failed to run nvcc");
    
    if !status.success() {
        panic!("nvcc compilation failed");
    }
    
    let ptx_string = std::fs::read_to_string(&ptx_path).expect("Failed to read PTX");
    cudarc::nvrtc::Ptx::from_src(ptx_string)
}

