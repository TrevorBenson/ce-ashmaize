// Test a single loop iteration CPU vs GPU

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
        println!("=== Single Loop Iteration Test ===\n");

        let rom = Rom::new(
            b"0",
            RomGenerationType::TwoStep {
                pre_size: 1 * MB,
                mixing_numbers: 1,
            },
            1 * MB,
        );

        let salt = b"test";
        let nb_instrs = 256u32;

        println!("Salt: {}", String::from_utf8_lossy(salt));
        println!("nb_instrs: {}", nb_instrs);
        println!("ROM size: {} bytes\n", rom.data.len());

        // CPU: Execute one loop
        println!("CPU:");
        let mut cpu_vm = VM::new(&rom.digest, nb_instrs, salt);
        println!("  Before loop:");
        println!("    regs[0]: 0x{:016x}", cpu_vm.regs[0]);
        println!("    regs[1]: 0x{:016x}", cpu_vm.regs[1]);
        println!("    memory_counter: {}", cpu_vm.memory_counter);
        
        cpu_vm.execute(&rom, nb_instrs);
        
        println!("  After 1 loop:");
        println!("    regs[0]: 0x{:016x}", cpu_vm.regs[0]);
        println!("    regs[1]: 0x{:016x}", cpu_vm.regs[1]);
        println!("    memory_counter: {}", cpu_vm.memory_counter);
        println!("    prog_seed[0..8]: {}", hex::encode(&cpu_vm.prog_seed[..8]));

        // GPU: Execute one loop
        println!("\nGPU:");
        let device = CudaDevice::new(0).expect("Failed to create device");
        
        let ptx = compile_single_loop_test();
        device.load_ptx(ptx, "test_single_loop", &["test_single_loop_kernel"]).expect("Load PTX failed");
        let kernel = device.get_func("test_single_loop", "test_single_loop_kernel").expect("Get func failed");
        
        let rom_buffer = device.htod_sync_copy(&rom.data).expect("ROM copy failed");
        let rom_digest_buffer = device.htod_sync_copy(rom.digest.as_bytes()).expect("ROM digest copy failed");
        let salt_buffer = device.htod_sync_copy(salt).expect("Salt copy failed");
        let mut regs_buffer = device.alloc_zeros::<u64>(32).expect("Regs alloc failed");
        let mut prog_seed_buffer = device.alloc_zeros::<u8>(64).expect("Prog seed alloc failed");
        let mut mem_counter_buffer = device.alloc_zeros::<u32>(1).expect("Mem counter alloc failed");
        
        let program_size = (nb_instrs * 20) as u32;
        
        let cfg = LaunchConfig {
            grid_dim: (1, 1, 1),
            block_dim: (1, 1, 1),
            shared_mem_bytes: 0,
        };
        
        unsafe {
            kernel.launch(cfg, (
                &rom_buffer,
                rom.data.len() as u32,
                &rom_digest_buffer,
                &salt_buffer,
                salt.len() as u32,
                nb_instrs,
                program_size,
                &mut regs_buffer,
                &mut prog_seed_buffer,
                &mut mem_counter_buffer,
            )).expect("Launch failed");
        }
        
        device.synchronize().expect("Sync failed");
        
        let gpu_regs = device.dtoh_sync_copy(&regs_buffer).expect("Regs copy failed");
        let gpu_prog_seed = device.dtoh_sync_copy(&prog_seed_buffer).expect("Prog seed copy failed");
        let gpu_mem_counter = device.dtoh_sync_copy(&mem_counter_buffer).expect("Mem counter copy failed");
        
        println!("  After 1 loop:");
        println!("    regs[0]: 0x{:016x}", gpu_regs[0]);
        println!("    regs[1]: 0x{:016x}", gpu_regs[1]);
        println!("    memory_counter: {}", gpu_mem_counter[0]);
        println!("    prog_seed[0..8]: {}", hex::encode(&gpu_prog_seed[..8]));

        // Compare
        println!("\nComparison:");
        let regs_match = cpu_vm.regs.iter().zip(gpu_regs.iter()).all(|(a, b)| a == b);
        let prog_seed_match = cpu_vm.prog_seed == &gpu_prog_seed[..];
        let mem_counter_match = cpu_vm.memory_counter == gpu_mem_counter[0];
        
        if regs_match && prog_seed_match && mem_counter_match {
            println!("  ✓ ALL MATCH!");
        } else {
            println!("  ✗ MISMATCH:");
            if !regs_match {
                println!("    Registers differ:");
                for i in 0..32 {
                    if cpu_vm.regs[i] != gpu_regs[i] {
                        println!("      reg[{}]: CPU=0x{:016x}, GPU=0x{:016x}",
                            i, cpu_vm.regs[i], gpu_regs[i]);
                    }
                }
            }
            if !prog_seed_match {
                println!("    prog_seed differs");
                println!("      CPU: {}", hex::encode(&cpu_vm.prog_seed[..16]));
                println!("      GPU: {}", hex::encode(&gpu_prog_seed[..16]));
            }
            if !mem_counter_match {
                println!("    memory_counter: CPU={}, GPU={}", 
                    cpu_vm.memory_counter, gpu_mem_counter[0]);
            }
        }
    }
}

#[cfg(feature = "cuda")]
fn compile_single_loop_test() -> cudarc::nvrtc::Ptx {
    let test_cu = include_str!("../cuda/test_single_loop.cu");
    let blake2b_cuh = include_str!("../cuda/blake2b.cuh");
    let argon2_cuh = include_str!("../cuda/argon2.cuh");
    let ashmaize_vm_cuh = include_str!("../cuda/ashmaize_vm.cuh");
    let ashmaize_cu = include_str!("../cuda/ashmaize.cu");
    
    // Remove includes to avoid duplication
    let argon2_cleaned = argon2_cuh.replace("#include \"blake2b.cuh\"", "");
    let vm_cleaned = ashmaize_vm_cuh
        .replace("#include \"blake2b.cuh\"", "")
        .replace("#include \"argon2.cuh\"", "");
    
    // Extract just execute_one_instruction and post_instructions from ashmaize.cu
    // Skip the main kernel
    let mut in_kernel = false;
    let mut brace_depth = 0;
    let ashmaize_cleaned = ashmaize_cu
        .replace("#include \"ashmaize_vm.cuh\"", "")
        .lines()
        .filter_map(|line| {
            // Skip extern "C" __global__ kernel definition
            if line.contains("extern \"C\" __global__") {
                in_kernel = true;
                brace_depth = 0;
                return None;
            }
            
            if in_kernel {
                // Track braces to know when kernel ends
                brace_depth += line.matches('{').count() as i32;
                brace_depth -= line.matches('}').count() as i32;
                
                if brace_depth <= 0 {
                    in_kernel = false;
                }
                return None;
            }
            
            Some(line)
        })
        .collect::<Vec<_>>()
        .join("\n");
    
    let full_source = format!("{}\n{}\n{}\n{}\n{}", 
        blake2b_cuh, argon2_cleaned, vm_cleaned, ashmaize_cleaned, test_cu);
    
    let out_dir = std::env::var("OUT_DIR").unwrap_or_else(|_| "/tmp".to_string());
    let source_path = format!("{}/test_single_loop_combined.cu", out_dir);
    let mut file = std::fs::File::create(&source_path).expect("Failed to create source");
    file.write_all(full_source.as_bytes()).expect("Failed to write source");
    
    let ptx_path = format!("{}/test_single_loop.ptx", out_dir);
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

