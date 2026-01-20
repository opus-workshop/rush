# 🚀 Getting Started with Rush

## Quick 3-Step Setup

### 1. Build Rush (release mode for speed)
```bash
cargo build --release
```

### 2. Run Rush
```bash
./target/release/rush
```

You should see:
```
rush>
```

### 3. Try Some Commands!
```bash
# Basic commands
pwd
ls
cat Cargo.toml

# Check out the features
history
undo list

# Exit when done
exit
```

## 📊 Run the Benchmark (Optional)

To compare Rush vs zsh performance:

```bash
# Install timing tool (one time only)
brew install coreutils

# Run the benchmark
cd benchmarks
./compare.sh
```

This will:
- Run a comprehensive 15-30 second benchmark suite
- Test 10 different operations
- Show you the speedup metrics

Expected results:
- **2-10x faster** for file operations (cat, ls, find, grep)
- **Lower overhead** for command execution
- **Smart features** with minimal performance cost

## 📖 Full Guide

See `QUICKSTART.md` for:
- All features explained
- Tips and tricks
- Performance optimization
- Configuration options
- Troubleshooting

## ⚡ Why Rush is Fast

1. **Optimized Builtins**
   - `cat` uses memory-mapped I/O
   - `ls` uses parallel directory reading
   - `find` uses WalkBuilder for parallel traversal
   - `grep` integrates ripgrep

2. **Zero Overhead**
   - Builtins execute instantly (no process spawning)
   - Direct system calls
   - Efficient Rust memory management

3. **Smart Caching**
   - Tab completion caches (5-min TTL)
   - Project context detection cached
   - PATH lookup cached

## 🎯 Key Features to Try

### 1. Command History with Fuzzy Search
```bash
history search cargo
history search "git commit"
```

### 2. Tab Completion (press TAB)
```bash
cat Ca<TAB>         # → cat Cargo.toml
cargo bu<TAB>       # → cargo build
git sta<TAB>        # → git status
```

### 3. User Functions
```bash
function hello(name) {
    echo "Hello, $name!"
}
hello "Rush"
```

### 4. Undo File Operations
```bash
# Accidentally delete something
touch test.txt
rm test.txt

# Oops! Undo it
undo
# File restored!
```

## 🐛 Troubleshooting

**Build fails?**
```bash
rustup update
cargo clean
cargo build --release
```

**Can't find gdate?**
```bash
brew install coreutils
```

**Want to exit Rush?**
```bash
exit
# or Ctrl+D
```

## 🎉 You're Ready!

Start using Rush and feel the difference. Your commands will execute faster, and you'll have powerful features like history search, undo, and smart completion at your fingertips.

Happy rushing! 🏃‍♂️💨
