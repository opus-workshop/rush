# Rush Release Testing Guide

This guide provides step-by-step instructions for testing Rush installations on clean machines before release publication.

## Pre-Release Checklist

Before releasing a new version, complete these tests:

- [ ] GitHub Actions release workflow runs successfully
- [ ] All binaries compile for all target platforms
- [ ] All checksums are generated correctly
- [ ] Checksums aggregate into single file
- [ ] Automated CI tests pass on all platforms
- [ ] Manual testing on clean macOS (Intel) machine
- [ ] Manual testing on clean macOS (ARM) machine
- [ ] Manual testing on clean Linux machine
- [ ] Homebrew formula works with new release
- [ ] README installation instructions verified
- [ ] Documentation is up-to-date

## Platform-Specific Tests

### macOS Intel (x86_64) Testing

#### 1. Binary Download and Verification

```bash
# Create test directory
mkdir -p ~/rush-test-intel
cd ~/rush-test-intel

# Download the binary
curl -LO https://github.com/opus-workshop/rush/releases/latest/download/rush-macos-x86_64.tar.gz

# Download checksums
curl -LO https://github.com/opus-workshop/rush/releases/latest/download/rush-macos-x86_64-SHA256SUMS.txt

# Verify checksum
shasum -a 256 -c rush-macos-x86_64-SHA256SUMS.txt
```

Expected output:
```
rush-macos-x86_64.tar.gz: OK
```

#### 2. Extract and Install

```bash
# Extract
tar xzf rush-macos-x86_64.tar.gz
chmod +x rush

# Test basic execution
./rush -c 'echo "Rush on macOS Intel"'
```

Expected output:
```
Rush on macOS Intel
```

#### 3. Install to System PATH

```bash
# Install to /usr/local/bin (may need sudo)
sudo cp rush /usr/local/bin/rush

# Test from PATH
rush -c 'pwd'
```

#### 4. Functionality Tests

```bash
# Variables
rush -c 'export TEST=hello && echo $TEST'
# Expected: hello

# Arithmetic
rush -c 'x=5; echo $((x + 3))'
# Expected: 8

# Conditionals
rush -c 'if [ 1 -eq 1 ]; then echo "true"; fi'
# Expected: true

# Loops
rush -c 'for i in 1 2 3; do echo $i; done'
# Expected: 1\n2\n3

# Pipes
rush -c 'echo -e "apple\nbanana" | grep apple'
# Expected: apple
```

#### 5. Daemon Mode Test

```bash
# Start daemon
rushd start

# Test command execution with daemon
rush -c 'echo "Using daemon"'

# Stop daemon
rushd stop
```

#### 6. Setting as Default Shell (Optional)

```bash
# Add to allowed shells
echo "/usr/local/bin/rush" | sudo tee -a /etc/shells

# Change to rush (temporary test only!)
# Don't do this on production machines
```

### macOS ARM (Apple Silicon) Testing

Follow the same steps as Intel, but with:

```bash
# Step 1: Download ARM binary
curl -LO https://github.com/opus-workshop/rush/releases/latest/download/rush-macos-aarch64.tar.gz
curl -LO https://github.com/opus-workshop/rush/releases/latest/download/rush-macos-aarch64-SHA256SUMS.txt
```

Verify that the binary is ARM-compiled:
```bash
file rush
# Expected: Mach-O 64-bit executable arm64
```

### Linux x86_64 Testing

#### 1. Binary Download and Verification

```bash
# Create test directory
mkdir -p ~/rush-test-linux
cd ~/rush-test-linux

# Download the binary
curl -LO https://github.com/opus-workshop/rush/releases/latest/download/rush-linux-x86_64.tar.gz

# Download checksums
curl -LO https://github.com/opus-workshop/rush/releases/latest/download/rush-linux-x86_64-SHA256SUMS.txt

# Verify checksum
sha256sum -c rush-linux-x86_64-SHA256SUMS.txt
```

Expected output:
```
rush-linux-x86_64.tar.gz: OK
```

#### 2. Extract and Install

```bash
# Extract
tar xzf rush-linux-x86_64.tar.gz
chmod +x rush

# Test basic execution
./rush -c 'echo "Rush on Linux"'
```

Expected output:
```
Rush on Linux
```

#### 3. Verify Architecture

```bash
# Check binary info
file rush
# Expected: ELF 64-bit LSB pie executable, x86-64, dynamically linked

# Check dependencies
ldd rush
# Should show normal glibc dependencies (or none for musl)
```

#### 4. Install to System PATH

```bash
# Install
sudo mv rush /usr/local/bin/rush

# Verify
which rush
# Expected: /usr/local/bin/rush

rush -c 'echo "System-wide installation works"'
```

#### 5. Functionality Tests

```bash
# Variables
rush -c 'export TEST=linux && echo $TEST'
# Expected: linux

# String manipulation
rush -c 'str="hello world"; echo ${str#hello}'
# Expected: ' world'

# Command substitution
rush -c 'echo "Home: $(cd ~ && pwd)"'
# Expected: Home: /home/username

# Background jobs
rush -c 'sleep 1 & echo "Background job started"'
```

#### 6. Daemon Mode Test

```bash
# Start daemon
rushd start

# Test multiple quick commands
time rush -c 'ls -la /' > /dev/null
# Should be very fast (~0.4ms with daemon)

# Stop
rushd stop
```

### Static Linux (musl) Testing

The musl variant should work on any Linux system without dependencies:

```bash
# Download musl variant
curl -LO https://github.com/opus-workshop/rush/releases/latest/download/rush-linux-x86_64-musl.tar.gz
tar xzf rush-linux-x86_64-musl.tar.gz

# Verify it's static
ldd ./rush
# Expected: "not a dynamic executable" or "statically linked"

# Test on different systems
./rush -c 'echo "Works without libc"'
```

## Homebrew Testing

### Install via Homebrew (macOS only)

```bash
# Add tap
brew tap opus-workshop/rush https://github.com/opus-workshop/rush

# Install
brew install rush

# Verify installation
which rush
# Expected: /usr/local/bin/rush (or similar brew path)

# Test
rush -c 'echo "Installed via Homebrew"'
```

### Update Homebrew Formula

After each release, verify the Homebrew formula works:

```bash
# The formula automatically downloads the latest release
# To test with a specific version, temporarily edit:
# /usr/local/Cellar/rush/*/Homebrew/Formula/rush.rb

# Test uninstall/reinstall
brew uninstall rush
brew install rush
```

## Checksum Aggregation Testing

Verify the combined checksum file works:

```bash
# Download combined checksums
curl -LO https://github.com/opus-workshop/rush/releases/latest/download/SHA256SUMS.txt

# Verify all files
sha256sum -c SHA256SUMS.txt --ignore-missing
```

Expected output:
```
rush-linux-x86_64.tar.gz: OK
rush-linux-x86_64-musl.tar.gz: OK
rush-macos-x86_64.tar.gz: OK
rush-macos-aarch64.tar.gz: OK
```

## Signature Testing

Current releases don't include GPG signatures, but future releases might. When available:

```bash
# Download public key
curl https://github.com/opus-workshop/rush.gpg | gpg --import

# Verify signature
gpg --verify rush-*.tar.gz.sig rush-*.tar.gz
```

## Regression Testing

For each release, verify these features still work:

```bash
# POSIX compatibility
rush -c '
  # Comments work
  x=10
  [ $x -gt 5 ] && echo "Arithmetic comparison"

  # Functions
  greet() { echo "Hello $1"; }
  greet "Rush"

  # Case statement
  case $x in
    10) echo "Found 10" ;;
    *) echo "Other" ;;
  esac
'
```

```bash
# Built-in commands
rush -c '
  # File operations
  echo "test" | cat

  # Directory operations
  mkdir -p /tmp/test
  cd /tmp/test
  pwd

  # List directory
  touch file.txt
  ls -la

  # Cleanup
  cd /
  rm -rf /tmp/test
'
```

```bash
# Performance baseline
time rush -c 'echo "startup test"'
# Should be <10ms for cold start
```

## Error Handling Testing

Test that errors are handled gracefully:

```bash
# Non-existent command
rush -c 'nonexistent_command 2>&1'
# Should show appropriate error

# Syntax error
rush -c 'if [ 1; then echo test' 2>&1
# Should show parse error

# Exit code
rush -c 'exit 42'
echo $?
# Should output 42
```

## Documentation Verification

- [ ] README.md installation section is accurate
- [ ] docs/INSTALLATION.md is up-to-date
- [ ] Release notes are clear and helpful
- [ ] Examples in documentation still work
- [ ] Links to resources are correct

## Performance Baselines

Record these metrics for each release:

```bash
# Startup time (cold)
time rush -c 'echo startup' > /dev/null

# Startup time (daemon)
rushd start
time rush -c 'echo daemon' > /dev/null
rushd stop

# Memory usage
rush -c 'ps aux | grep rush'

# Binary size
ls -lh /usr/local/bin/rush
```

## Test Summary Template

Use this template to document test results:

```
Release: v0.x.x
Date: YYYY-MM-DD
Tester: Name
System: macOS/Linux version

macOS Intel (x86_64):
  - [ ] Binary downloads: PASS/FAIL
  - [ ] Checksum verification: PASS/FAIL
  - [ ] Installation: PASS/FAIL
  - [ ] Basic tests: PASS/FAIL
  - [ ] Functionality tests: PASS/FAIL
  - [ ] Daemon mode: PASS/FAIL

macOS ARM (aarch64):
  - [ ] Binary downloads: PASS/FAIL
  - [ ] Checksum verification: PASS/FAIL
  - [ ] Installation: PASS/FAIL
  - [ ] Basic tests: PASS/FAIL
  - [ ] Functionality tests: PASS/FAIL

Linux x86_64:
  - [ ] Binary downloads: PASS/FAIL
  - [ ] Checksum verification: PASS/FAIL
  - [ ] Installation: PASS/FAIL
  - [ ] Basic tests: PASS/FAIL
  - [ ] Functionality tests: PASS/FAIL

Homebrew:
  - [ ] Install via brew: PASS/FAIL
  - [ ] Functionality: PASS/FAIL

Issues found:
- None

Overall: READY FOR RELEASE
```

## Continuous Testing

The GitHub Actions workflow automatically tests releases, but manual testing on fresh machines provides additional confidence:

1. **Automated tests** (GitHub Actions):
   - Run automatically for every release
   - Test on clean Ubuntu, macOS Intel, macOS ARM
   - Verify checksums and basic functionality

2. **Manual tests** (this guide):
   - Performed on actual machines by human testers
   - Verify real-world installation experience
   - Catch issues that CI might miss
   - Test user documentation accuracy

## Troubleshooting

### Binary won't execute on macOS

```bash
# Check quarantine attribute
xattr -l rush

# Remove if quarantined
xattr -d com.apple.quarantine rush

# Try running again
./rush -c 'echo test'
```

### Checksum mismatch

```bash
# Re-download files (might be incomplete)
rm *.tar.gz *.txt
curl -LO https://github.com/opus-workshop/rush/releases/latest/download/...

# Try different hash tool
md5 rush-*.tar.gz
```

### Daemon port in use

```bash
# Find process using daemon port
lsof -i :9090  # Default daemon port

# Kill existing daemon
pkill -f rushd
```

## Sign-Off

After completing all tests, document results:

```bash
# Create test report
cat > RELEASE_TEST_RESULTS.md << 'EOF'
# Test Results for v0.x.x

Date: YYYY-MM-DD
Tester: Your Name

## Summary
All tests passed successfully.

## Platform Results
- macOS Intel: PASS
- macOS ARM: PASS
- Linux: PASS
- Homebrew: PASS

## Ready for Release: YES
EOF
```

## Next Steps

After successful testing:
1. Merge any final fixes to main branch
2. Create release tag: `git tag v0.x.x && git push origin v0.x.x`
3. Wait for GitHub Actions to build and publish
4. Verify release is available on GitHub
5. Test installation from release (user perspective)
6. Close related issues
7. Announce release
