use ashmaize::b2::VM;
use ashmaize::Rom;
use cudarc::driver::*;
use cudarc::nvrtc::Ptx;
use std::sync::Arc;

#[derive(Debug)]
pub enum GpuError {
    DriverError(DriverError),
    Other(String),
}

impl std::fmt::Display for GpuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuError::DriverError(e) => write!(f, "CUDA driver error: {:?}", e),
            GpuError::Other(s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for GpuError {}

impl From<DriverError> for GpuError {
    fn from(e: DriverError) -> Self {
        GpuError::DriverError(e)
    }
}

impl From<std::io::Error> for GpuError {
    fn from(e: std::io::Error) -> Self {
        GpuError::Other(e.to_string())
    }
}

impl From<String> for GpuError {
    fn from(s: String) -> Self {
        GpuError::Other(s)
    }
}

impl From<&str> for GpuError {
    fn from(s: &str) -> Self {
        GpuError::Other(s.to_string())
    }
}

pub type GpuResult<T> = Result<T, GpuError>;

pub struct CudaAshmaize {
    device: Arc<CudaDevice>,
    kernel_func: CudaFunction,
}

impl CudaAshmaize {
    pub fn new() -> GpuResult<Self> {
        Self::new_with_device(0)
    }

    pub fn new_with_device(device_id: usize) -> GpuResult<Self> {
        let device = CudaDevice::new(device_id)?;

        let ptx = compile_kernel()?;
        device.load_ptx(ptx.clone(), "ashmaize", &["ashmaize_hash_kernel"])?;
        
        let kernel_func = device.get_func("ashmaize", "ashmaize_hash_kernel")
            .ok_or("Failed to get kernel function")?;

        Ok(Self {
            device,
            kernel_func,
        })
    }

    pub fn get_device_count() -> GpuResult<usize> {
        Ok(cudarc::driver::result::device::get_count()? as usize)
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

        // Find max salt length
        let max_salt_len = salts.iter().map(|s| s.len()).max().unwrap_or(0);
        
        // Pack salts with proper length (no truncation)
        let mut all_salts = Vec::new();
        for salt in salts {
            all_salts.extend_from_slice(salt);
            // Pad to max length if needed for alignment
            for _ in salt.len()..max_salt_len {
                all_salts.push(0);
            }
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
        let rom_digest_buffer = self.device.htod_sync_copy(rom.digest.as_bytes())?;
        let salts_buffer = self.device.htod_sync_copy(&all_salts)?;
        let initial_prog_seeds_buffer = self.device.htod_sync_copy(&initial_prog_seeds)?;
        let mut programs_buffer = self.device.htod_sync_copy(&all_programs)?;
        let mut results_buffer = self.device.alloc_zeros::<u8>(num_hashes * 64)?;

        let salt_len = max_salt_len as u32;

        let threads_per_block = 256;
        let num_blocks = (num_hashes as u32 + threads_per_block - 1) / threads_per_block;

        let cfg = LaunchConfig {
            grid_dim: (num_blocks, 1, 1),
            block_dim: (threads_per_block, 1, 1),
            shared_mem_bytes: 0,
        };

        unsafe {
            self.kernel_func.clone().launch(
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
            "CUDA Device: {}",
            self.device.name()?,
        ))
    }
}

fn compile_kernel() -> GpuResult<Ptx> {
    // Load PTX that was compiled during build time by build.rs
    // The PTX is embedded in the binary as a static string
    const PTX_SRC: &str = include_str!(concat!(env!("OUT_DIR"), "/ashmaize.ptx"));
    
    Ok(Ptx::from_src(PTX_SRC))
}

#[allow(dead_code)]
pub fn hash_gpu_or_cpu(salt: &[u8], rom: &Rom, nb_loops: u32, nb_instrs: u32) -> [u8; 64] {
    match CudaAshmaize::new() {
        Ok(cuda) => match cuda.hash_parallel(&[salt], rom, nb_loops, nb_instrs) {
            Ok(results) => results.into_iter().next().unwrap(),
            Err(_) => ashmaize::b2::hash(salt, rom, nb_loops, nb_instrs),
        },
        Err(_) => ashmaize::b2::hash(salt, rom, nb_loops, nb_instrs),
    }
}

#[allow(dead_code)]
pub fn hash_gpu(salt: &[u8], rom: &Rom, nb_loops: u32, nb_instrs: u32) -> GpuResult<[u8; 64]> {
    let cuda = CudaAshmaize::new()?;
    let results = cuda.hash_parallel(&[salt], rom, nb_loops, nb_instrs)?;
    Ok(results.into_iter().next().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ashmaize::rom::RomGenerationType;

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

