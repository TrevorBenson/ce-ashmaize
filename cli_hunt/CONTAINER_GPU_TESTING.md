# Container GPU Testing Guide

This guide explains how to build and test the CUDA-enabled Ashmaize solver using containers (Podman or Docker).

## CUDA Version

**Current Version:** CUDA 12.6

The project uses CUDA 12.6 (not 13.0) because the Rust `cudarc` crate version 0.12.x only supports CUDA up to 12.x. See `CUDA_VERSION_NOTES.md` for details.

## Containerfile Options

Four Containerfiles are provided:

1. **`Containerfile`** (original) - CPU-only, Fedora 43 base
2. **`Containerfile.cuda`** (recommended for Fedora users) - GPU support, nvidia/cuda:12.6.0-devel-fedora39 base
3. **`Containerfile.cuda-ubuntu`** (recommended for Ubuntu users) - GPU support, nvidia/cuda:12.6.0-devel-ubuntu22.04 base
4. **`Containerfile.fedora43-cuda`** - GPU support, Fedora 43 + CUDA installation (larger, slower build)

## Recommended: Use Containerfile.cuda or Containerfile.cuda-ubuntu

The `Containerfile.cuda` (Fedora) and `Containerfile.cuda-ubuntu` (Ubuntu) are recommended because:
- ✅ Smaller image size (~5GB vs ~8GB)
- ✅ Faster build time
- ✅ Pre-configured CUDA environment
- ✅ Maintained by NVIDIA
- ✅ Guaranteed compatibility

**Choose based on your preference:**
- **Fedora-based**: `Containerfile.cuda` - uses dnf, familiar if you use Fedora/RHEL
- **Ubuntu-based**: `Containerfile.cuda-ubuntu` - uses apt, familiar if you use Ubuntu/Debian

## Prerequisites

### For Podman (Recommended on Fedora)

```bash
# Install Podman (if not already installed)
sudo dnf install -y podman

# Install NVIDIA Container Toolkit
sudo dnf install -y nvidia-container-toolkit

# Generate CDI configuration for GPU access
sudo nvidia-ctk cdi generate --output=/etc/cdi/nvidia.yaml

# Verify GPU access
podman run --rm --device nvidia.com/gpu=all \
  nvidia/cuda:12.6.0-base-fedora39 nvidia-smi
```

### For Docker

```bash
# Install Docker (if not already installed)
sudo dnf install -y docker

# Install NVIDIA Container Toolkit
sudo dnf install -y nvidia-container-toolkit

# Configure Docker runtime
sudo nvidia-ctk runtime configure --runtime=docker
sudo systemctl restart docker

# Verify GPU access
docker run --rm --gpus all \
  nvidia/cuda:12.6.0-base-fedora39 nvidia-smi
```

## Building the Container Image

### Option 1: Build with GPU Support (Recommended)

**Using Podman:**
```bash
cd /path/to/ce-ashmaize/cli_hunt

# Build from Containerfile.cuda (Fedora-based)
podman build -f Containerfile.cuda -t ashmaize-solver:cuda-fedora .

# Or build from Containerfile.cuda-ubuntu (Ubuntu-based)
podman build -f Containerfile.cuda-ubuntu -t ashmaize-solver:cuda-ubuntu .

# Or build from Containerfile.fedora43-cuda (Fedora 43 + CUDA, larger)
podman build -f Containerfile.fedora43-cuda -t ashmaize-solver:cuda-f43 .
```

**Using Docker:**
```bash
cd /path/to/ce-ashmaize/cli_hunt

# Build from Containerfile.cuda (Fedora-based)
docker build -f Containerfile.cuda -t ashmaize-solver:cuda-fedora .

# Or build from Containerfile.cuda-ubuntu (Ubuntu-based)
docker build -f Containerfile.cuda-ubuntu -t ashmaize-solver:cuda-ubuntu .

# Or build from Containerfile.fedora43-cuda (Fedora 43 + CUDA, larger)
docker build -f Containerfile.fedora43-cuda -t ashmaize-solver:cuda-f43 .
```

### Option 2: Build CPU-Only

```bash
# Using Podman
podman build -f Containerfile -t ashmaize-solver:cpu .

# Using Docker
docker build -f Containerfile -t ashmaize-solver:cpu .
```

## Running Tests

### 1. Verify Build

**Podman:**
```bash
# Check that solver was built with CUDA support
podman run --rm ashmaize-solver:cuda \
  /ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver --help

# Should show --use-gpu option if CUDA is enabled
```

**Docker:**
```bash
docker run --rm ashmaize-solver:cuda \
  /ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver --help
```

### 2. Validate Hash Correctness

Test with known example solution to verify GPU produces correct results:

**Podman:**
```bash
podman run --rm --device nvidia.com/gpu=all ashmaize-solver:cuda \
  /ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver test-hash \
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

**Docker:**
```bash
docker run --rm --gpus all ashmaize-solver:cuda \
  /ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver test-hash \
  --address addr1q84h0q756f6fslk9y3v48kztxug9nk2es3wvw3dyumfy2qwvpuzhn97jay38vh4sspz45ukzavalsm0tf6q4gx39rl8sc7f5rf \
  --challenge-id "**D01C17" \
  --difficulty 00007FFF \
  --no-pre-mine e8a195800bae57517c85955a784faa6162051f41ef86bcb93be0c3e01a9b63c8 \
  --latest-submission 2025-10-31T15:59:59.000Z \
  --no-pre-mine-hour 967125414 \
  --nonce 001af01e65703909
```

### 3. Benchmark GPU Performance

10-second GPU benchmark:

**Podman:**
```bash
podman run --rm --device nvidia.com/gpu=all ashmaize-solver:cuda \
  /ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver benchmark \
  --address addr1q84h0q756f6fslk9y3v48kztxug9nk2es3wvw3dyumfy2qwvpuzhn97jay38vh4sspz45ukzavalsm0tf6q4gx39rl8sc7f5rf \
  --challenge-id "**D01C17" \
  --difficulty 00007FFF \
  --no-pre-mine e8a195800bae57517c85955a784faa6162051f41ef86bcb93be0c3e01a9b63c8 \
  --latest-submission 2025-10-31T15:59:59.000Z \
  --no-pre-mine-hour 967125414 \
  --duration-secs 10 \
  --batch-size 2048
```

**Docker:**
```bash
docker run --rm --gpus all ashmaize-solver:cuda \
  /ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver benchmark \
  --address addr1q84h0q756f6fslk9y3v48kztxug9nk2es3wvw3dyumfy2qwvpuzhn97jay38vh4sspz45ukzavalsm0tf6q4gx39rl8sc7f5rf \
  --challenge-id "**D01C17" \
  --difficulty 00007FFF \
  --no-pre-mine e8a195800bae57517c85955a784faa6162051f41ef86bcb93be0c3e01a9b63c8 \
  --latest-submission 2025-10-31T15:59:59.000Z \
  --no-pre-mine-hour 967125414 \
  --duration-secs 10 \
  --batch-size 2048
```

### 4. CPU vs GPU Comparison

30-second comparison benchmark:

**Podman:**
```bash
podman run --rm --device nvidia.com/gpu=all ashmaize-solver:cuda \
  /ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver benchmark \
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

**Docker:**
```bash
docker run --rm --gpus all ashmaize-solver:cuda \
  /ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver benchmark \
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

**Expected output (RTX 5090):**
```
=== CPU vs GPU Benchmark ===

CPU Benchmark Results:
  Total Hashes: 8,400
  Duration: 30.00s
  Hash Rate: 280.00 H/s

GPU Benchmark Results:
  Total Hashes: 750,000
  Duration: 30.00s
  Hash Rate: 25,000.00 H/s

=== Comparison ===
GPU Speedup: 89.29x faster than CPU
```

### 5. Interactive Container Access

For debugging or manual testing:

**Podman:**
```bash
# Start interactive shell with GPU access
podman run -it --device nvidia.com/gpu=all ashmaize-solver:cuda /bin/bash

# Inside container:
cd /ce-ashmaize/cli_hunt/rust_solver
./target/release/ashmaize-solver --help
nvidia-smi
```

**Docker:**
```bash
docker run -it --gpus all ashmaize-solver:cuda /bin/bash
```

## Batch Size Tuning for Your GPU

Test different batch sizes to find optimal performance:

**Podman:**
```bash
for batch in 1024 2048 4096 8192; do
  echo "Testing batch size: $batch"
  podman run --rm --device nvidia.com/gpu=all ashmaize-solver:cuda \
    /ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver benchmark \
    --batch-size $batch \
    --duration-secs 10 \
    --address addr1q84h0q756f6fslk9y3v48kztxug9nk2es3wvw3dyumfy2qwvpuzhn97jay38vh4sspz45ukzavalsm0tf6q4gx39rl8sc7f5rf \
    --challenge-id "**D01C17" \
    --difficulty 00007FFF \
    --no-pre-mine e8a195800bae57517c85955a784faa6162051f41ef86bcb93be0c3e01a9b63c8 \
    --latest-submission 2025-10-31T15:59:59.000Z \
    --no-pre-mine-hour 967125414
done
```

Recommended batch sizes:
- **RTX 5080**: 2048-4096
- **RTX 5090**: 4096-8192
- **A100**: 2048-4096

## Monitoring GPU in Container

**Podman:**
```bash
# In separate terminal, monitor GPU usage
watch -n 1 nvidia-smi

# Or run nvidia-smi inside container
podman run --rm --device nvidia.com/gpu=all ashmaize-solver:cuda nvidia-smi
```

**Docker:**
```bash
watch -n 1 nvidia-smi

docker run --rm --gpus all ashmaize-solver:cuda nvidia-smi
```

## Troubleshooting

### GPU Not Detected in Container

**Check host GPU:**
```bash
nvidia-smi
```

**Verify CDI configuration (Podman):**
```bash
nvidia-ctk cdi list
# Should show: nvidia.com/gpu=all, nvidia.com/gpu=0, etc.
```

**Verify Docker runtime:**
```bash
docker info | grep -i nvidia
# Should show nvidia runtime
```

### Build Failures

**CUDA not found during build:**
- Ensure using `Containerfile.cuda` (nvidia/cuda base)
- OR ensure CUDA installed in `Containerfile.fedora43-cuda`

**nvcc compilation fails:**
```bash
# Test nvcc in container
podman run --rm ashmaize-solver:cuda nvcc --version
# Should show CUDA 12.6.x
```

### Runtime Errors

**libcudart.so not found:**
- Check LD_LIBRARY_PATH in container
- Verify CUDA base image or installation

**Permission denied:**
```bash
# Add user to group (Podman)
sudo usermod -aG video $USER
newgrp video

# Or run with sudo (not recommended)
```

### Performance Issues

**Low GPU utilization (<50%):**
- Increase batch size: `--batch-size 4096`
- Check thermal throttling: `nvidia-smi -q -d TEMPERATURE`
- Check power limit: `nvidia-smi -q -d POWER`

**Slower than expected:**
- Verify GPU is actually being used: Run with `--use-gpu` flag
- Compare CPU-only vs GPU in same container
- Check if running in compatibility mode

## Multi-GPU Testing

Run separate containers for each GPU:

**Podman:**
```bash
# GPU 0
podman run -d --name solver-gpu0 \
  --device nvidia.com/gpu=0 \
  ashmaize-solver:cuda \
  /ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver \
  --use-gpu --gpu-batch-size 4096 ...

# GPU 1
podman run -d --name solver-gpu1 \
  --device nvidia.com/gpu=1 \
  ashmaize-solver:cuda \
  /ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver \
  --use-gpu --gpu-batch-size 4096 ...
```

**Docker:**
```bash
docker run -d --name solver-gpu0 --gpus '"device=0"' ashmaize-solver:cuda ...
docker run -d --name solver-gpu1 --gpus '"device=1"' ashmaize-solver:cuda ...
```

## Image Size Comparison

| Containerfile | Base | Size | Build Time |
|---------------|------|------|------------|
| `Containerfile` | Fedora 43 | ~2GB | ~5 min |
| `Containerfile.cuda` | nvidia/cuda:12.6.0-devel-fedora39 | ~5GB | ~8 min |
| `Containerfile.cuda-ubuntu` | nvidia/cuda:12.6.0-devel-ubuntu22.04 | ~5GB | ~8 min |
| `Containerfile.fedora43-cuda` | Fedora 43 + CUDA | ~8GB | ~15 min |

## Quick Reference

```bash
# Build Fedora-based (Podman)
podman build -f Containerfile.cuda -t ashmaize-solver:cuda-fedora .

# Build Ubuntu-based (Podman)
podman build -f Containerfile.cuda-ubuntu -t ashmaize-solver:cuda-ubuntu .

# Build Fedora-based (Docker)
docker build -f Containerfile.cuda -t ashmaize-solver:cuda-fedora .

# Build Ubuntu-based (Docker)
docker build -f Containerfile.cuda-ubuntu -t ashmaize-solver:cuda-ubuntu .

# Test hash (Podman)
podman run --rm --device nvidia.com/gpu=all ashmaize-solver:cuda \
  /ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver test-hash --nonce <nonce> ...

# Test hash (Docker)
docker run --rm --gpus all ashmaize-solver:cuda \
  /ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver test-hash --nonce <nonce> ...

# Benchmark (Podman)
podman run --rm --device nvidia.com/gpu=all ashmaize-solver:cuda \
  /ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver benchmark --compare ...

# Benchmark (Docker)
docker run --rm --gpus all ashmaize-solver:cuda \
  /ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver benchmark --compare ...
```

## Next Steps

1. ✅ Build container: `podman build -f Containerfile.cuda -t ashmaize-solver:cuda .`
2. ✅ Verify build: Check `--help` shows `--use-gpu` option
3. ✅ Validate correctness: Run `test-hash` command
4. ✅ Benchmark performance: Run `benchmark --compare`
5. ✅ Optimize batch size: Test different values
6. ✅ Deploy: Use for actual challenge solving

## Additional Resources

- Full solver documentation: `rust_solver/README.md`
- RTX 5000 series guide: `rust_solver/QUICKSTART_RTX5000.md`
- Optimization guide: `rust_solver/CUDA_OPTIMIZATION_NOTES.md`
- Implementation details: `rust_solver/CUDA_IMPLEMENTATION_SUMMARY.md`

