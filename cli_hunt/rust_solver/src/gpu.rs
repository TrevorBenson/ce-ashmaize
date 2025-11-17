use ashmaize::b2::VM;
use ashmaize::{Rom, RomDigest};
use cudarc::driver::*;
use std::sync::Arc;

pub type GpuResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct CudaAshmaize {
    device: Arc<CudaDevice>,
    kernel_func: CudaFunction,
}

impl CudaAshmaize {
    pub fn new() -> GpuResult<Self> {
        let device = CudaDevice::new(0)?;

        let ptx = compile_kernel()?;
        device.load_ptx(ptx.clone(), "ashmaize", &["ashmaize_hash_kernel"])?;
        
        let kernel_func = device.get_func("ashmaize", "ashmaize_hash_kernel")
            .ok_or("Failed to get kernel function")?;

        Ok(Self {
            device,
            kernel_func,
        })
    }

    pub fn hash_parallel(
        &self,
        salts: &[&[u8]],
        rom: &Rom,
        nb_loops: u32,
        nb_instrs: u32,
    ) -> GpuResult<Vec<[u8; 64]>> {
        let num_hashes = salts.len();
        let program_size = nb_instrs as usize * 20;

        let mut all_salts = Vec::new();
        for salt in salts {
            let mut padded_salt = vec![0u8; 32];
            let len = std::cmp::min(salt.len(), 32);
            padded_salt[..len].copy_from_slice(&salt[..len]);
            all_salts.extend_from_slice(&padded_salt);
        }

        let mut initial_prog_seeds = Vec::new();
        for salt in salts {
            let vm = VM::new(&rom.digest, nb_instrs, salt);
            initial_prog_seeds.extend_from_slice(&vm.prog_seed);
        }

        let template_program = vec![0u8; program_size];
        let mut all_programs = Vec::new();
        for _ in 0..num_hashes {
            all_programs.extend_from_slice(&template_program);
        }

        let rom_buffer = self.device.htod_sync_copy(&rom.data)?;
        let rom_digest_buffer = self.device.htod_sync_copy(&rom.digest.0)?;
        let salts_buffer = self.device.htod_sync_copy(&all_salts)?;
        let initial_prog_seeds_buffer = self.device.htod_sync_copy(&initial_prog_seeds)?;
        let mut programs_buffer = self.device.htod_sync_copy(&all_programs)?;
        let mut results_buffer = self.device.alloc_zeros::<u8>(num_hashes * 64)?;

        let salt_len = salts[0].len() as u32;

        let threads_per_block = 256;
        let num_blocks = (num_hashes as u32 + threads_per_block - 1) / threads_per_block;

        let cfg = LaunchConfig {
            grid_dim: (num_blocks, 1, 1),
            block_dim: (threads_per_block, 1, 1),
            shared_mem_bytes: 0,
        };

        unsafe {
            self.kernel_func.launch(
                cfg,
                (
                    &rom_buffer,
                    rom.data.len() as u32,
                    &rom_digest_buffer,
                    &salts_buffer,
                    salt_len,
                    &initial_prog_seeds_buffer,
                    &mut programs_buffer,
                    nb_loops,
                    nb_instrs,
                    program_size as u32,
                    &mut results_buffer,
                    num_hashes as u32,
                ),
            )?;
        }

        let results_host = self.device.dtoh_sync_copy(&results_buffer)?;
        
        let mut results = Vec::new();
        for i in 0..num_hashes {
            let mut result = [0u8; 64];
            result.copy_from_slice(&results_host[i * 64..(i + 1) * 64]);
            results.push(result);
        }

        Ok(results)
    }

    pub fn get_device_info(&self) -> GpuResult<String> {
        Ok(format!(
            "CUDA Device: {}\nCompute Capability: {}.{}",
            self.device.name()?,
            self.device.attribute(CudaDeviceAttribute::ComputeCapabilityMajor)?,
            self.device.attribute(CudaDeviceAttribute::ComputeCapabilityMinor)?,
        ))
    }
}

fn compile_kernel() -> GpuResult<CudaPtx> {
    let cuda_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("cuda");
    
    let output = std::process::Command::new("nvcc")
        .args(&[
            "-ptx",
            "-arch=sm_60",
            "-o",
            cuda_dir.join("ashmaize.ptx").to_str().unwrap(),
            cuda_dir.join("ashmaize.cu").to_str().unwrap(),
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "nvcc compilation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let ptx_path = cuda_dir.join("ashmaize.ptx");
    let ptx_string = std::fs::read_to_string(ptx_path)?;
    
    Ok(CudaPtx::from_src(&ptx_string))
}

pub fn hash_gpu_or_cpu(salt: &[u8], rom: &Rom, nb_loops: u32, nb_instrs: u32) -> [u8; 64] {
    match CudaAshmaize::new() {
        Ok(cuda) => match cuda.hash_parallel(&[salt], rom, nb_loops, nb_instrs) {
            Ok(results) => results.into_iter().next().unwrap(),
            Err(_) => ashmaize::b2::hash(salt, rom, nb_loops, nb_instrs),
        },
        Err(_) => ashmaize::b2::hash(salt, rom, nb_loops, nb_instrs),
    }
}

pub fn hash_gpu(salt: &[u8], rom: &Rom, nb_loops: u32, nb_instrs: u32) -> GpuResult<[u8; 64]> {
    let cuda = CudaAshmaize::new()?;
    let results = cuda.hash_parallel(&[salt], rom, nb_loops, nb_instrs)?;
    Ok(results.into_iter().next().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ashmaize::RomGenerationType;

    #[test]
    fn test_gpu_hash_basic() {
        let rom = Rom::new(
            b"test_seed",
            RomGenerationType::TwoStep {
                pre_size: 1024,
                mixing_numbers: 4,
            },
            10_240,
        );

        let salt = b"test_salt";
        let result = hash_gpu_or_cpu(salt, &rom, 8, 256);

        assert_eq!(result.len(), 64);
        assert!(!result.iter().all(|&x| x == 0));
    }

    #[test]
    fn test_gpu_hash_consistency() {
        let rom = Rom::new(
            b"test_seed",
            RomGenerationType::TwoStep {
                pre_size: 1024,
                mixing_numbers: 4,
            },
            10_240,
        );

        let salt = b"consistent_test";
        let result1 = hash_gpu_or_cpu(salt, &rom, 8, 256);
        let result2 = hash_gpu_or_cpu(salt, &rom, 8, 256);

        assert_eq!(result1, result2);
    }

    #[test]
    fn test_gpu_hash_vs_cpu() {
        let rom = Rom::new(
            b"comparison_seed",
            RomGenerationType::TwoStep {
                pre_size: 1024,
                mixing_numbers: 4,
            },
            10_240,
        );

        let salt = b"comparison_test";
        let cpu_result = ashmaize::b2::hash(salt, &rom, 8, 256);
        
        if let Ok(gpu_result) = hash_gpu(salt, &rom, 8, 256) {
            assert_eq!(cpu_result, gpu_result);
        }
    }
}

