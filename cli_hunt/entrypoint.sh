#!/bin/bash
# Entrypoint script to set up CUDA library symlink for cudarc
# cudarc expects libcuda.so (without version), but the driver provides libcuda.so.1
# This script creates the necessary symlink at runtime

# Create symlink if it doesn't exist
if [ ! -f /usr/local/cuda/lib64/libcuda.so ]; then
    if [ -f /lib/x86_64-linux-gnu/libcuda.so.1 ]; then
        ln -sf /lib/x86_64-linux-gnu/libcuda.so.1 /usr/local/cuda/lib64/libcuda.so
        echo "Created symlink: /usr/local/cuda/lib64/libcuda.so -> /lib/x86_64-linux-gnu/libcuda.so.1"
    elif [ -f /usr/lib/x86_64-linux-gnu/libcuda.so.1 ]; then
        ln -sf /usr/lib/x86_64-linux-gnu/libcuda.so.1 /usr/local/cuda/lib64/libcuda.so
        echo "Created symlink: /usr/local/cuda/lib64/libcuda.so -> /usr/lib/x86_64-linux-gnu/libcuda.so.1"
    else
        echo "Warning: libcuda.so.1 not found. CUDA may not work properly."
    fi
fi

# Execute the command passed to the container
exec "$@"

