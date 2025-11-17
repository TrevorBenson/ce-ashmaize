# Cloud GPU Deployment Guide

This guide covers deploying the CUDA-enabled Ashmaize solver to a cloud server with GPU support.

## Container Image

**Image:** `ghcr.io/trevorbenson/ce-ashmaize:cuda12-ubuntu24`  
**Size:** ~5.2 GB  
**CUDA Version:** 12.6.0  
**Base:** Ubuntu 24.04

---

## Step 1: Push Container to GitHub Container Registry

### Prerequisites
- GitHub account with access to `trevorbenson/ce-ashmaize`
- GitHub Personal Access Token (PAT) with `write:packages` permission
- Podman or Docker installed locally

### Authentication

#### Option A: Using GitHub CLI (Recommended)
```bash
# Login to ghcr.io
echo $GITHUB_TOKEN | podman login ghcr.io -u trevorbenson --password-stdin
# OR
echo $GITHUB_TOKEN | docker login ghcr.io -u trevorbenson --password-stdin
```

#### Option B: Using Personal Access Token
```bash
# Create token at: https://github.com/settings/tokens
# Select scope: write:packages

# Login
podman login ghcr.io -u trevorbenson -p <YOUR_TOKEN>
# OR
docker login ghcr.io -u trevorbenson -p <YOUR_TOKEN>
```

### Tag and Push

```bash
cd /home/illuminatus/Projects/ce-ashmaize

# Tag the image (if not already done)
podman tag ashmaize-solver:cuda12-ubuntu24 ghcr.io/trevorbenson/ce-ashmaize:cuda12-ubuntu24
# OR
docker tag ashmaize-solver:cuda12-ubuntu24 ghcr.io/trevorbenson/ce-ashmaize:cuda12-ubuntu24

# Push to registry
podman push ghcr.io/trevorbenson/ce-ashmaize:cuda12-ubuntu24
# OR
docker push ghcr.io/trevorbenson/ce-ashmaize:cuda12-ubuntu24
```

**Note:** First push may take 5-10 minutes depending on upload speed.

---

## Step 2: Cloud Server Setup

### Prerequisites on Cloud Server
- Ubuntu 20.04+ or similar Linux distribution
- NVIDIA GPU (RTX 3000/4000/5000 series, Tesla T4, A100, etc.)
- NVIDIA drivers installed (version 525+ for CUDA 12.6)
- Docker installed and configured
- nvidia-container-toolkit installed

### Install Docker (if not present)

```bash
# Update package index
sudo apt-get update

# Install Docker
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh

# Add your user to docker group (logout/login required)
sudo usermod -aG docker $USER
```

### Install NVIDIA Drivers

```bash
# Check GPU
lspci | grep -i nvidia

# Install NVIDIA drivers (Ubuntu)
sudo apt-get update
sudo apt-get install -y nvidia-driver-535  # or latest available

# Reboot required
sudo reboot
```

### Install nvidia-container-toolkit

```bash
# Add NVIDIA repository
distribution=$(. /etc/os-release;echo $ID$VERSION_ID)
curl -fsSL https://nvidia.github.io/libnvidia-container/gpgkey | sudo gpg --dearmor -o /usr/share/keyrings/nvidia-container-toolkit-keyring.gpg
curl -s -L https://nvidia.github.io/libnvidia-container/$distribution/libnvidia-container.list | \
  sed 's#deb https://#deb [signed-by=/usr/share/keyrings/nvidia-container-toolkit-keyring.gpg] https://#g' | \
  sudo tee /etc/apt/sources.list.d/nvidia-container-toolkit.list

# Install toolkit
sudo apt-get update
sudo apt-get install -y nvidia-container-toolkit

# Configure Docker
sudo nvidia-ctk runtime configure --runtime=docker
sudo systemctl restart docker

# Verify
docker run --rm --gpus all nvidia/cuda:12.6.0-base-ubuntu24.04 nvidia-smi
```

---

## Step 3: Pull and Run Container

### Pull Image

```bash
# Login to ghcr.io (if private)
echo $GITHUB_TOKEN | docker login ghcr.io -u trevorbenson --password-stdin

# Pull image
docker pull ghcr.io/trevorbenson/ce-ashmaize:cuda12-ubuntu24
```

### Verify GPU Access

```bash
# Test GPU detection
docker run --rm --gpus all \
  ghcr.io/trevorbenson/ce-ashmaize:cuda12-ubuntu24 \
  nvidia-smi
```

### Run Interactive Container

```bash
# Start container with bash
docker run -it --rm --gpus all \
  --name ashmaize-gpu \
  ghcr.io/trevorbenson/ce-ashmaize:cuda12-ubuntu24 \
  /bin/bash
```

Inside container:
```bash
# Verify CUDA
nvidia-smi

# Check solver
/app/ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver --help

# Test hash (CPU mode)
/app/ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver \
  test-hash \
  --address addr_test1vr6w0ja6rjz8gc3wvn6hh0v0chcsa2tg2ymzmz9e9x9 \
  --challenge-id 0000000000000000000000000000000000000000000000000000000000000001 \
  --difficulty 12 \
  --no-pre-mine 0 \
  --latest-submission 0 \
  --no-pre-mine-hour 0 \
  --nonce 12345

# Test hash (GPU mode)
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

---

## Step 4: SSH Testing Workflow

### Connect to Cloud Server

```bash
ssh user@your-cloud-server
```

### Run Commands via SSH

#### Quick Test
```bash
docker run --rm --gpus all \
  ghcr.io/trevorbenson/ce-ashmaize:cuda12-ubuntu24 \
  /app/ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver \
  --version
```

#### Benchmark Test
```bash
docker run --rm --gpus all \
  ghcr.io/trevorbenson/ce-ashmaize:cuda12-ubuntu24 \
  /app/ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver \
  benchmark \
  --address <ADDRESS> \
  --challenge-id <CHALLENGE_ID> \
  --difficulty <DIFFICULTY> \
  --no-pre-mine 0 \
  --latest-submission 0 \
  --no-pre-mine-hour 0 \
  --duration-secs 10 \
  --compare
```

#### Interactive Session
```bash
# Start detached container
docker run -d --name ashmaize-test --gpus all \
  ghcr.io/trevorbenson/ce-ashmaize:cuda12-ubuntu24 \
  sleep infinity

# Execute commands
docker exec -it ashmaize-test /bin/bash

# Inside container, run tests
cd /app/ce-ashmaize/cli_hunt/rust_solver
/app/ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver benchmark --help

# Cleanup
docker stop ashmaize-test
docker rm ashmaize-test
```

---

## Step 5: Production Deployment

### Docker Compose (Optional)

Create `docker-compose.yml`:
```yaml
version: '3.8'

services:
  ashmaize-solver:
    image: ghcr.io/trevorbenson/ce-ashmaize:cuda12-ubuntu24
    deploy:
      resources:
        reservations:
          devices:
            - driver: nvidia
              count: all
              capabilities: [gpu]
    command: >
      /app/ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver
      --address ${ADDRESS}
      --challenge-id ${CHALLENGE_ID}
      --difficulty ${DIFFICULTY}
      --no-pre-mine ${NO_PRE_MINE}
      --latest-submission ${LATEST_SUBMISSION}
      --no-pre-mine-hour ${NO_PRE_MINE_HOUR}
      --use-gpu
      --gpu-batch-size ${GPU_BATCH_SIZE:-4096}
    environment:
      - ADDRESS=${ADDRESS}
      - CHALLENGE_ID=${CHALLENGE_ID}
      - DIFFICULTY=${DIFFICULTY}
    restart: unless-stopped
```

Run:
```bash
docker compose up -d
```

### Systemd Service (Optional)

Create `/etc/systemd/system/ashmaize-solver.service`:
```ini
[Unit]
Description=Ashmaize GPU Solver
After=docker.service
Requires=docker.service

[Service]
Type=oneshot
RemainAfterExit=yes
ExecStart=/usr/bin/docker run --rm --gpus all \
  --name ashmaize-solver \
  ghcr.io/trevorbenson/ce-ashmaize:cuda12-ubuntu24 \
  /app/ce-ashmaize/cli_hunt/rust_solver/target/release/ashmaize-solver \
  --address <ADDRESS> \
  --challenge-id <CHALLENGE_ID> \
  --difficulty <DIFFICULTY> \
  --no-pre-mine 0 \
  --latest-submission 0 \
  --no-pre-mine-hour 0 \
  --use-gpu \
  --gpu-batch-size 4096

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl enable ashmaize-solver
sudo systemctl start ashmaize-solver
sudo systemctl status ashmaize-solver
```

---

## Troubleshooting

### Issue: "docker: Error response from daemon: could not select device driver"
**Solution:** Ensure nvidia-container-toolkit is installed and Docker is restarted:
```bash
sudo nvidia-ctk runtime configure --runtime=docker
sudo systemctl restart docker
```

### Issue: "nvidia-smi: command not found" in container
**Solution:** Base image doesn't include nvidia-smi. Install in container or use host nvidia-smi:
```bash
# On host
nvidia-smi
```

### Issue: "Unable to dynamically load the cuda shared library"
**Solution:** Verify CUDA libraries are accessible:
```bash
docker run --rm --gpus all \
  ghcr.io/trevorbenson/ce-ashmaize:cuda12-ubuntu24 \
  ls -la /usr/local/cuda/lib64/libcuda.so*
```

### Issue: Low GPU utilization
**Solution:** 
- Increase `--gpu-batch-size` (try 8192, 16384)
- Monitor with `nvidia-smi dmon` on host
- Check container logs for errors

### Issue: Permission denied pulling from ghcr.io
**Solution:** 
- Ensure GitHub token has `read:packages` permission
- Login: `docker login ghcr.io -u trevorbenson -p <TOKEN>`
- If repository is private, ensure token has access

---

## Monitoring

### GPU Monitoring (on host)
```bash
# Real-time monitoring
watch -n 1 nvidia-smi

# Detailed monitoring
nvidia-smi dmon -s u  # utilization
nvidia-smi dmon -s m  # memory
```

### Container Logs
```bash
# View logs
docker logs ashmaize-solver

# Follow logs
docker logs -f ashmaize-solver
```

---

## Cost Considerations

### Cloud GPU Providers
- **AWS EC2**: g4dn, g5, p3, p4 instances
- **Google Cloud**: T4, V100, A100 instances
- **Azure**: NC, ND, NV series
- **Paperspace**: Gradient instances
- **Lambda Labs**: GPU cloud instances

### Optimization Tips
- Use spot/preemptible instances for cost savings
- Right-size batch sizes for your GPU
- Monitor GPU utilization to avoid over-provisioning
- Consider reserved instances for long-term use

---

## Security Notes

1. **GitHub Token**: Store securely, use environment variables or secrets management
2. **Container Registry**: Consider making repository private if sensitive
3. **SSH Access**: Use key-based authentication, disable password auth
4. **Network**: Consider VPN or private networking for cloud deployments
5. **Secrets**: Don't hardcode addresses/challenge IDs in containers

---

## Next Steps

1. ✅ Push container to ghcr.io
2. ✅ Set up cloud server with GPU
3. ✅ Pull and test container
4. ⏳ Run benchmarks and collect performance data
5. ⏳ Optimize batch sizes for your GPU
6. ⏳ Validate hash correctness
7. ⏳ Deploy production solver

---

**Container Image:** `ghcr.io/trevorbenson/ce-ashmaize:cuda12-ubuntu24`  
**Ready for deployment!** 🚀

