// Test Blake2b GPU implementation against known values

#[cfg(feature = "cuda")]
use cudarc::driver::*;

fn main() {
    #[cfg(not(feature = "cuda"))]
    {
        eprintln!("This example requires the 'cuda' feature");
        std::process::exit(1);
    }

    #[cfg(feature = "cuda")]
    {
        println!("=== Blake2b GPU Test ===\n");

        // CPU reference results
        let test_cases = vec![
            ("Empty", b"" as &[u8]),
            ("abc", b"abc"),
            ("test", b"test"),
        ];

        println!("CPU Reference Results:");
        for (name, input) in &test_cases {
            // Use blake2b_simd which is available in ashmaize
            let hash = blake2b_simd::blake2b(input);
            println!("  {}: {}", name, hex::encode(&hash.as_bytes()[..16]));
        }

        // Test GPU
        println!("\nCompiling and testing GPU Blake2b...");
        
        let device = CudaDevice::new(0).expect("Failed to create CUDA device");
        
        // Compile test kernel
        let ptx = compile_test_kernel();
        device.load_ptx(ptx, "test_blake2b", &["test_blake2b_kernel"]).expect("Failed to load PTX");
        
        let kernel = device.get_func("test_blake2b", "test_blake2b_kernel").expect("Failed to get kernel");
        
        // Allocate results buffer (3 tests * 64 bytes each)
        let mut results_buffer = device.alloc_zeros::<u8>(192).expect("Failed to allocate");
        
        let cfg = LaunchConfig {
            grid_dim: (1, 1, 1),
            block_dim: (3, 1, 1),
            shared_mem_bytes: 0,
        };
        
        unsafe {
            kernel.launch(cfg, (&mut results_buffer,)).expect("Kernel launch failed");
        }
        
        device.synchronize().expect("Sync failed");
        
        let results_host = device.dtoh_sync_copy(&results_buffer).expect("Copy failed");
        
        println!("\nGPU Results:");
        let names = vec!["Empty", "abc", "test"];
        for (i, name) in names.iter().enumerate() {
            let start = i * 64;
            println!("  {}: {}", name, hex::encode(&results_host[start..start+16]));
        }
        
        println!("\nComparison:");
        for (i, (name, input)) in test_cases.iter().enumerate() {
            let cpu_hash = blake2b_simd::blake2b(input);
            let gpu_start = i * 64;
            let gpu_hash = &results_host[gpu_start..gpu_start+64];
            
            let matches = cpu_hash.as_bytes() == gpu_hash;
            let marker = if matches { "✓" } else { "✗" };
            println!("  {} {}", marker, name);
            
            if !matches {
                println!("    CPU: {}", hex::encode(cpu_hash.as_bytes()));
                println!("    GPU: {}", hex::encode(gpu_hash));
            }
        }
    }
}

#[cfg(feature = "cuda")]
fn compile_test_kernel() -> cudarc::nvrtc::Ptx {
    use std::io::Write;
    
    // Read the test kernel
    let test_cu = include_str!("../cuda/test_blake2b.cu");
    let blake2b_cuh = include_str!("../cuda/blake2b.cuh");
    
    // Combine includes
    let full_source = format!("{}\n{}", blake2b_cuh, test_cu);
    
    // Write to temp file for compilation
    let out_dir = std::env::var("OUT_DIR").unwrap_or_else(|_| "/tmp".to_string());
    let source_path = format!("{}/test_blake2b_combined.cu", out_dir);
    let mut file = std::fs::File::create(&source_path).expect("Failed to create source file");
    file.write_all(full_source.as_bytes()).expect("Failed to write source");
    
    // Compile with nvcc
    let ptx_path = format!("{}/test_blake2b.ptx", out_dir);
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

