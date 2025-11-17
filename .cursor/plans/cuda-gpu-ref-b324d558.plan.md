<!-- b324d558-45ac-43ce-9f58-26c29eb2ba22 ce409ffb-d5ff-4ff9-a291-2d0470e71d86 -->
# GPU Auto-Detection and Multi-Mode Solver Integration

## Overview

Enhance the solver to automatically detect and use all available GPUs by default, compile for multiple RTX generations, and add flexible solver modes controlled via `--solver-mode` flag in the Python orchestrator.

## Key Design Decisions

- **Architecture Support**: Compile PTX for sm_86 (RTX 3000), sm_89 (RTX 4000), sm_90 (RTX 5000)
- **Default Behavior**: Auto-detect and use GPUs when available, CPU as fallback
- **Mixed Mode**: Single process running CPU threads (5 by default) + all GPUs simultaneously
- **User Control**: `--solver-mode` with values: `cpu`, `gpu`, `auto`, `mixed`
- **Optimal Batch Size**: 131,072 per GPU (proven to achieve 286k H/s on 4x RTX 4090)
- **Execution Model**: Async workers (one thread per GPU) with clone-based data distribution

## Optimization Status

**PHASE 1-5 OPTIMIZATION COMPLETE** ✅ | **PHASE 6 PLANNED** ⏳

- **Current Peak Performance**: **285,996 H/s** (4x RTX 4090)
- **Per GPU**: **71,499 H/s**
- **Optimal Configuration (Phase 5)**: **Async + Clone + 131,072 batch per GPU**
- **Production-Ready Reference**: `cli_hunt/rust_solver/examples/multi_gpu_final.rs`
- **Phase 6**: Advanced GPU optimizations planned (target: 300-430k+ H/s)

See `cli_hunt/OPTIMIZATION_COMPLETE_STATUS.md` and `cli_hunt/PHASE6_ADVANCED_OPTIMIZATION_PLAN.md` for full details.

## Implementation Steps

### 1. Update Rust Build System (`cli_hunt/rust_solver/build.rs`)

**Current Issue**: Hardcoded `-arch=sm_89` only works for RTX 4000 series.

**Fix**: Compile fat binary with multiple architectures:

```rust
// Change line 30 from:
"-arch=sm_89",

// To:
"-gencode=arch=compute_86,code=sm_86",  // RTX 3000
"-gencode=arch=compute_89,code=sm_89",  // RTX 4000
"-gencode=arch=compute_90,code=sm_90",  // RTX 5000 (Blackwell uses sm_90)
```

Note: RTX 5000 series uses compute capability 9.0 (sm_90), not sm_100.

### 2. Add Solver Mode Argument (`cli_hunt/rust_solver/src/main.rs`)

**Add new argument** to `Args` struct (after line 50):

```rust
#[arg(long, default_value = "auto", value_parser = ["cpu", "gpu", "auto", "mixed"])]
solver_mode: String,
```

**Remove/deprecate** old `--use-gpu` flag (keep for backwards compatibility but mark deprecated).

**Clarification on NUM_THREADS**: This constant (line 9) is **CPU-only**. It controls Rayon thread pool size for CPU mining and has **zero impact on GPU**. In mixed mode, this determines how many CPU threads run alongside GPUs.

### 3. Implement Solver Mode Logic (`cli_hunt/rust_solver/src/main.rs`)

**Refactor `solve()` function** (starts around line 180) to support modes:

```rust
fn solve(/* existing params */, solver_mode: &str) -> Option<u64> {
    let rom = init_rom(no_pre_mine_hex);
    
    match solver_mode {
        "cpu" => solve_cpu_only(&rom, /* params */),
        "gpu" => solve_gpu_only(&rom, /* params */),
        "auto" => {
            // Try GPU first, fallback to CPU
            if cfg!(feature = "cuda") {
                match CudaAshmaize::get_device_count() {
                    Ok(count) if count > 0 => solve_gpu_only(&rom, /* params */),
                    _ => {
                        eprintln!("No GPU available, falling back to CPU");
                        solve_cpu_only(&rom, /* params */)
                    }
                }
            } else {
                solve_cpu_only(&rom, /* params */)
            }
        },
        "mixed" => solve_mixed(&rom, /* params */),
        _ => panic!("Invalid solver mode"),
    }
}
```

### 4. Implement GPU-Only Solver (`cli_hunt/rust_solver/src/main.rs`)

**IMPORTANT**: Use `cli_hunt/rust_solver/examples/multi_gpu_final.rs` as the reference implementation. This has been tested and proven to achieve 286k H/s.

**Key Constants** (add to top of main.rs):

```rust
const NUM_THREADS: u64 = 5;  // CPU threads only
const OPTIMAL_BATCH_PER_GPU: usize = 131_072;  // Proven optimal from testing
```

**Create `solve_gpu_only()` function** following multi_gpu_final.rs pattern:

```rust
#[cfg(feature = "cuda")]
fn solve_gpu_only(rom: &Rom, difficulty_mask: u32, suffix: &str) -> Option<u64> {
    use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
    use std::sync::mpsc;
    use std::thread;
    
    // 1. Detect and initialize GPUs
    let gpu_count = match CudaAshmaize::get_device_count() {
        Ok(count) if count > 0 => count,
        _ => {
            eprintln!("No GPUs detected");
            return None;
        }
    };
    
    let mut gpus = Vec::new();
    for gpu_id in 0..gpu_count {
        match CudaAshmaize::new_with_device(gpu_id) {
            Ok(c) => gpus.push(Arc::new(c)),
            Err(e) => {
                eprintln!("Failed to init GPU {}: {:?}", gpu_id, e);
                return None;
            }
        }
    }
    
    let batch_per_gpu = OPTIMAL_BATCH_PER_GPU;
    let found = Arc::new(AtomicBool::new(false));
    let result_nonce = Arc::new(AtomicU64::new(0));
    
    // 2. Result channel
    let (result_tx, result_rx) = mpsc::channel::<(usize, Result<Vec<[u8; 64]>, String>)>();
    
    // 3. Spawn async GPU workers (one thread per GPU)
    let mut handles = Vec::new();
    for (gpu_id, cuda) in gpus.iter().enumerate() {
        let cuda = Arc::clone(cuda);
        let rom = Arc::new(rom.clone());
        let suffix = suffix.to_string();
        let result_tx = result_tx.clone();
        let found = Arc::clone(&found);
        let result_nonce = Arc::clone(&result_nonce);
        
        let handle = thread::spawn(move || {
            let base_nonce = (gpu_id as u64) * (batch_per_gpu as u64);
            let mut local_nonce = base_nonce;
            
            while !found.load(Ordering::Relaxed) {
                // Generate batch of salts for this GPU
                let mut batch_salts = Vec::with_capacity(batch_per_gpu);
                for i in 0..batch_per_gpu {
                    let nonce = local_nonce + i as u64;
                    let preimage = format!("{:016x}{}", nonce, suffix);
                    batch_salts.push(preimage.into_bytes());
                }
                
                // Clone for this iteration (proven faster than zero-copy)
                let salt_refs: Vec<&[u8]> = batch_salts.iter()
                    .map(|s| s.as_slice())
                    .collect();
                
                // Hash batch
                let result = cuda.hash_parallel(&salt_refs, &rom, 8, 256)
                    .map_err(|e| format!("{:?}", e));
                
                // Check results
                if let Ok(hashes) = result {
                    for (i, hash) in hashes.iter().enumerate() {
                        if hash_structure_good(hash, difficulty_mask) {
                            let winning_nonce = local_nonce + i as u64;
                            found.store(true, Ordering::Relaxed);
                            result_nonce.store(winning_nonce, Ordering::Relaxed);
                            return;
                        }
                    }
                }
                
                local_nonce += (batch_per_gpu * gpu_count) as u64;
            }
        });
        handles.push(handle);
    }
    
    // 4. Wait for solution
    for handle in handles {
        handle.join().ok();
    }
    
    if found.load(Ordering::Relaxed) {
        Some(result_nonce.load(Ordering::Relaxed))
    } else {
        None
    }
}
```

### 5. Implement Mixed Mode Solver (`cli_hunt/rust_solver/src/main.rs`)

**Create `solve_mixed()` function** combining CPU + GPU:

```rust
#[cfg(feature = "cuda")]
fn solve_mixed(rom: &Rom, difficulty_mask: u32, suffix: &str) -> Option<u64> {
    use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
    use rayon::prelude::*;
    
    let gpu_count = CudaAshmaize::get_device_count().unwrap_or(0);
    if gpu_count == 0 {
        eprintln!("No GPUs available for mixed mode, falling back to CPU only");
        return solve_cpu_only(rom, difficulty_mask, suffix);
    }
    
    let found = Arc::new(AtomicBool::new(false));
    let result_nonce = Arc::new(AtomicU64::new(0));
    
    // Spawn GPU workers (same as solve_gpu_only)
    let gpu_handles = /* ... spawn GPU workers as above ... */;
    
    // Spawn CPU workers (NUM_THREADS = 5, using existing CPU logic)
    let cpu_found = Arc::clone(&found);
    let cpu_result = Arc::clone(&result_nonce);
    let rom_cpu = Arc::new(rom.clone());
    let suffix_cpu = suffix.to_string();
    
    let cpu_handle = thread::spawn(move || {
        let start_nonce = (gpu_count as u64 + 1) * 1_000_000_000;  // Offset from GPUs
        
        (0..NUM_THREADS).into_par_iter().for_each(|thread_id| {
            let mut local_nonce = start_nonce + thread_id;
            let stride = NUM_THREADS;
            
            while !cpu_found.load(Ordering::Relaxed) {
                let preimage = format!("{:016x}{}", local_nonce, suffix_cpu);
                let hash = hash(preimage.as_bytes(), &rom_cpu, 8, 256);
                
                if hash_structure_good(&hash, difficulty_mask) {
                    cpu_found.store(true, Ordering::Relaxed);
                    cpu_result.store(local_nonce, Ordering::Relaxed);
                    break;
                }
                
                local_nonce += stride;
            }
        });
    });
    
    // Wait for first to finish (GPU or CPU)
    for handle in gpu_handles {
        handle.join().ok();
    }
    cpu_handle.join().ok();
    
    if found.load(Ordering::Relaxed) {
        Some(result_nonce.load(Ordering::Relaxed))
    } else {
        None
    }
}
```

### 5. Update GPU Module (`cli_hunt/rust_solver/src/gpu.rs`)

**Add helper for multi-GPU solving**:

```rust
pub fn solve_all_gpus(
    salts: Vec<Vec<u8>>,
    rom: &Rom,
    nb_loops: u32,
    nb_instrs: u32,
) -> GpuResult<Vec<[u8; 64]>> {
    let gpu_count = Self::get_device_count()?;
    let batch_per_gpu = salts.len() / gpu_count;
    
    // Distribute work across GPUs (existing multi-GPU logic from examples)
    // Return aggregated results
}
```

### 6. Update Python Orchestrator (`cli_hunt/python_orchestrator/main.py`)

**Add `--solver-mode` argument** to `run_parser` (after line 760):

```python
run_parser.add_argument(
    "--solver-mode",
    type=str,
    choices=["cpu", "gpu", "auto", "mixed"],
    default="auto",
    help="Solver mode: 'cpu' (CPU only), 'gpu' (GPU only), 'auto' (GPU if available, else CPU), 'mixed' (CPU + GPU together). Default: auto",
)
```

**Pass to worker** in `run_orchestrator()` (around line 706):

```python
worker_args = {
    # existing args...
    "solver_mode": args.solver_mode,
}
```

**Update `_solve_one_challenge()`** to add flag (around line 326):

```python
command = [
    RUST_SOLVER_PATH,
    "--address", address,
    "--challenge-id", c["challengeId"],
    "--difficulty", c["difficulty"],
    "--no-pre-mine", str(c["noPreMine"]),
    "--latest-submission", c["latestSubmission"],
    "--no-pre-mine-hour", str(c["noPreMineHour"]),
    "--solver-mode", solver_mode,  # Add this line
]
```

**Update `solver_worker()` signature** (line 467):

```python
def solver_worker(
    db_manager, stop_event, solve_interval, tui_app, max_solvers, 
    challenge_selection, solver_mode  # Add this parameter
):
```

**Pass `solver_mode`** when calling `_solve_one_challenge()` (around line 542):

```python
future = executor.submit(
    _solve_one_challenge,
    db_manager,
    tui_app,
    stop_event,
    address,
    deepcopy(c),
    solver_mode,  # Add this
)
```

### 7. Testing Plan

**Important Test Considerations**:
- Use timeouts on tests (30-40 seconds for 15-20 second expected duration)
- Previous testing showed Persistent Workers hung, but that optimization was already excluded
- All combination tests with Async + Clone passed successfully

**Test 1: CPU-only mode**

```bash
cd cli_hunt/python_orchestrator
python main.py run --solver-mode cpu --max-solvers 2
```

Expected: Uses 5 CPU threads per solver, no GPU usage.

**Test 2: GPU-only mode (explicit)**

```bash
python main.py run --solver-mode gpu --max-solvers 2
```

Expected: Each solver uses all GPUs, no CPU mining threads.

**Test 3: Auto mode (default)**

```bash
python main.py run --max-solvers 2
```

Expected: Auto-detects GPUs and uses them if available, falls back to CPU otherwise.

**Test 4: Mixed mode**

```bash
python main.py run --solver-mode mixed --max-solvers 1
```

Expected: Single process using 5 CPU threads + all GPUs simultaneously.

**Test 5: Multi-GPU verification**

```bash
cd cli_hunt/rust_solver
cargo run --release --features cuda -- test-hash --solver-mode gpu --address test --challenge-id 1 --difficulty 00000001 --no-pre-mine 0 --latest-submission 0 --no-pre-mine-hour 1 --nonce 0000000000012345
```

Expected: Auto-detects and reports GPU count, produces correct hash.

**Test 6: Fat binary verification**

After rebuild, test on systems with different GPUs:

- RTX 3090: Should use sm_86
- RTX 4090: Should use sm_89  
- RTX 5090 (when available): Should use sm_90

CUDA automatically selects the best architecture at runtime from the fat binary.

### 8. Documentation Updates

**Update `cli_hunt/rust_solver/README.md`**:

- Document `--solver-mode` flag
- Explain `NUM_THREADS` is CPU-only
- Add examples for each mode
- Document fat binary support for RTX 3000/4000/5000

**Update `cli_hunt/python_orchestrator/README.md` or main docstring**:

- Document `--solver-mode` orchestrator argument
- Explain default behavior (auto)
- Provide usage examples

## Summary of User-Facing Changes

**Python Orchestrator:**

- New flag: `--solver-mode {cpu,gpu,auto,mixed}` (default: `auto`)
- Behavior is mostly automatic - users only need to set mode if they want non-default

**Rust Solver:**

- Automatically detects all GPUs (no manual GPU count needed)
- Works on RTX 3000/4000/5000 series cards (automatic)
- `NUM_THREADS` constant only affects CPU mining (unchanged)
- New flag: `--solver-mode` (replaces `--use-gpu`)

**Default Experience:**

```bash
python main.py run
```

Just works! Auto-detects GPUs, uses them if available, falls back to CPU silently.

**Advanced Usage:**

```bash
# Force CPU only (for testing or mixed GPU workloads)
python main.py run --solver-mode cpu

# Use CPU+GPU together (maximize solutions)
python main.py run --solver-mode mixed --max-solvers 1

# GPU only, 4 parallel solvers
python main.py run --solver-mode gpu --max-solvers 4
```

## Files to Modify

1. `cli_hunt/rust_solver/build.rs` - Multi-arch compilation
2. `cli_hunt/rust_solver/src/main.rs` - Solver mode logic, mixed mode implementation
3. `cli_hunt/rust_solver/src/gpu.rs` - Helper functions (optional)
4. `cli_hunt/python_orchestrator/main.py` - Orchestrator arguments and command construction
5. Documentation files (README updates)

## Backwards Compatibility

- Old `--use-gpu` flag can remain for backwards compatibility (internally maps to `--solver-mode gpu`)
- Default behavior is improved (auto-detection) but doesn't break existing usage

### To-dos

- [ ] Update build.rs to compile fat binary for sm_86, sm_89, sm_90
- [ ] Add --solver-mode argument to main.rs Args struct
- [ ] Implement solve() mode dispatch logic (cpu, gpu, auto, mixed)
- [ ] Implement solve_mixed() function for CPU+GPU hybrid
- [ ] Refactor existing solve logic into solve_cpu_only() and solve_gpu_only()
- [ ] Add --solver-mode argument to Python orchestrator run command
- [ ] Pass solver_mode to solver_worker and _solve_one_challenge
- [ ] Update command construction to include --solver-mode flag
- [ ] Test CPU-only mode
- [ ] Test GPU-only mode with multi-GPU detection
- [ ] Test auto mode (GPU with CPU fallback)
- [ ] Test mixed mode (CPU+GPU simultaneous)
- [ ] Verify fat binary works on different RTX generations
- [ ] Update Rust solver README with solver-mode documentation
- [ ] Update Python orchestrator documentation