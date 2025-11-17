use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let cuda_feature = env::var("CARGO_FEATURE_CUDA").is_ok();
    
    if !cuda_feature {
        return;
    }

    println!("cargo:rerun-if-changed=cuda/");
    println!("cargo:rerun-if-changed=cuda/ashmaize.cu");
    println!("cargo:rerun-if-changed=cuda/ashmaize_shared_mem.cu");
    println!("cargo:rerun-if-changed=cuda/blake2b.cuh");
    println!("cargo:rerun-if-changed=cuda/argon2.cuh");
    println!("cargo:rerun-if-changed=cuda/ashmaize_vm.cuh");

    let cuda_path = find_cuda_path();
    
    if let Some(cuda_root) = cuda_path {
        let out_dir = env::var("OUT_DIR").unwrap();
        let cuda_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("cuda");
        let nvcc = format!("{}/bin/nvcc", cuda_root);
        
        // Compile main kernel
        let ptx_path = PathBuf::from(&out_dir).join("ashmaize.ptx");
        compile_cuda_kernel(&nvcc, &cuda_dir, &ptx_path, "ashmaize.cu");
        
        // TODO: Compile shared memory kernel when fixed
        // let ptx_shared_path = PathBuf::from(&out_dir).join("ashmaize_shared_mem.ptx");
        // compile_cuda_kernel(&nvcc, &cuda_dir, &ptx_shared_path, "ashmaize_shared_mem.cu");
    } else {
        println!("cargo:warning=CUDA not found, GPU support will be disabled");
    }
}

fn compile_cuda_kernel(nvcc: &str, cuda_dir: &PathBuf, ptx_path: &PathBuf, cu_file: &str) {
    let output = Command::new(nvcc)
        .args(&[
            "-ptx",
            "-arch=sm_89",
            "--ptxas-options=-v",
            "-I", cuda_dir.to_str().unwrap(),
            "-o", ptx_path.to_str().unwrap(),
            cuda_dir.join(cu_file).to_str().unwrap(),
        ])
        .output();

    match output {
        Ok(output) => {
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            if !stderr_str.is_empty() {
                for line in stderr_str.lines() {
                    println!("cargo:warning={}:{}", cu_file, line);
                }
            }
            
            if !output.status.success() {
                eprintln!("nvcc stdout: {}", String::from_utf8_lossy(&output.stdout));
                panic!("CUDA compilation failed for {}", cu_file);
            }
            println!("cargo:warning=CUDA kernel {} compiled to {}", cu_file, ptx_path.display());
        }
        Err(e) => {
            eprintln!("Failed to run nvcc: {}", e);
            panic!("Could not execute nvcc");
        }
    }
}

fn find_cuda_path() -> Option<String> {
    if let Ok(cuda_path) = env::var("CUDA_PATH") {
        return Some(cuda_path);
    }
    
    if let Ok(cuda_home) = env::var("CUDA_HOME") {
        return Some(cuda_home);
    }

    let common_paths = [
        "/usr/local/cuda",
        "/opt/cuda",
        "/usr/lib/cuda",
    ];

    for path in &common_paths {
        let nvcc_path = format!("{}/bin/nvcc", path);
        if std::path::Path::new(&nvcc_path).exists() {
            return Some(path.to_string());
        }
    }

    let output = Command::new("which").arg("nvcc").output();
    if let Ok(output) = output {
        if output.status.success() {
            let nvcc_path = String::from_utf8_lossy(&output.stdout);
            let nvcc_path = nvcc_path.trim();
            if let Some(cuda_bin) = nvcc_path.strip_suffix("/bin/nvcc") {
                return Some(cuda_bin.to_string());
            }
        }
    }

    None
}



    let output = Command::new("which").arg("nvcc").output();
    if let Ok(output) = output {
        if output.status.success() {
            let nvcc_path = String::from_utf8_lossy(&output.stdout);
            let nvcc_path = nvcc_path.trim();
            if let Some(cuda_bin) = nvcc_path.strip_suffix("/bin/nvcc") {
                return Some(cuda_bin.to_string());
            }
        }
    }

    None
}



    let output = Command::new("which").arg("nvcc").output();
    if let Ok(output) = output {
        if output.status.success() {
            let nvcc_path = String::from_utf8_lossy(&output.stdout);
            let nvcc_path = nvcc_path.trim();
            if let Some(cuda_bin) = nvcc_path.strip_suffix("/bin/nvcc") {
                return Some(cuda_bin.to_string());
            }
        }
    }

    None
}

