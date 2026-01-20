# Rush Testing Guide

## Status: Phase 3 Complete ✓

All Phase 3 features have been implemented and tested:
- ✅ Fast builtins (ls, cat, find, grep)
- ✅ Command history with fuzzy search
- ✅ Tab completion system
- ✅ User-defined functions
- ✅ Undo file operations

**Test results:** 109/109 tests passing
**Build status:** Successful (3.2MB optimized binary)

## Quick Start (3 Steps)

### 1. Build Rush
```bash
cd /Users/asher/knowledge/rush
cargo build --release
```

### 2. Run Rush
```bash
./target/release/rush
```

You should see:
```
Rush v0.1.0 - A Modern Shell in Rust
Type 'exit' to quit

rush>
```

### 3. Try Commands
```bash
# Basic commands
pwd
ls
cat Cargo.toml

# Fast builtins
cat benchmarks/benchmark-data/large-file.txt
ls benchmarks/benchmark-data/deep-tree/dir1
find benchmarks/benchmark-data/deep-tree -name "*.txt"
grep "FOUND" benchmarks/benchmark-data/grep-test.txt

# History
history
history search cargo

# Tab completion (press TAB)
cat Car<TAB>
ls bench<TAB>

# Functions
function greet(name) {
    echo "Hello, $name!"
}
greet "Rush"

# Undo
touch test.txt
rm test.txt
undo list
undo
ls test.txt

# Exit
exit
```

## Performance Benchmarks

### Automated Zsh Benchmark
```bash
cd benchmarks
./compare.sh
```

**Zsh baseline results (M2 Macbook Air):**
```
Total time: 539ms (100 iterations each)
- Command overhead: 8ms
- cat large file: 46ms
- ls 1000 files: 65ms
- ls -la: 123ms
- find deep tree: 43ms
- grep large file: 57ms
- pwd builtin: 6ms
- cd navigation: 8ms
- Variables: 7ms
- Pipelines: 124ms
```

### Manual Rush Testing

Since Rush doesn't support shell scripts yet (Phase 4), test performance manually:

1. **Run Rush:** `./target/release/rush`

2. **Test these commands and compare feel to zsh:**
   - `cat benchmarks/benchmark-data/large-file.txt` (Should feel instant)
   - `ls benchmarks/benchmark-data/deep-tree/dir1` (Fast directory listing)
   - `find benchmarks/benchmark-data/deep-tree -name "*.txt"` (Parallel find)
   - `grep "FOUND" benchmarks/benchmark-data/grep-test.txt` (ripgrep speed)

3. **Expected improvements:**
   - cat: 2-5x faster (memory-mapped I/O)
   - ls: 1.5-3x faster (parallel reads)
   - find: 2-4x faster (WalkBuilder)
   - grep: 3-10x faster (ripgrep integration)

See `benchmarks/manual-rush-test.md` for detailed manual testing guide.

## Feature Testing

### 1. Command History
```bash
# In Rush:
echo "test 1"
echo "test 2"
cargo build
git status

# View history
history

# Fuzzy search
history search cargo
history search git
```

### 2. Tab Completion
```bash
# File completion
cat Ca<TAB>           # → Cargo.toml
ls bench<TAB>         # → benchmarks/

# Command completion
cargo bu<TAB>         # → cargo build
git st<TAB>           # → git status

# Flag completion
ls -<TAB>             # Shows -l, -a, -la, etc.
```

### 3. User Functions
```bash
# Define a function
function deploy(env) {
    echo "Deploying to $env..."
    ls -la
    echo "Done!"
}

# Call it
deploy "staging"
deploy "production"

# Nested functions
function outer(x) {
    function inner(y) {
        echo "x=$x, y=$y"
    }
    inner "inner value"
}
outer "outer value"
```

### 4. Undo Operations
```bash
# Check undo status
undo list

# Create and delete a file
touch important-file.txt
echo "important data" > important-file.txt
rm important-file.txt

# Oops! Undo it
undo
cat important-file.txt  # File restored!

# View undo history
undo list

# Disable/enable
undo disable
touch temp.txt
rm temp.txt
undo  # Won't work - undo was disabled

undo enable
```

### 5. Fast Builtins

#### cat (Memory-mapped I/O)
```bash
# Large file test
cat benchmarks/benchmark-data/large-file.txt
# Should feel instant even for 10K lines
```

#### ls (Parallel directory reading)
```bash
# Many files
ls benchmarks/benchmark-data/deep-tree/dir1
ls -la benchmarks/benchmark-data/deep-tree/dir1
# Faster than traditional ls with many files
```

#### find (WalkBuilder parallel traversal)
```bash
# Deep directory tree
find benchmarks/benchmark-data/deep-tree -name "*.txt"
# Parallel traversal, faster on deep trees
```

#### grep (ripgrep integration)
```bash
# Large file search
grep "FOUND" benchmarks/benchmark-data/grep-test.txt
# Much faster than traditional grep
```

## Verification Checklist

- [ ] Rush builds successfully: `cargo build --release`
- [ ] Binary exists: `ls -lah target/release/rush` (should be ~3.2MB)
- [ ] Tests pass: `cargo test` (109/109 passing)
- [ ] Rush launches: `./target/release/rush`
- [ ] Basic commands work: `pwd`, `ls`, `cat`
- [ ] Fast builtins work: `cat`, `ls`, `find`, `grep`
- [ ] History works: `history`, `history search`
- [ ] Tab completion works: Press TAB after partial commands
- [ ] Functions work: Define and call a function
- [ ] Undo works: Delete a file and restore it
- [ ] Can exit cleanly: `exit` or Ctrl+D

## Known Limitations (Phase 3)

1. **No script execution** - Can't run `.sh` files
   - Coming in Phase 4
   - Use interactive mode for now

2. **Limited piping** - Basic pipes work, complex ones don't
   - `echo "test" | cat` ✓
   - Complex multi-stage pipelines may fail

3. **No job control** - Can't background processes
   - No `&`, `fg`, `bg`, `jobs`
   - Coming in future phase

4. **Interactive mode required** - TTY needed for input
   - Can't pipe commands to Rush yet
   - Reedline requires interactive terminal

## Documentation

- **RUN_THIS_FIRST.md** - Quickest way to get started
- **QUICKSTART.md** - Complete user guide with all features
- **benchmarks/README.md** - Benchmark suite documentation
- **benchmarks/manual-rush-test.md** - Manual performance testing
- **docs/phase-3-completion-summary.md** - Complete feature documentation

## Troubleshooting

### Build fails
```bash
rustup update
cargo clean
cargo build --release
```

### "Device not configured" error
This is expected when piping input. Rush needs an interactive TTY.
Run directly: `./target/release/rush`

### Tests fail
```bash
# Run with single thread to avoid flakiness
cargo test -- --test-threads=1
```

### Rush won't start
```bash
# Check binary exists
ls -lah target/release/rush

# Rebuild
cargo clean
cargo build --release
```

## Next Steps

1. **Test Rush interactively** - Feel the performance improvements
2. **Compare with zsh** - Run same commands in both shells
3. **Try all features** - History, completion, functions, undo
4. **Review benchmark results** - See the numbers
5. **Provide feedback** - What works well? What needs improvement?

---

**Ready to test!** Start with: `./target/release/rush`
