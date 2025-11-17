# Quick Start Guide for RTX 5000 Series

This guide provides streamlined instructions for testing the CUDA Ashmaize solver on NVIDIA RTX 5000 series GPUs (RTX 5080, RTX 5090).

## System Requirements

- **GPU**: NVIDIA RTX 5080/5090 (Blackwell/Ada Lovelace architecture)
- **Driver**: NVIDIA Driver 560.0+ (for RTX 5000 series)
- **CUDA**: 12.6 or later (RTX 5000 requires CUDA 12.4+)
- **OS**: Ubuntu 22.04+, Fedora 39+, or compatible Linux

## Quick Install

```bash
# 1. Verify GPU is detected
nvidia-smi

# 2. Install CUDA 12.6 (if not already installed)
wget https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2204/x86_64/cuda-keyring_1.1-1_all.deb
sudo dpkg -i cuda-keyring_1.1-1_all.deb
sudo apt update
sudo apt install cuda-toolkit-12-6

# 3. Set environment
export PATH=/usr/local/cuda-12.6/bin:$PATH
export LD_LIBRARY_PATH=/usr/local/cuda-12.6/lib64:$LD_LIBRARY_PATH

# 4. Verify CUDA
nvcc --version
```

## Build and Test

```bash
# Clone or navigate to repository
cd cli_hunt/rust_solver

# Build with CUDA support
cargo build --release --features cuda

# Verify build
./target/release/ashmaize-solver --version
```

## Quick Tests

### 1. Validate Hash Correctness

Test with the known example solution:

```bash
./target/release/ashmaize-solver test-hash \
  --address addr1q84h0q756f6fslk9y3v48kztxug9nk2es3wvw3dyumfy2qwvpuzhn97jay38vh4sspz45ukzavalsm0tf6q4gx39rl8sc7f5rf \
  --challenge-id "**D01C17" \
  --difficulty 00007FFF \
  --no-pre-mine e8a195800bae57517c85955a784faa6162051f41ef86bcb93be0c3e01a9b63c8 \
  --latest-submission 2025-10-31T15:59:59.000Z \
  --no-pre-mine-hour 967125414 \
  --nonce 001af01e65703909
```

**Expected output:**
```
Testing single hash with nonce: 001af01e65703909
CPU result: 00001a3f...
GPU result: 00001a3f...
✓ Results match!
```

### 2. Benchmark GPU Performance

10-second GPU-only benchmark:

```bash
./target/release/ashmaize-solver benchmark \
  --address addr1q84h0q756f6fslk9y3v48kztxug9nk2es3wvw3dyumfy2qwvpuzhn97jay38vh4sspz45ukzavalsm0tf6q4gx39rl8sc7f5rf \
  --challenge-id "**D01C17" \
  --difficulty 00007FFF \
  --no-pre-mine e8a195800bae57517c85955a784faa6162051f41ef86bcb93be0c3e01a9b63c8 \
  --latest-submission 2025-10-31T15:59:59.000Z \
  --no-pre-mine-hour 967125414 \
  --duration-secs 10 \
  --batch-size 2048
```

### 3. CPU vs GPU Comparison

30-second comparison benchmark:

```bash
./target/release/ashmaize-solver benchmark \
  --address addr1q84h0q756f6fslk9y3v48kztxug9nk2es3wvw3dyumfy2qwvpuzhn97jay38vh4sspz45ukzavalsm0tf6q4gx39rl8sc7f5rf \
  --challenge-id "**D01C17" \
  --difficulty 00007FFF \
  --no-pre-mine e8a195800bae57517c85955a784faa6162051f41ef86bcb93be0c3e01a9b63c8 \
  --latest-submission 2025-10-31T15:59:59.000Z \
  --no-pre-mine-hour 967125414 \
  --duration-secs 30 \
  --batch-size 2048 \
  --compare
```

**Expected for RTX 5090:**
```
CPU Benchmark Results:
  Hash Rate: ~280 H/s

GPU Benchmark Results:
  Hash Rate: ~25,000 H/s

GPU Speedup: ~89x faster than CPU
```

### 4. Run Actual Solver

Solve with GPU (replace with actual challenge parameters):

```bash
./target/release/ashmaize-solver \
  --use-gpu \
  --gpu-batch-size 4096 \
  --address <your_address> \
  --challenge-id <challenge_id> \
  --difficulty <difficulty_hex> \
  --no-pre-mine <npm_hex> \
  --latest-submission <timestamp> \
  --no-pre-mine-hour <hour>
```

## Container Testing (Docker)

### Setup

```bash
# Install NVIDIA Container Toolkit
curl -fsSL https://nvidia.github.io/libnvidia-container/gpgkey | sudo gpg --dearmor -o /usr/share/keyrings/nvidia-container-toolkit-keyring.gpg
curl -s -L https://nvidia.github.io/libnvidia-container/stable/deb/nvidia-container-toolkit.list | \
  sed 's#deb https://#deb [signed-by=/usr/share/keyrings/nvidia-container-toolkit-keyring.gpg] https://#g' | \
  sudo tee /etc/apt/sources.list.d/nvidia-container-toolkit.list

sudo apt update
sudo apt install -y nvidia-container-toolkit
sudo nvidia-ctk runtime configure --runtime=docker
sudo systemctl restart docker

# Verify
docker run --rm --gpus all nvidia/cuda:12.6.0-base-ubuntu22.04 nvidia-smi
```

### Build Container

```bash
# Create Dockerfile (see README.md for full example)
docker build -t ashmaize-solver:cuda .
```

### Run in Container

```bash
# Validate hash
docker run --rm --gpus all ashmaize-solver:cuda \
  test-hash --nonce 001af01e65703909 \
  --address addr1q84h0q756f6fslk9y3v48kztxug9nk2es3wvw3dyumfy2qwvpuzhn97jay38vh4sspz45ukzavalsm0tf6q4gx39rl8sc7f5rf \
  --challenge-id "**D01C17" \
  --difficulty 00007FFF \
  --no-pre-mine e8a195800bae57517c85955a784faa6162051f41ef86bcb93be0c3e01a9b63c8 \
  --latest-submission 2025-10-31T15:59:59.000Z \
  --no-pre-mine-hour 967125414

# Benchmark
docker run --rm --gpus all ashmaize-solver:cuda \
  benchmark --compare --duration-secs 30 \
  --address addr1q84h0q756f6fslk9y3v48kztxug9nk2es3wvw3dyumfy2qwvpuzhn97jay38vh4sspz45ukzavalsm0tf6q4gx39rl8sc7f5rf \
  --challenge-id "**D01C17" \
  --difficulty 00007FFF \
  --no-pre-mine e8a195800bae57517c85955a784faa6162051f41ef86bcb93be0c3e01a9b63c8 \
  --latest-submission 2025-10-31T15:59:59.000Z \
  --no-pre-mine-hour 967125414
```

## Container Testing (Podman)

### Setup

```bash
# Install NVIDIA Container Toolkit
sudo dnf install -y nvidia-container-toolkit  # Fedora
# or
sudo apt install -y nvidia-container-toolkit  # Ubuntu

# Generate CDI configuration
sudo nvidia-ctk cdi generate --output=/etc/cdi/nvidia.yaml

# Verify
podman run --rm --device nvidia.com/gpu=all nvidia/cuda:12.6.0-base-ubuntu22.04 nvidia-smi
```

### Build and Run

```bash
# Build
podman build -t ashmaize-solver:cuda .

# Run (same as Docker, just replace --gpus with --device)
podman run --rm --device nvidia.com/gpu=all ashmaize-solver:cuda \
  test-hash --nonce 001af01e65703909 \
  --address addr1q84h0q756f6fslk9y3v48kztxug9nk2es3wvw3dyumfy2qwvpuzhn97jay38vh4sspz45ukzavalsm0tf6q4gx39rl8sc7f5rf \
  --challenge-id "**D01C17" \
  --difficulty 00007FFF \
  --no-pre-mine e8a195800bae57517c85955a784faa6162051f41ef86bcb93be0c3e01a9b63c8 \
  --latest-submission 2025-10-31T15:59:59.000Z \
  --no-pre-mine-hour 967125414
```

## Optimization for RTX 5000 Series

### Batch Size Tuning

RTX 5000 series has significantly more CUDA cores than previous generations:
- **RTX 5080**: ~10,240 cores
- **RTX 5090**: ~21,760 cores

Recommended batch sizes:
- **RTX 5080**: 2048-4096
- **RTX 5090**: 4096-8192

Test different batch sizes:

```bash
for batch in 1024 2048 4096 8192; do
  echo "Testing batch size: $batch"
  ./target/release/ashmaize-solver benchmark \
    --batch-size $batch \
    --duration-secs 10 \
    --address <addr> ... # (other params)
done
```

### Monitor GPU Utilization

```bash
# Real-time monitoring
watch -n 1 nvidia-smi

# Or detailed stats
nvidia-smi dmon -s pucvmet
```

Target: **>95% GPU utilization** for optimal performance

## Troubleshooting

### GPU Not Detected

```bash
# Check driver
nvidia-smi

# Check CUDA
nvcc --version

# Check device
./target/release/ashmaize-solver --help
# Should show --use-gpu option if built with cuda feature
```

### Poor Performance

```bash
# Check power limit
nvidia-smi -q -d POWER

# Check temperature
nvidia-smi -q -d TEMPERATURE

# Check clocks
nvidia-smi -q -d CLOCK
```

RTX 5000 series may throttle if:
- Temperature >83°C
- Power limit reached
- Insufficient cooling

### Build Errors

```bash
# Verify CUDA installation
ls -la /usr/local/cuda-12.6/bin/nvcc

# Check cargo features
cargo build --release --features cuda --verbose
```

## Performance Expectations

### RTX 5080
- **Hash Rate**: ~18,000 H/s
- **Speedup**: ~64x vs CPU
- **Power**: ~320W typical

### RTX 5090
- **Hash Rate**: ~25,000 H/s
- **Speedup**: ~89x vs CPU
- **Power**: ~450W typical

*Note: These are estimates. Actual performance may vary based on cooling, power limits, and system configuration.*

## Next Steps

1. **Validate Correctness**: Run `test-hash` command first
2. **Benchmark**: Use `benchmark --compare` to measure speedup
3. **Profile**: Use Nsight Compute for optimization (see `CUDA_OPTIMIZATION_NOTES.md`)
4. **Deploy**: Use actual challenge parameters for production solving

## Support

- Full documentation: `README.md`
- Optimization guide: `CUDA_OPTIMIZATION_NOTES.md`
- Implementation details: `CUDA_IMPLEMENTATION_SUMMARY.md`

## Quick Reference

```bash
# Test hash correctness
./target/release/ashmaize-solver test-hash --nonce <nonce> ...

# Benchmark (10 seconds)
./target/release/ashmaize-solver benchmark --duration-secs 10 ...

# Compare CPU vs GPU (30 seconds)
./target/release/ashmaize-solver benchmark --compare --duration-secs 30 ...

# Solve with GPU
./target/release/ashmaize-solver --use-gpu --gpu-batch-size 4096 ...

# Docker
docker run --rm --gpus all ashmaize-solver:cuda test-hash ...

# Podman
podman run --rm --device nvidia.com/gpu=all ashmaize-solver:cuda test-hash ...
```

