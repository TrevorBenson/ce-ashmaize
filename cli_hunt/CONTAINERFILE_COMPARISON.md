# Containerfile Comparison Guide

Quick guide to help you choose the right Containerfile for your needs.

## Available Containerfiles

| File | Purpose | Base Image | Size | Build Time | GPU Support |
|------|---------|------------|------|------------|-------------|
| `Containerfile` | CPU-only testing | Fedora 43 | ~2GB | ~5 min | ❌ No |
| `Containerfile.cuda` | GPU testing (Fedora) | nvidia/cuda:12.6.0-devel-fedora39 | ~5GB | ~8 min | ✅ Yes |
| `Containerfile.cuda-ubuntu` | GPU testing (Ubuntu) | nvidia/cuda:12.6.0-devel-ubuntu22.04 | ~5GB | ~8 min | ✅ Yes |
| `Containerfile.fedora43-cuda` | GPU testing (manual CUDA) | Fedora 43 + CUDA install | ~8GB | ~15 min | ✅ Yes |

## Which One Should You Use?

### For GPU Testing (Recommended) ⭐

**Choose `Containerfile.cuda` if:**
- ✅ You prefer Fedora/RHEL ecosystem
- ✅ You use `dnf` package manager
- ✅ You want smaller image size
- ✅ You want faster builds

**Choose `Containerfile.cuda-ubuntu` if:**
- ✅ You prefer Ubuntu/Debian ecosystem
- ✅ You use `apt` package manager
- ✅ Your team is more familiar with Ubuntu
- ✅ You want smaller image size

**Both are equally good - just pick your preferred distro!**

### For CPU-Only Testing

**Choose `Containerfile` if:**
- ✅ You only need CPU solver
- ✅ You want smallest image
- ✅ You don't have NVIDIA GPU
- ✅ You want fastest build time

### Advanced: Manual CUDA Installation

**Choose `Containerfile.fedora43-cuda` if:**
- ⚠️ You specifically need Fedora 43 base
- ⚠️ You want to customize CUDA installation
- ⚠️ You're okay with larger image and slower build

**Note:** This is generally NOT recommended unless you have specific requirements.

## Quick Start Commands

### Fedora-based GPU (Recommended for Fedora users)

```bash
# Build
podman build -f Containerfile.cuda -t ashmaize-solver:cuda-fedora .

# Test
podman run --rm --device nvidia.com/gpu=all ashmaize-solver:cuda-fedora \
  /ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver test-hash \
  --nonce 001af01e65703909 \
  --address addr1q84h0q756f6fslk9y3v48kztxug9nk2es3wvw3dyumfy2qwvpuzhn97jay38vh4sspz45ukzavalsm0tf6q4gx39rl8sc7f5rf \
  --challenge-id "**D01C17" \
  --difficulty 00007FFF \
  --no-pre-mine e8a195800bae57517c85955a784faa6162051f41ef86bcb93be0c3e01a9b63c8 \
  --latest-submission 2025-10-31T15:59:59.000Z \
  --no-pre-mine-hour 967125414
```

### Ubuntu-based GPU (Recommended for Ubuntu users)

```bash
# Build
podman build -f Containerfile.cuda-ubuntu -t ashmaize-solver:cuda-ubuntu .

# Test
podman run --rm --device nvidia.com/gpu=all ashmaize-solver:cuda-ubuntu \
  /ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver test-hash \
  --nonce 001af01e65703909 \
  --address addr1q84h0q756f6fslk9y3v48kztxug9nk2es3wvw3dyumfy2qwvpuzhn97jay38vh4sspz45ukzavalsm0tf6q4gx39rl8sc7f5rf \
  --challenge-id "**D01C17" \
  --difficulty 00007FFF \
  --no-pre-mine e8a195800bae57517c85955a784faa6162051f41ef86bcb93be0c3e01a9b63c8 \
  --latest-submission 2025-10-31T15:59:59.000Z \
  --no-pre-mine-hour 967125414
```

### CPU-Only

```bash
# Build
podman build -f Containerfile -t ashmaize-solver:cpu .

# Test (no GPU flag needed)
podman run --rm ashmaize-solver:cpu \
  /ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver \
  --address <addr> ...
```

## Detailed Comparison

### Package Managers

| Containerfile | Package Manager | Update Command | Install Command |
|---------------|----------------|----------------|-----------------|
| `Containerfile.cuda` | dnf (Fedora) | `dnf update` | `dnf install` |
| `Containerfile.cuda-ubuntu` | apt (Ubuntu) | `apt update` | `apt install` |
| `Containerfile` | dnf (Fedora) | `dnf update` | `dnf install` |
| `Containerfile.fedora43-cuda` | dnf (Fedora) | `dnf update` | `dnf install` |

### CUDA Configuration

| Containerfile | CUDA Source | CUDA Version | nvcc Location |
|---------------|-------------|--------------|---------------|
| `Containerfile.cuda` | Pre-installed in base | 12.6.0 | `/usr/local/cuda/bin/nvcc` |
| `Containerfile.cuda-ubuntu` | Pre-installed in base | 12.6.0 | `/usr/local/cuda/bin/nvcc` |
| `Containerfile` | N/A | N/A | N/A |
| `Containerfile.fedora43-cuda` | NVIDIA repo (manual) | 12.6 | `/usr/local/cuda-12.6/bin/nvcc` |

### Build Dependencies

| Containerfile | Rust Install | Python (uv) | Build Tools |
|---------------|-------------|-------------|-------------|
| `Containerfile.cuda` | From Fedora repos | ✅ Yes | gcc, make |
| `Containerfile.cuda-ubuntu` | From rustup.rs | ✅ Yes | build-essential |
| `Containerfile` | From Fedora repos | ✅ Yes | rust (built-in) |
| `Containerfile.fedora43-cuda` | From Fedora repos | ✅ Yes | gcc, make |

### Build Output

All GPU-enabled Containerfiles produce a binary at:
```
/ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver
```

With CUDA support enabled via `--features cuda` flag.

## Performance Comparison

**All GPU Containerfiles have identical performance:**
- Same CUDA 12.6 toolkit
- Same compilation flags
- Same CUDA kernel code
- Same runtime behavior

**The only differences are:**
- Base OS (Fedora vs Ubuntu)
- Package management
- Image size and build time (due to different base layers)

## Troubleshooting

### Fedora-based Issues

```bash
# Check CUDA in Fedora container
podman run --rm ashmaize-solver:cuda-fedora nvcc --version
podman run --rm ashmaize-solver:cuda-fedora dnf list installed | grep cuda
```

### Ubuntu-based Issues

```bash
# Check CUDA in Ubuntu container
podman run --rm ashmaize-solver:cuda-ubuntu nvcc --version
podman run --rm ashmaize-solver:cuda-ubuntu apt list --installed | grep cuda
```

### Build Failures

**Fedora issues:**
- DNF repository errors: Check internet connection and NVIDIA repos
- Missing packages: Ensure `rust` and `cargo` are available

**Ubuntu issues:**
- APT update errors: Run `apt update` first
- Missing packages: Install `build-essential` and `curl`

## Migration Between Containerfiles

If you need to switch from one Containerfile to another:

```bash
# Example: Switch from Fedora to Ubuntu version

# 1. Stop any running containers
podman stop $(podman ps -q --filter ancestor=ashmaize-solver:cuda-fedora)

# 2. Build new version
podman build -f Containerfile.cuda-ubuntu -t ashmaize-solver:cuda-ubuntu .

# 3. Test new version
podman run --rm --device nvidia.com/gpu=all ashmaize-solver:cuda-ubuntu \
  /ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver --help

# 4. Remove old image (optional)
podman rmi ashmaize-solver:cuda-fedora
```

## Best Practices

1. **Choose based on familiarity**: Use the distro you know best
2. **Stick with NVIDIA base images**: `Containerfile.cuda` or `Containerfile.cuda-ubuntu`
3. **Avoid manual CUDA installation**: Unless you have specific requirements
4. **Tag your images clearly**: Use descriptive tags like `cuda-fedora` or `cuda-ubuntu`
5. **Test before production**: Always run validation tests with `test-hash` command

## Summary

**For most users:**
- **Fedora users** → `Containerfile.cuda`
- **Ubuntu users** → `Containerfile.cuda-ubuntu`
- **CPU only** → `Containerfile`

Both CUDA Containerfiles are excellent choices with identical performance. Pick the one that matches your preferred Linux distribution!

## Additional Resources

- Full container testing guide: `CONTAINER_GPU_TESTING.md`
- RTX 5000 series quick start: `rust_solver/QUICKSTART_RTX5000.md`
- General documentation: `rust_solver/README.md`

