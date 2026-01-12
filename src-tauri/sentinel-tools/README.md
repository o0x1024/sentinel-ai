# Sentinel Tools - Docker Sandbox

## Overview

Sentinel Tools now supports secure shell command execution in isolated Docker containers **based on Kali Linux**. This provides better security isolation compared to executing commands directly on the host machine, with pre-installed security testing tools.

**🔥 New**: Now using **Kali Linux** as the base image for comprehensive security testing capabilities!

## Architecture

### Execution Modes

1. **Docker Mode (Default)**: Commands run in isolated Docker containers
   - Resource limits (CPU, memory)
   - Network isolation options
   - Pre-installed security tools
   - Container pooling for performance

2. **Host Mode**: Commands run directly on host machine
   - Less secure but faster
   - Full access to host filesystem
   - Requires explicit configuration

### Container Pool

- Reuses containers to avoid creation overhead
- Automatic cleanup of stale containers
- Configurable pool size and reuse limits
- 5-minute idle timeout by default

## Setup

### 1. Install Docker

Make sure Docker is installed and running on your system:

```bash
# Check Docker installation
docker --version

# Test Docker
docker run hello-world
```

### 2. Build Sandbox Image

Build the Kali Linux security sandbox image:

```bash
# Linux/macOS
# Minimal version (recommended, ~370MB, 2-3 min)
./scripts/build-docker-sandbox.sh minimal

# Standard Kali with top tools (~1.5GB, 5-10 min)
./scripts/build-docker-sandbox.sh kali

# Full Kali with all tools (~3-4GB, 15-30 min)
./scripts/build-docker-sandbox.sh kali-full

# Windows PowerShell
.\scripts\build-docker-sandbox.ps1
```

Or build manually:

```bash
cd src-tauri/sentinel-tools

# Minimal Kali (recommended)
docker build -t sentinel-sandbox:latest -f Dockerfile.sandbox.minimal .

# Standard Kali
docker build -t sentinel-sandbox:latest -f Dockerfile.sandbox .

# Full Kali
docker build -t sentinel-sandbox:latest -f Dockerfile.sandbox.kali-full .
```

**Note**: 
- First build may take longer due to downloading Kali base image
- See [Kali Docker Guide](./KALI_DOCKER_README.md) for detailed tool list
- If build fails, see [Troubleshooting Guide](./DOCKER_TROUBLESHOOTING.md)

### 3. Verify Image

```bash
docker images | grep sentinel-sandbox
```

## Usage

### Configuration

The shell tool can be configured through the Tauri command API:

```typescript
// Get current configuration
const config = await invoke('get_shell_configuration');

// Update configuration
await invoke('update_shell_configuration', {
  config: {
    default_execution_mode: 'Docker', // or 'Host'
    default_policy: 'RequestReview',
    allowed_commands: ['ls', 'cat', 'grep'],
    denied_commands: ['rm -rf', 'dd', 'mkfs'],
    docker_config: {
      image: 'sentinel-sandbox:latest',
      memory_limit: '512m',
      cpu_limit: '1.0',
      network_mode: 'bridge',
      read_only_rootfs: false,
      volumes: {},
      env_vars: {}
    }
  }
});
```

### Initialize Docker Sandbox

```typescript
// Initialize (builds image if needed)
await invoke('initialize_docker_sandbox');

// Check Docker availability
const available = await invoke('check_docker_availability');

// Build/rebuild image
await invoke('build_docker_sandbox_image');

// Cleanup all containers
await invoke('cleanup_docker_containers');
```

### Execute Commands

Commands are executed through the unified tool interface:

```typescript
// Execute in Docker (default)
const result = await invoke('unified_execute_tool', {
  toolName: 'shell',
  args: {
    command: 'nmap -sV localhost',
    timeout_secs: 60
  }
});

// Force host execution
const result = await invoke('unified_execute_tool', {
  toolName: 'shell',
  args: {
    command: 'ls -la',
    execution_mode: 'Host',
    timeout_secs: 30
  }
});
```

## Pre-installed Tools (Kali Linux)

The Docker sandbox is based on **Kali Linux** and includes:

### Minimal Version (~370MB)
- **Network**: nmap, netcat, dnsutils, ping
- **Basic**: curl, wget, git, python3, jq
- **Text**: grep, sed, awk

### Standard Version (~1.5GB) - Kali Top 10
- **Network Scanning**: nmap, masscan, netcat
- **Web Testing**: burpsuite, nikto, dirb, gobuster, ffuf, wfuzz
- **Exploitation**: metasploit-framework, sqlmap
- **Password Attacks**: hydra, john, hashcat
- **Sniffing**: wireshark, tcpdump
- **Info Gathering**: theharvester, recon-ng

### Full Version (~3-4GB) - 600+ Tools
Includes all Standard tools plus:
- **Web Apps**: wpscan, joomscan, commix, xsser
- **Wireless**: aircrack-ng, reaver, wifite
- **Social Engineering**: SET
- **Vulnerability Analysis**: openvas, skipfish
- **Reverse Engineering**: radare2, ghidra, binwalk
- **Forensics**: autopsy, volatility

### Development Tools (All Versions)
- Python 3 + pip (requests, beautifulsoup4, pwntools)
- Node.js + npm (retire, eslint, snyk)
- Go (for custom tools)

**See [Kali Docker Guide](./KALI_DOCKER_README.md) for complete tool list and usage examples.**

## Security Features

### Resource Limits

```rust
DockerSandboxConfig {
    memory_limit: "512m",  // 512MB RAM
    cpu_limit: "1.0",      // 1 CPU core
    network_mode: "bridge", // Isolated network
    read_only_rootfs: false,
    // ...
}
```

### Permission System

Commands are subject to the shell tool permission system:

1. **Denied List**: Commands that are always blocked
2. **Allowed List**: Commands that auto-execute
3. **Default Policy**: 
   - `RequestReview`: Ask user for confirmation
   - `AlwaysProceed`: Auto-execute (except denied)

### Network Isolation

Network modes:
- `none`: No network access
- `bridge`: Isolated bridge network (default)
- `host`: Host network (less secure)

## Performance

### Container Pooling

- Containers are reused up to 10 times
- Maximum 5 concurrent containers
- Idle containers cleaned up after 5 minutes

### Benchmarks

| Operation | Docker Mode | Host Mode |
|-----------|-------------|-----------|
| First execution | ~2-3s | ~100ms |
| Subsequent (pooled) | ~200-300ms | ~100ms |
| Cleanup | Automatic | N/A |

## Troubleshooting

For detailed troubleshooting guide, see [DOCKER_TROUBLESHOOTING.md](./DOCKER_TROUBLESHOOTING.md)

### Quick Fixes

**Build fails (exit code 100)**:
```bash
# Use minimal version
./scripts/build-docker-sandbox.sh minimal
```

**Docker Not Available**:
```bash
# Check Docker daemon
docker ps

# Start Docker service (Linux)
sudo systemctl start docker

# macOS: Start Docker Desktop
```

**Permission Denied**:
```bash
# Linux: Add user to docker group
sudo usermod -aG docker $USER
newgrp docker
```

**Container Cleanup**:
```bash
# Clean all containers
docker ps -a | grep sentinel-sandbox | awk '{print $1}' | xargs docker rm -f

# Or use the command
await invoke('cleanup_docker_containers');
```

## Development

### Custom Tools

To add custom tools to the sandbox, modify `Dockerfile.sandbox`:

```dockerfile
# Add custom tool
RUN apt-get update && apt-get install -y your-tool

# Or install via pip
RUN pip3 install your-python-package
```

Then rebuild:

```bash
./scripts/build-docker-sandbox.sh
```

### Testing

```rust
#[tokio::test]
async fn test_docker_execution() {
    let config = DockerSandboxConfig::default();
    let sandbox = DockerSandbox::new(config);
    
    let (stdout, stderr, code) = sandbox
        .execute("echo 'Hello from Docker'", 10)
        .await
        .unwrap();
    
    assert_eq!(code, 0);
    assert!(stdout.contains("Hello from Docker"));
}
```

## Best Practices

1. **Use Docker Mode by Default**: Better security isolation
2. **Set Resource Limits**: Prevent resource exhaustion
3. **Enable Permission Review**: For sensitive commands
4. **Regular Image Updates**: Keep tools up-to-date
5. **Monitor Container Usage**: Check for leaks

## Future Enhancements

- [ ] Support for custom Docker images per tool
- [ ] Volume mounting for file access
- [ ] Network proxy configuration
- [ ] Container resource monitoring
- [ ] Multi-architecture support (ARM64)
- [ ] Kubernetes pod execution support
