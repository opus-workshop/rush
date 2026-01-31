# Installing Rush

Rush is a high-performance, POSIX-compliant shell written in Rust. This document covers all installation methods.

## Quick Start

### macOS (Homebrew) - Recommended

The easiest way to install Rush on macOS:

```bash
brew tap opus-workshop/rush https://github.com/opus-workshop/rush
brew install rush
```

### Linux and macOS (Binary Download)

Pre-built binaries are available for:
- **macOS Intel** (x86_64)
- **macOS ARM** (Apple Silicon / aarch64)
- **Linux** (x86_64 glibc)
- **Linux** (x86_64 musl - static, portable)

#### Download and Install

```bash
# Determine your platform
PLATFORM=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

# macOS ARM (Apple Silicon)
curl -LO https://github.com/opus-workshop/rush/releases/latest/download/rush-macos-aarch64.tar.gz
tar xzf rush-macos-aarch64.tar.gz
sudo mv rush /usr/local/bin/

# macOS Intel
curl -LO https://github.com/opus-workshop/rush/releases/latest/download/rush-macos-x86_64.tar.gz
tar xzf rush-macos-x86_64.tar.gz
sudo mv rush /usr/local/bin/

# Linux x86_64
curl -LO https://github.com/opus-workshop/rush/releases/latest/download/rush-linux-x86_64.tar.gz
tar xzf rush-linux-x86_64.tar.gz
sudo mv rush /usr/local/bin/

# Linux x86_64 (static binary - more portable)
curl -LO https://github.com/opus-workshop/rush/releases/latest/download/rush-linux-x86_64-musl.tar.gz
tar xzf rush-linux-x86_64-musl.tar.gz
sudo mv rush /usr/local/bin/
```

#### Verify the Download

Each release includes SHA256 checksums for verification:

```bash
# Download the checksum file
curl -LO https://github.com/opus-workshop/rush/releases/latest/download/SHA256SUMS.txt

# Verify on Linux
sha256sum -c SHA256SUMS.txt --ignore-missing

# Verify on macOS
shasum -a 256 -c SHA256SUMS.txt --ignore-missing
```

### Cargo Install

If you have Rust installed:

```bash
cargo install --git https://github.com/opus-workshop/rush
```

To install a specific version:

```bash
cargo install --git https://github.com/opus-workshop/rush --tag v0.1.0
```

### Build from Source

Clone the repository and build:

```bash
git clone https://github.com/opus-workshop/rush.git
cd rush
cargo build --release
sudo cp target/release/rush /usr/local/bin/
```

**Requirements:**
- Rust 1.70 or later
- Cargo

## Setting as Default Shell

After installing, you can make Rush your default shell:

### macOS (Homebrew)

```bash
# Add Rush to allowed shells
echo "$(brew --prefix)/bin/rush" | sudo tee -a /etc/shells

# Change your shell
chsh -s "$(brew --prefix)/bin/rush"
```

### Linux / macOS (Binary Install)

```bash
# Add Rush to allowed shells
echo "/usr/local/bin/rush" | sudo tee -a /etc/shells

# Change your shell
chsh -s /usr/local/bin/rush
```

## Daemon Mode (Optional)

For ultra-fast startup times, use Rush's daemon mode:

```bash
# Start the daemon
rushd start

# Commands now connect to the daemon (much faster)
rush -c "ls"      # ~0.4ms instead of ~4.9ms

# Stop the daemon
rushd stop
```

This is ideal for:
- CI/CD pipelines with many shell invocations
- Build systems (Make, scripts)
- Test suites that fork many processes
- AI agents making rapid shell calls

## Updating

### Homebrew

```bash
brew update
brew upgrade rush
```

### Binary Downloads

Download and install the latest release using the instructions above.

### Cargo

```bash
cargo install --git https://github.com/opus-workshop/rush --force
```

## Uninstalling

### Homebrew

```bash
brew uninstall rush
brew untap opus-workshop/rush
```

### Binary Install

```bash
sudo rm /usr/local/bin/rush
```

### From Source

```bash
sudo rm /usr/local/bin/rush
```

## Troubleshooting

### Binary not found after installation

Make sure `/usr/local/bin` is in your PATH:

```bash
echo $PATH
```

If not, add it:

```bash
export PATH="/usr/local/bin:$PATH"
```

### Permission denied when running rush

Make sure the binary is executable:

```bash
chmod +x /usr/local/bin/rush
```

### macOS: "rush cannot be opened because it is from an unidentified developer"

This is a macOS security feature. You can bypass it with:

```bash
xattr -d com.apple.quarantine /usr/local/bin/rush
```

Or via System Preferences:
1. Go to System Preferences > Security & Privacy
2. Click "Open Anyway" next to Rush

### Linux: "cannot execute binary file: Exec format error"

This usually means the binary doesn't match your architecture. Make sure you downloaded the correct version:

```bash
# Check your architecture
uname -m    # Should be x86_64
uname -s    # Should be Linux

# Download the appropriate binary
# For x86_64: rush-linux-x86_64.tar.gz
# For ARM: rush-linux-aarch64.tar.gz (if available)
```

## Performance

Rush binary sizes:

| Platform | Size (uncompressed) | Size (compressed) |
|----------|-------------------|-------------------|
| macOS ARM | ~4.7MB | ~1.2MB |
| macOS Intel | ~4.7MB | ~1.2MB |
| Linux x86_64 | ~5.1MB | ~1.3MB |
| Linux x86_64 musl | ~5.2MB | ~1.3MB |

Startup times:

| Mode | Time |
|------|------|
| Cold start | ~4.9ms |
| Daemon mode | ~0.4ms |

## Platform Support

| Platform | Architecture | Status |
|----------|-------------|--------|
| macOS | ARM (Apple Silicon) | Fully supported |
| macOS | Intel (x86_64) | Fully supported |
| Linux | x86_64 (glibc) | Fully supported |
| Linux | x86_64 (musl) | Fully supported |
| Linux | ARM/aarch64 | Available (community builds) |
| Windows | WSL2 | Tested and working |
| BSD | FreeBSD | Community support |

## Security

All binaries are:
- Built from source in GitHub Actions
- Signed checksums provided for verification
- Hosted on official GitHub releases
- No telemetry or tracking

## Getting Help

- **Documentation**: https://github.com/opus-workshop/rush
- **Issues**: https://github.com/opus-workshop/rush/issues
- **Discussions**: https://github.com/opus-workshop/rush/discussions

## License

Rush is dual-licensed under MIT or Apache-2.0 (your choice).
