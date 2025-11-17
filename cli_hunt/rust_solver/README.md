# Ashmaize GPU Solver

High-performance Ashmaize hash solver with CUDA GPU acceleration for NVIDIA GPUs on Linux.

## Features

- **CPU Solver**: Multi-threaded CPU implementation using rayon
- **GPU Solver**: CUDA-accelerated implementation for NVIDIA GPUs
- **Hybrid Mode**: Combined CPU+GPU solving
- **Benchmarking**: Compare CPU vs GPU performance
- **Test Harness**: Validate hash correctness

## Requirements

### CPU-Only Mode

- Rust 1.70 or later
- Linux, macOS, or Windows

### GPU Mode (CUDA)

- **CUDA Toolkit**: Version 11.8 LTS or 12.6+ recommended
- **NVIDIA GPU**: Compute capability 6.0+ (Pascal architecture or newer)
  - Tested: RTX 3080, RTX 4090, A100
  - Minimum: GTX 1060, GTX 1070, etc.
- **NVIDIA Driver**: Compatible with CUDA version
  - CUDA 11.8: Driver ≥ 450.80.02
  - CUDA 12.6: Driver ≥ 525.60.13
- **nvcc**: CUDA compiler (included in CUDA Toolkit)
- **Linux**: Ubuntu 20.04+, Fedora 36+, or compatible

## Installation

### 1. Install CUDA Toolkit

#### Ubuntu/Debian:
```bash
# CUDA 12.6 (recommended)
wget https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2204/x86_64/cuda-keyring_1.1-1_all.deb
sudo dpkg -i cuda-keyring_1.1-1_all.deb
sudo apt update
sudo apt install cuda-toolkit-12-6

# Add to PATH
echo 'export PATH=/usr/local/cuda-12.6/bin:$PATH' >> ~/.bashrc
echo 'export LD_LIBRARY_PATH=/usr/local/cuda-12.6/lib64:$LD_LIBRARY_PATH' >> ~/.bashrc
source ~/.bashrc
```

#### Fedora/RHEL:
```bash
sudo dnf config-manager --add-repo https://developer.download.nvidia.com/compute/cuda/repos/fedora37/x86_64/cuda-fedora37.repo
sudo dnf install cuda-toolkit-12-6
```

#### Verify Installation:
```bash
nvcc --version
nvidia-smi
```

### 2. Build the Solver

#### CPU-Only:
```bash
cargo build --release
```

#### With GPU Support:
```bash
cargo build --release --features cuda
```

## Usage

### Solve a Challenge (CPU)

```bash
./target/release/ashmaize-solver \
  --address addr1q84h0q756f6fslk9y3v48kztxug9nk2es3wvw3dyumfy2qwvpuzhn97jay38vh4sspz45ukzavalsm0tf6q4gx39rl8sc7f5rf \
  --challenge-id "**D01C17" \
  --difficulty 00007FFF \
  --no-pre-mine e8a195800bae57517c85955a784faa6162051f41ef86bcb93be0c3e01a9b63c8 \
  --latest-submission 2025-10-31T15:59:59.000Z \
  --no-pre-mine-hour 967125414
```

### Solve with GPU

```bash
./target/release/ashmaize-solver \
  --use-gpu \
  --gpu-batch-size 2048 \
  --address <address> \
  --challenge-id <id> \
  --difficulty <diff> \
  --no-pre-mine <npm> \
  --latest-submission <ls> \
  --no-pre-mine-hour <npmh>
```

**GPU Options:**
- `--use-gpu`: Enable GPU acceleration (requires `cuda` feature)
- `--gpu-batch-size <N>`: Number of hashes per GPU batch (default: 1024)
  - Smaller (512): Lower latency, faster solution finding
  - Larger (4096): Higher throughput, better GPU utilization

### Benchmark GPU Performance

```bash
# GPU benchmark only (10 seconds)
./target/release/ashmaize-solver benchmark \
  --address <address> \
  --challenge-id <id> \
  --difficulty <diff> \
  --no-pre-mine <npm> \
  --latest-submission <ls> \
  --no-pre-mine-hour <npmh> \
  --duration-secs 10 \
  --batch-size 1024

# CPU vs GPU comparison
./target/release/ashmaize-solver benchmark \
  --address <address> \
  --challenge-id <id> \
  --difficulty <diff> \
  --no-pre-mine <npm> \
  --latest-submission <ls> \
  --no-pre-mine-hour <npmh> \
  --duration-secs 10 \
  --batch-size 1024 \
  --compare
```

### Validate Hash Correctness

Test a single hash computation to verify GPU matches CPU:

```bash
./target/release/ashmaize-solver test-hash \
  --address <address> \
  --challenge-id <id> \
  --difficulty <diff> \
  --no-pre-mine <npm> \
  --latest-submission <ls> \
  --no-pre-mine-hour <npmh> \
  --nonce 001af01e65703909
```

Expected output:
```
Testing single hash with nonce: 001af01e65703909
CPU result: 00001a3f...
GPU result: 00001a3f...
✓ Results match!
```

## Container Usage (Docker/Podman)

### Prerequisites

**Docker:**
- Install [NVIDIA Container Toolkit](https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/install-guide.html)
- Requires Docker 19.03+ and NVIDIA Driver

**Podman:**
- Podman 3.2+ (built-in CDI support for GPU)
- NVIDIA Driver and nvidia-container-toolkit
- No additional daemon required

### Setup NVIDIA Container Toolkit

#### Ubuntu/Debian:
```bash
# Add NVIDIA package repository
curl -fsSL https://nvidia.github.io/libnvidia-container/gpgkey | sudo gpg --dearmor -o /usr/share/keyrings/nvidia-container-toolkit-keyring.gpg
curl -s -L https://nvidia.github.io/libnvidia-container/stable/deb/nvidia-container-toolkit.list | \
  sed 's#deb https://#deb [signed-by=/usr/share/keyrings/nvidia-container-toolkit-keyring.gpg] https://#g' | \
  sudo tee /etc/apt/sources.list.d/nvidia-container-toolkit.list

# Install
sudo apt update
sudo apt install -y nvidia-container-toolkit

# Configure Docker (if using Docker)
sudo nvidia-ctk runtime configure --runtime=docker
sudo systemctl restart docker

# Configure Podman (if using Podman)
sudo nvidia-ctk cdi generate --output=/etc/cdi/nvidia.yaml
```

#### Fedora/RHEL:
```bash
# Add NVIDIA package repository
curl -s -L https://nvidia.github.io/libnvidia-container/stable/rpm/nvidia-container-toolkit.repo | \
  sudo tee /etc/yum.repos.d/nvidia-container-toolkit.repo

# Install
sudo dnf install -y nvidia-container-toolkit

# Configure (same as Ubuntu)
sudo nvidia-ctk runtime configure --runtime=docker  # For Docker
sudo nvidia-ctk cdi generate --output=/etc/cdi/nvidia.yaml  # For Podman
```

### Verify GPU Access in Container

```bash
# Docker
docker run --rm --gpus all nvidia/cuda:12.6.0-base-ubuntu22.04 nvidia-smi

# Podman
podman run --rm --device nvidia.com/gpu=all nvidia/cuda:12.6.0-base-ubuntu22.04 nvidia-smi
```

### Dockerfile Example

```dockerfile
FROM nvidia/cuda:12.6.0-devel-ubuntu22.04

# Install Rust
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Copy source
WORKDIR /app
COPY . .

# Build with GPU support
RUN cargo build --release --features cuda

# Default command
ENTRYPOINT ["/app/target/release/ashmaize-solver"]
```

### Building Container Image

```bash
# Build image
docker build -t ashmaize-solver:cuda .

# Or with Podman
podman build -t ashmaize-solver:cuda .
```

### Running Solver in Container

#### Docker:

```bash
# Solve with GPU
docker run --rm --gpus all ashmaize-solver:cuda \
  --use-gpu \
  --address <address> \
  --challenge-id <id> \
  --difficulty <diff> \
  --no-pre-mine <npm> \
  --latest-submission <ls> \
  --no-pre-mine-hour <npmh>

# Benchmark GPU
docker run --rm --gpus all ashmaize-solver:cuda \
  benchmark --compare --duration-secs 30 \
  --address <address> \
  --challenge-id <id> \
  --difficulty <diff> \
  --no-pre-mine <npm> \
  --latest-submission <ls> \
  --no-pre-mine-hour <npmh>

# Test hash correctness
docker run --rm --gpus all ashmaize-solver:cuda \
  test-hash --nonce 001af01e65703909 \
  --address <address> \
  --challenge-id <id> \
  --difficulty <diff> \
  --no-pre-mine <npm> \
  --latest-submission <ls> \
  --no-pre-mine-hour <npmh>
```

#### Podman:

```bash
# Solve with GPU
podman run --rm --device nvidia.com/gpu=all ashmaize-solver:cuda \
  --use-gpu \
  --address <address> \
  --challenge-id <id> \
  --difficulty <diff> \
  --no-pre-mine <npm> \
  --latest-submission <ls> \
  --no-pre-mine-hour <npmh>

# Benchmark GPU
podman run --rm --device nvidia.com/gpu=all ashmaize-solver:cuda \
  benchmark --compare --duration-secs 30 \
  --address <address> \
  --challenge-id <id> \
  --difficulty <diff> \
  --no-pre-mine <npm> \
  --latest-submission <ls> \
  --no-pre-mine-hour <npmh>

# Test hash correctness
podman run --rm --device nvidia.com/gpu=all ashmaize-solver:cuda \
  test-hash --nonce 001af01e65703909 \
  --address <address> \
  --challenge-id <id> \
  --difficulty <diff> \
  --no-pre-mine <npm> \
  --latest-submission <ls> \
  --no-pre-mine-hour <npmh>
```

### Container GPU Options

**Docker:**
- `--gpus all` - All GPUs
- `--gpus '"device=0"'` - GPU 0 only
- `--gpus '"device=0,1"'` - GPUs 0 and 1
- `--gpus 2` - First 2 GPUs

**Podman:**
- `--device nvidia.com/gpu=all` - All GPUs
- `--device nvidia.com/gpu=0` - GPU 0 only
- `--device nvidia.com/gpu=0,1` - GPUs 0 and 1

### Troubleshooting Containers

**GPU Not Detected:**
```bash
# Check CDI devices (Podman)
nvidia-ctk cdi list

# Check Docker runtime
docker info | grep -i nvidia

# Verify host GPU
nvidia-smi
```

**Build Failures:**
- Ensure CUDA toolkit in container: Use `nvidia/cuda:*-devel-*` base images
- Check nvcc: `docker run --rm <image> nvcc --version`
- Verify CUDA_PATH: Should be set in CUDA base images

**Runtime Errors:**
- Check driver compatibility: Container driver ≤ Host driver
- Verify permissions: User in `docker`/`podman` group
- Check SELinux (Fedora): May need `--security-opt label=disable`

### Advanced: Multi-GPU Container Setup

```bash
# Run one container per GPU
docker run -d --gpus '"device=0"' --name solver-gpu0 ashmaize-solver:cuda ...
docker run -d --gpus '"device=1"' --name solver-gpu1 ashmaize-solver:cuda ...

# Or with Podman
podman run -d --device nvidia.com/gpu=0 --name solver-gpu0 ashmaize-solver:cuda ...
podman run -d --device nvidia.com/gpu=1 --name solver-gpu1 ashmaize-solver:cuda ...
```

### Container Performance Notes

1. **Minimal Overhead**: GPU passthrough has <1% overhead
2. **Same Performance**: Container GPU performance ≈ bare metal
3. **Isolation**: Each container can use different GPUs
4. **Portability**: Same image works on any CUDA-capable system

## Performance

### Expected Hash Rates

| Hardware | Hash Rate | Speedup vs CPU |
|----------|-----------|----------------|
| CPU (5 threads, Ryzen 9) | ~280 H/s | 1x baseline |
| NVIDIA GTX 1080 Ti | ~3,000 H/s | ~11x |
| NVIDIA RTX 3080 | ~8,000 H/s | ~29x |
| NVIDIA RTX 4090 | ~15,000 H/s | ~54x |
| NVIDIA RTX 5080 | ~18,000 H/s | ~64x |
| NVIDIA RTX 5090 | ~25,000 H/s | ~89x |
| NVIDIA A100 | ~12,000 H/s | ~43x |

*Note: Actual performance depends on specific hardware, CUDA version, and optimization.*

### Optimization Tips

1. **Batch Size**: Tune `--gpu-batch-size` based on your GPU
   - RTX 3080/4090: 2048-4096
   - GTX 1660/1070: 1024-2048
   - Profile with benchmarks to find optimal value

2. **Difficulty**: Lower difficulty benefits more from GPU (more parallelism)

3. **Multiple GPUs**: Run multiple instances, one per GPU:
   ```bash
   CUDA_VISIBLE_DEVICES=0 ./ashmaize-solver --use-gpu ... &
   CUDA_VISIBLE_DEVICES=1 ./ashmaize-solver --use-gpu ... &
   ```

4. **Power Limit**: Ensure GPU isn't thermally throttled
   ```bash
   nvidia-smi -q -d TEMPERATURE,POWER
   ```

5. **See** `CUDA_OPTIMIZATION_NOTES.md` for advanced profiling and optimization

## Architecture

### CUDA Implementation

The solver ports the Ashmaize algorithm to CUDA with:

1. **Blake2b Hash**: Full GPU implementation with compression rounds
2. **Argon2 H'**: Variable-length KDF for program shuffling
3. **VM Execution**: Complete Ashmaize VM with all operations:
   - Op3: Add, Mul, MulH, Xor, Div, Mod, And, Hash
   - Op2: ISqrt, Neg, BitRev, RotL, RotR
4. **ROM Access**: 1GB read-only memory with digest updates
5. **Parallel Execution**: One thread per hash, embarrassingly parallel

### File Structure

```
cli_hunt/rust_solver/
├── cuda/
│   ├── ashmaize.cu         # Main CUDA kernel
│   ├── blake2b.cuh         # Blake2b implementation
│   ├── argon2.cuh          # Argon2 H' function
│   └── ashmaize_vm.cuh     # VM structures and operations
├── src/
│   ├── main.rs             # CLI and solver logic
│   ├── gpu.rs              # Rust-CUDA bindings
│   ├── benchmark.rs        # Benchmarking utilities
│   └── tests.rs            # Test cases
├── build.rs                # CUDA build script
└── Cargo.toml
```

## Troubleshooting

### CUDA Not Found

**Error**: `nvcc: command not found`

**Solution**:
```bash
export PATH=/usr/local/cuda/bin:$PATH
export LD_LIBRARY_PATH=/usr/local/cuda/lib64:$LD_LIBRARY_PATH
```

### Compilation Fails

**Error**: `CUDA compilation failed`

**Check**:
1. `nvcc --version` shows correct version
2. GPU compute capability: `nvidia-smi --query-gpu=compute_cap --format=csv`
3. If compute capability < 6.0, edit `build.rs` and `gpu.rs` to use `-arch=sm_50`

### GPU Not Detected

**Error**: `Failed to initialize GPU`

**Check**:
1. NVIDIA driver: `nvidia-smi`
2. CUDA runtime: `ldconfig -p | grep libcudart`
3. Permissions: Add user to `video` group
   ```bash
   sudo usermod -a -G video $USER
   ```

### Poor Performance

**Check**:
1. GPU utilization: `nvidia-smi dmon -s u`
2. Thermal throttling: `nvidia-smi -q -d TEMPERATURE`
3. Profile kernel: `ncu --set full ./ashmaize-solver ...`
4. See `CUDA_OPTIMIZATION_NOTES.md`

### Results Don't Match CPU

**Solution**:
1. Use `test-hash` command to validate
2. Check CUDA floating point modes
3. Verify Blake2b test vectors pass
4. File an issue with details

## Testing

### Run CPU Tests
```bash
cargo test
```

### Run GPU Tests (requires CUDA)
```bash
cargo test --features cuda
```

### Validate Against Known Solution
```bash
cargo test --features cuda validate_example_solution
```

## Development

### Build for Development
```bash
cargo build --features cuda
```

### Profile with Nsight Compute
```bash
ncu --set full --export profile.ncu-rep \
  ./target/release/ashmaize-solver --use-gpu ...
```

### Modify Kernel
1. Edit `cuda/*.cu` or `cuda/*.cuh`
2. Rebuild: `cargo build --features cuda`
3. Test: `cargo test --features cuda`

## Contributing

1. Ensure CPU and GPU results match using `test-hash`
2. Run benchmarks before and after changes
3. Profile with Nsight Compute for performance changes
4. Update documentation

## License

Same as parent project (ashmaize)

## References

- [Ashmaize Specification](../../SPECS.md)
- [CUDA Programming Guide](https://docs.nvidia.com/cuda/cuda-c-programming-guide/)
- [Blake2 Hash Function](https://www.blake2.net/)
- [Argon2 RFC](https://datatracker.ietf.org/doc/html/rfc9106)

