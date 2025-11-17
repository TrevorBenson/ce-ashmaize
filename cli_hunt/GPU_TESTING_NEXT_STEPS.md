# GPU Testing Next Steps

## ✅ Build Status: SUCCESS

The CUDA-enabled container has been successfully built:
- **Image:** `ashmaize-solver:cuda12-ubuntu24`
- **CUDA Version:** 12.6.0
- **Base:** Ubuntu 24.04
- **Architecture Target:** sm_75 (Turing and newer)

## 🧪 Testing Completed (This Machine)

### Container Build Verification ✅
- [x] Container builds without errors
- [x] Binary runs and shows version: `ashmaize-solver 0.1.0`
- [x] Help commands work correctly
- [x] CUDA kernels compile successfully during build

### Testing Limitations
- ⚠️ This development machine doesn't have NVIDIA drivers loaded
- ⚠️ GPU runtime testing requires a machine with proper GPU setup

## 🚀 Testing on GPU-Enabled Machines

### Prerequisites
Your RTX 3000 series GPU machine needs:
1. NVIDIA drivers installed (version 525+ for CUDA 12.6)
2. nvidia-container-toolkit installed (for Podman/Docker GPU support)
3. Podman or Docker configured for GPU passthrough

### Setup GPU Support (One-Time Setup)

#### For Fedora/RHEL:
```bash
# Install nvidia-container-toolkit
sudo dnf install nvidia-container-toolkit

# Configure Podman for GPU
sudo nvidia-ctk cdi generate --output=/etc/cdi/nvidia.yaml

# Verify CDI devices
nvidia-ctk cdi list
```

#### For Ubuntu/Debian:
```bash
# Install nvidia-container-toolkit
distribution=$(. /etc/os-release;echo $ID$VERSION_ID)
curl -s -L https://nvidia.github.io/libnvidia-container/gpgkey | sudo apt-key add -
curl -s -L https://nvidia.github.io/libnvidia-container/$distribution/libnvidia-container.list | \
  sudo tee /etc/apt/sources.list.d/nvidia-container-toolkit.list

sudo apt-get update
sudo apt-get install -y nvidia-container-toolkit

# Configure Docker/Podman
sudo nvidia-ctk runtime configure --runtime=docker
sudo systemctl restart docker  # if using docker

# For Podman
sudo nvidia-ctk cdi generate --output=/etc/cdi/nvidia.yaml
```

### Transfer Container Image

Option 1 - Save/Load:
```bash
# On build machine (this one)
podman save ashmaize-solver:cuda12-ubuntu24 -o ashmaize-cuda12.tar

# Transfer to GPU machine (scp, rsync, etc.)
scp ashmaize-cuda12.tar user@gpu-machine:/tmp/

# On GPU machine
podman load -i /tmp/ashmaize-cuda12.tar
```

Option 2 - Registry:
```bash
# Push to a registry (Docker Hub, Quay.io, private registry)
podman tag ashmaize-solver:cuda12-ubuntu24 your-registry/ashmaize-solver:cuda12-ubuntu24
podman push your-registry/ashmaize-solver:cuda12-ubuntu24

# On GPU machine
podman pull your-registry/ashmaize-solver:cuda12-ubuntu24
```

### Run Tests with GPU

#### 1. Quick GPU Verification
```bash
# Test GPU detection
podman run --rm \
  --device nvidia.com/gpu=all \
  --security-opt=label=disable \
  ashmaize-solver:cuda12-ubuntu24 \
  nvidia-smi
```

#### 2. Test Hash (CPU Mode First)
```bash
# Run without --use-gpu to verify CPU fallback
podman run --rm \
  --device nvidia.com/gpu=all \
  --security-opt=label=disable \
  ashmaize-solver:cuda12-ubuntu24 \
  /app/ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver \
  test-hash \
  --address addr_test1vr6w0ja6rjz8gc3wvn6hh0v0chcsa2tg2ymzmz9e9x9 \
  --challenge-id 0000000000000000000000000000000000000000000000000000000000000001 \
  --difficulty 12 \
  --no-pre-mine 0 \
  --latest-submission 0 \
  --no-pre-mine-hour 0 \
  --nonce 12345
```

#### 3. Test Hash (GPU Mode)
```bash
# Run with --use-gpu
podman run --rm \
  --device nvidia.com/gpu=all \
  --security-opt=label=disable \
  ashmaize-solver:cuda12-ubuntu24 \
  /app/ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver \
  test-hash \
  --address addr_test1vr6w0ja6rjz8gc3wvn6hh0v0chcsa2tg2ymzmz9e9x9 \
  --challenge-id 0000000000000000000000000000000000000000000000000000000000000001 \
  --difficulty 12 \
  --no-pre-mine 0 \
  --latest-submission 0 \
  --no-pre-mine-hour 0 \
  --nonce 12345 \
  --use-gpu
```

#### 4. Benchmark (Compare CPU vs GPU)
```bash
# CPU benchmark (10 seconds)
podman run --rm \
  --device nvidia.com/gpu=all \
  --security-opt=label=disable \
  ashmaize-solver:cuda12-ubuntu24 \
  /app/ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver \
  benchmark \
  --address addr_test1vr6w0ja6rjz8gc3wvn6hh0v0chcsa2tg2ymzmz9e9x9 \
  --challenge-id 0000000000000000000000000000000000000000000000000000000000000001 \
  --difficulty 12 \
  --no-pre-mine 0 \
  --latest-submission 0 \
  --no-pre-mine-hour 0 \
  --duration-secs 10

# GPU benchmark (10 seconds) with comparison
podman run --rm \
  --device nvidia.com/gpu=all \
  --security-opt=label=disable \
  ashmaize-solver:cuda12-ubuntu24 \
  /app/ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver \
  benchmark \
  --address addr_test1vr6w0ja6rjz8gc3wvn6hh0v0chcsa2tg2ymzmz9e9x9 \
  --challenge-id 0000000000000000000000000000000000000000000000000000000000000001 \
  --difficulty 12 \
  --no-pre-mine 0 \
  --latest-submission 0 \
  --no-pre-mine-hour 0 \
  --duration-secs 10 \
  --compare
```

#### 5. Full Solver Run (GPU)
```bash
podman run --rm \
  --device nvidia.com/gpu=all \
  --security-opt=label=disable \
  ashmaize-solver:cuda12-ubuntu24 \
  /app/ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver \
  --address addr_test1vr6w0ja6rjz8gc3wvn6hh0v0chcsa2tg2ymzmz9e9x9 \
  --challenge-id 0000000000000000000000000000000000000000000000000000000000000001 \
  --difficulty 12 \
  --no-pre-mine 0 \
  --latest-submission 0 \
  --no-pre-mine-hour 0 \
  --use-gpu \
  --gpu-batch-size 4096
```

### Interactive Testing in Container

Start container with bash:
```bash
podman run -it --rm \
  --device nvidia.com/gpu=all \
  --security-opt=label=disable \
  ashmaize-solver:cuda12-ubuntu24 \
  /bin/bash
```

Inside container:
```bash
# Check GPU
nvidia-smi

# Check CUDA device info
cd /app/ce-ashmaize/cli_hunt/rust_solver
cargo run --release --features cuda -- --help

# Run tests
cargo test --release --features cuda

# Run benchmarks with different batch sizes
for size in 512 1024 2048 4096 8192; do
  echo "Testing batch size: $size"
  cargo run --release --features cuda -- \
    benchmark \
    --address addr_test1vr6w0ja6rjz8gc3wvn6hh0v0chcsa2tg2ymzmz9e9x9 \
    --challenge-id 0000000000000000000000000000000000000000000000000000000000000001 \
    --difficulty 12 \
    --no-pre-mine 0 \
    --latest-submission 0 \
    --no-pre-mine-hour 0 \
    --duration-secs 5 \
    --batch-size $size \
    --compare
done
```

## 📊 Expected Results

### RTX 3000 Series (sm_86 Ampere)
Based on similar workloads, expect:
- **RTX 3090**: ~500-800 MH/s (GPU) vs ~10-20 MH/s (CPU)
- **RTX 3080**: ~400-600 MH/s (GPU) vs ~10-20 MH/s (CPU)
- **RTX 3070**: ~300-500 MH/s (GPU) vs ~10-20 MH/s (CPU)
- **RTX 3060 Ti**: ~250-400 MH/s (GPU) vs ~10-20 MH/s (CPU)

*Actual performance depends on:*
- BLAKE2b and Argon2 compute intensity
- Memory bandwidth utilization
- Batch size tuning
- VM instruction complexity

## 🐛 Troubleshooting

### Issue: "Unable to dynamically load the cuda shared library"
**Solution:** Ensure NVIDIA drivers are installed and `/usr/local/cuda/lib64` is in `LD_LIBRARY_PATH` (should be automatic in container).

### Issue: "nvidia.com/gpu=all" not found (CDI error)
**Solution:** 
```bash
# Generate CDI specifications
sudo nvidia-ctk cdi generate --output=/etc/cdi/nvidia.yaml
nvidia-ctk cdi list  # verify
```

### Issue: Permission denied accessing GPU
**Solution:** Add `--security-opt=label=disable` for rootless Podman.

### Issue: Low GPU utilization
**Solution:** 
- Increase `--gpu-batch-size` (try 8192, 16384)
- Check with `nvidia-smi dmon` during run
- Monitor with `nvtop` or `nvidia-smi pmon`

### Issue: Hash results don't match CPU
**Solution:** This would be a bug. Report with:
- GPU model
- CUDA version
- Input parameters
- Expected vs actual hash output

## 📝 Performance Profiling

### Using nvidia-smi
```bash
# Monitor during run (separate terminal)
watch -n 1 nvidia-smi

# Detailed monitoring
nvidia-smi dmon -s u  # utilization
nvidia-smi dmon -s m  # memory
```

### Using NSight Compute (Advanced)
```bash
# Profile a single run
ncu --set full -o profile \
  podman run --rm --device nvidia.com/gpu=all --security-opt=label=disable \
  ashmaize-solver:cuda12-ubuntu24 \
  /app/ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver \
  test-hash <args> --use-gpu

# View results
ncu-ui profile.ncu-rep
```

## ✅ Success Criteria

- [ ] Container runs on GPU machine
- [ ] nvidia-smi shows GPU visible
- [ ] CPU mode produces valid hashes
- [ ] GPU mode produces valid hashes
- [ ] GPU hashes match CPU hashes (same inputs)
- [ ] GPU shows significant speedup (>10x)
- [ ] Benchmark compares CPU vs GPU correctly
- [ ] Different batch sizes work
- [ ] No memory errors or crashes
- [ ] Clean shutdown after Ctrl+C

## 📧 Reporting Results

When testing is complete, please report:
1. GPU model and driver version
2. Hash rate (CPU vs GPU)
3. Optimal batch size
4. Memory usage
5. Any errors or issues encountered

Good luck with testing! 🚀

