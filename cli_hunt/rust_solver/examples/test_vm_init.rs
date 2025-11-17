// Test VM initialization CPU vs GPU

use ashmaize::{Rom, RomGenerationType, b2::VM};

#[cfg(feature = "cuda")]
use cudarc::driver::*;
#[cfg(feature = "cuda")]
use std::io::Write;

const MB: usize = 1_024 * 1_024;

fn main() {
    #[cfg(not(feature = "cuda"))]
    {
        eprintln!("This example requires the 'cuda' feature");
        std::process::exit(1);
    }

    #[cfg(feature = "cuda")]
    {
        println!("=== VM Initialization Test ===\n");

        let rom = Rom::new(
            b"0",
            RomGenerationType::TwoStep {
                pre_size: 1 * MB,
                mixing_numbers: 1,
            },
            1 * MB,
        );

        let test_salts = vec![
            ("empty", b"" as &[u8]),
            ("a", b"a"),
            ("test", b"test"),
        ];

        println!("ROM digest: {}\n", hex::encode(&rom.digest.as_bytes()[..16]));

        let device = CudaDevice::new(0).expect("Failed to create device");

        for (name, salt) in &test_salts {
            println!("Test: '{}'", name);
            println!("  Salt: {} (len={})", hex::encode(salt), salt.len());
            
            // CPU
            let cpu_vm = VM::new(&rom.digest, 256, salt);
            println!("  CPU:");
            println!("    prog_seed[0..16]: {}", hex::encode(&cpu_vm.prog_seed[..16]));
            println!("    regs[0]: 0x{:016x}", cpu_vm.regs[0]);
            println!("    regs[1]: 0x{:016x}", cpu_vm.regs[1]);
            
            // GPU
            let ptx = compile_vm_init_test();
            device.load_ptx(ptx, "test_vm_init", &["test_vm_init_kernel"]).expect("Load PTX failed");
            let kernel = device.get_func("test_vm_init", "test_vm_init_kernel").expect("Get func failed");
            
            let rom_digest_buffer = device.htod_sync_copy(rom.digest.as_bytes()).expect("ROM digest copy failed");
            let salt_buffer = device.htod_sync_copy(salt).expect("Salt copy failed");
            let mut regs_buffer = device.alloc_zeros::<u64>(32).expect("Regs alloc failed");
            let mut prog_seed_buffer = device.alloc_zeros::<u8>(64).expect("Prog seed alloc failed");
            
            let cfg = LaunchConfig {
                grid_dim: (1, 1, 1),
                block_dim: (1, 1, 1),
                shared_mem_bytes: 0,
            };
            
            unsafe {
                kernel.launch(cfg, (
                    &rom_digest_buffer,
                    &salt_buffer,
                    salt.len() as u32,
                    &mut regs_buffer,
                    &mut prog_seed_buffer,
                )).expect("Launch failed");
            }
            
            device.synchronize().expect("Sync failed");
            
            let gpu_regs = device.dtoh_sync_copy(&regs_buffer).expect("Regs copy failed");
            let gpu_prog_seed = device.dtoh_sync_copy(&prog_seed_buffer).expect("Prog seed copy failed");
            
            println!("  GPU:");
            println!("    prog_seed[0..16]: {}", hex::encode(&gpu_prog_seed[..16]));
            println!("    regs[0]: 0x{:016x}", gpu_regs[0]);
            println!("    regs[1]: 0x{:016x}", gpu_regs[1]);
            
            // Compare
            let prog_seed_match = cpu_vm.prog_seed == &gpu_prog_seed[..];
            let regs_match = cpu_vm.regs.iter().zip(gpu_regs.iter()).all(|(a, b)| a == b);
            
            if prog_seed_match && regs_match {
                println!("  ✓ MATCH\n");
            } else {
                println!("  ✗ MISMATCH");
                if !prog_seed_match {
                    println!("    prog_seed differs!");
                }
                if !regs_match {
                    println!("    registers differ!");
                    for i in 0..32 {
                        if cpu_vm.regs[i] != gpu_regs[i] {
                            println!("      reg[{}]: CPU=0x{:016x}, GPU=0x{:016x}",
                                i, cpu_vm.regs[i], gpu_regs[i]);
                        }
                    }
                }
                println!();
            }
        }
    }
}

#[cfg(feature = "cuda")]
fn compile_vm_init_test() -> cudarc::nvrtc::Ptx {
    let test_cu = include_str!("../cuda/test_vm_init.cu");
    let blake2b_cuh = include_str!("../cuda/blake2b.cuh");
    let argon2_cuh = include_str!("../cuda/argon2.cuh");
    let ashmaize_vm_cuh = include_str!("../cuda/ashmaize_vm.cuh");
    
    let argon2_cuh_cleaned = argon2_cuh.replace("#include \"blake2b.cuh\"", "");
    let vm_cuh_cleaned = ashmaize_vm_cuh
        .replace("#include \"blake2b.cuh\"", "")
        .replace("#include \"argon2.cuh\"", "");
    
    let full_source = format!("{}\n{}\n{}\n{}", 
        blake2b_cuh, argon2_cuh_cleaned, vm_cuh_cleaned, test_cu);
    
    let out_dir = std::env::var("OUT_DIR").unwrap_or_else(|_| "/tmp".to_string());
    let source_path = format!("{}/test_vm_init_combined.cu", out_dir);
    let mut file = std::fs::File::create(&source_path).expect("Failed to create source");
    file.write_all(full_source.as_bytes()).expect("Failed to write source");
    
    let ptx_path = format!("{}/test_vm_init.ptx", out_dir);
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

