# CUDA Version Selection Notes

## Summary

The project uses **CUDA 12.6** instead of CUDA 13.0 due to Rust ecosystem compatibility.

## Why CUDA 12.6?

### Current Status (November 2024)
- **cudarc** version 0.12.x (the Rust CUDA bindings used) only supports up to CUDA 12.x
- The `cuda-12000` feature flag is the latest available in cudarc 0.12.x
- CUDA 13.0 support requires cudarc 0.13+ which was not available at the time of implementation

### Technical Details
```toml
# Current configuration in Cargo.toml
cudarc = { version = "0.12", features = ["cuda-12000", "driver", "nvrtc"], default-features = false, optional = true }
```

### Container Images
- **Ubuntu 24.04**: `docker.io/nvidia/cuda:12.6.0-devel-ubuntu24.04`
- **Fedora 39**: `docker.io/nvidia/cuda:12.6.0-devel-fedora39`

## GPU Compatibility

CUDA 12.6 with compute capability sm_75 (Turing and newer) supports:

### RTX 5000 Series (Ada Lovelace/Blackwell)
- RTX 5090 (sm_89 or newer)
- RTX 5080 (sm_89 or newer)
- RTX 5070 Ti (sm_89 or newer)
- RTX 5070 (sm_89 or newer)

### RTX 4000 Series (Ada Lovelace)  
- RTX 4090 (sm_89)
- RTX 4080 (sm_89)
- RTX 4070 Ti (sm_89)
- RTX 4070 (sm_89)
- RTX 4060 Ti (sm_89)
- RTX 4060 (sm_89)

### RTX 3000 Series (Ampere)
- RTX 3090 Ti (sm_86)
- RTX 3090 (sm_86)
- RTX 3080 Ti (sm_86)
- RTX 3080 (sm_86)
- RTX 3070 Ti (sm_86)
- RTX 3070 (sm_86)
- RTX 3060 Ti (sm_86)
- RTX 3060 (sm_86)

### RTX 2000 Series (Turing)
- RTX 2080 Ti (sm_75)
- RTX 2080 (sm_75)
- RTX 2070 (sm_75)
- RTX 2060 (sm_75)

### Data Center GPUs
- A100 (sm_80)
- A40 (sm_86)
- A30 (sm_80)
- A10 (sm_86)
- V100 (sm_70)
- T4 (sm_75)

## Future Migration to CUDA 13.x

When cudarc adds CUDA 13.x support:

1. Update `Cargo.toml`:
```toml
cudarc = { version = "0.13+", features = ["cuda-13000", "driver", "nvrtc"], default-features = false, optional = true }
```

2. Update Containerfiles:
```dockerfile
FROM docker.io/nvidia/cuda:13.0.0-devel-ubuntu24.04
```

3. Update build scripts if needed:
```rust
// build.rs - update architecture if targeting newer GPUs
"-arch=sm_89",  // For Ada Lovelace/Blackwell
```

## Additional Notes

### Why default-features = false?
We disable default features to avoid pulling in macOS-specific dependencies (`metal`, `core-graphics-types`, etc.) that cause compilation errors on Linux.

### Required cudarc Features
- `cuda-12000`: CUDA 12.x API support
- `driver`: CUDA driver API (CudaDevice, CudaFunction, etc.)
- `nvrtc`: Runtime compilation support (Ptx type)

## Building Without GPU Support

For CPU-only builds, simply omit the `--features cuda` flag:

```bash
cargo build --release
```

The solver will automatically fall back to CPU-only mode when CUDA is not available.

