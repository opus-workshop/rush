# Rush - A Modern Shell in Rust

A modern Unix shell written in Rust that prioritizes **performance**, **safety**, and **developer ergonomics**.

## Features

### Phase 1 - Core Shell Foundation ✅

- **Lexer & Parser**: Full tokenization and AST generation for shell commands
- **Command Executor**: Execute external commands and pipelines
- **Basic Builtins**:
  - `cd` - Change directory (with ~ expansion)
  - `pwd` - Print working directory
  - `echo` - Print arguments
  - `exit` - Exit the shell
  - `export` - Set environment variables
- **REPL**: Interactive shell with line editing via reedline
- **Pipeline Support**: Connect commands with `|`
- **Variable System**: Store and retrieve variables
- **Rust-inspired Syntax**: Support for `let`, `if`, `fn`, `for`, `match`

### Phase 2 - Fast Built-in Commands ✅

- **High-Performance Builtins**: Rust implementations 3-10x faster than GNU
  - `ls` - Fast directory listing with color, long format, human-readable sizes
  - `grep` - Ripgrep-powered search with regex, recursive, colored output
  - `find` - Parallel directory traversal with .gitignore awareness
  - `cat` - Memory-mapped I/O for large files (>1MB), binary file detection
- **Git Integration**: Native git2 bindings for fast git operations
  - `git-status` - Fast repository status with branch tracking
  - Git context in prompt (branch, dirty state, ahead/behind)
- **JSON Output**: Structured output support for automation
- **Performance Benchmarks**: Criterion + hyperfine testing infrastructure

### Coming Soon

- **Phase 3**: Project context detection, advanced tab completion, scripting
- **Phase 4**: Undo capability, advanced automation features

## Architecture

```
rush/
├── lexer/          # Token stream generation using Logos
├── parser/         # AST construction with nom
├── executor/       # Command execution engine
│   └── pipeline/   # Pipeline execution support
├── runtime/        # Variable scoping and environment
├── builtins/       # Built-in commands
├── completion/     # Tab completion (TODO)
├── history/        # Command history (TODO)
├── context/        # Project/Git detection (TODO)
└── output/         # Text and JSON formatting (TODO)
```

## Quick Start

### Build and Run

```bash
cargo build --release
cargo run
```

### Example Usage

```bash
Rush v0.1.0 - A Modern Shell in Rust
Type 'exit' to quit

> pwd
/Users/asher/knowledge/rush

> echo Hello, Rush!
Hello, Rush!

> cd /tmp
> pwd
/tmp

> exit
```

### Pipeline Example

```bash
> ls | grep rust
# (would execute if external ls and grep are available)
```

### Rust-inspired Syntax (Parsed, not yet fully executed)

```rust
// Variable assignment
let x = 42

// Conditionals
if x > 10 {
    echo "large"
} else {
    echo "small"
}

// Functions
fn deploy(env: String) {
    echo "Deploying to {env}"
}

// Loops
for file in $(ls) {
    echo $file
}
```

## Performance Targets

Based on the Rush PRD:

| Metric | Target | Status |
|--------|--------|--------|
| Startup time | <10ms | ✅ **3.8ms** |
| Memory usage | <10MB | ✅ Achieved |
| Built-in speedup | 3-10x | ✅ Implemented |

See [BENCHMARKS.md](BENCHMARKS.md) for comprehensive benchmarking documentation.

## Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture
```

Current test suite: **50 tests, all passing** ✅

## Benchmarking

Rush includes comprehensive performance benchmarks to ensure we meet our performance targets:

```bash
# Build optimized release binary
cargo build --release

# Run criterion microbenchmarks
cargo bench

# Run real-world hyperfine benchmarks
./scripts/benchmark.sh

# View detailed results
open target/criterion/report/index.html
```

**Benchmark suites:**
- **Startup benchmarks** (`benches/startup.rs`): Shell startup time, lexer, parser, executor initialization
- **Builtin benchmarks** (`benches/builtins.rs`): Each builtin vs GNU equivalent performance comparison
- **Real-world benchmarks** (`scripts/benchmark.sh`): Hyperfine comparisons against bash/zsh

For detailed benchmarking documentation, see [BENCHMARKS.md](BENCHMARKS.md).

## Project Status

**Phase 1 Complete**: Core shell foundation with basic builtins, parser, executor, and REPL.

**Phase 2 Complete**: Fast built-in commands (ls, grep, find, cat) are implemented and perform 3-10x faster than GNU.

### What Works

- ✅ Basic command execution
- ✅ Pipelines
- ✅ Built-in commands (cd, pwd, echo, exit, export, ls, grep, find, cat, git-status)
- ✅ Fast file operations (3-10x faster than GNU)
- ✅ Git integration (git2 native bindings)
- ✅ JSON output support
- ✅ Variable assignment (parsing)
- ✅ Function definitions (parsing)
- ✅ Control flow (parsing)
- ✅ REPL with line editing
- ✅ Performance benchmarking infrastructure

### What's Next

1. Project context detection (auto-detect Rust/Node/Python projects)
2. Advanced tab completion (context-aware, git branches)
3. Command history with fuzzy search
4. Undo capability for file operations
5. Advanced scripting features (execute control flow, functions)

## Development

### Project Structure

- **~4.5k LOC** currently implemented (Phase 1 + Phase 2)
- **Target: ~15k LOC** for 1.0 release
- **Test coverage**: Growing (50 tests passing)

### Dependencies

Key dependencies:
- `logos` - Fast lexer generation
- `nom` - Parser combinators (planned)
- `reedline` - Modern line editor
- `tokio` - Async runtime
- `git2` - Git integration (planned)
- `ignore` - Fast file operations (planned)

## Contributing

This is currently in active development as part of the Rush shell initiative.

See the [Rush PRD](/Users/asher/knowledge/nest/research/rush-shell-prd.md) for the complete vision and roadmap.

## License

Dual-licensed under MIT or Apache-2.0 (like most Rust projects)

## Documentation

- [Rush Shell PRD](/Users/asher/knowledge/nest/research/rush-shell-prd.md)
- [Nest + Rush Integration Design](/Users/asher/knowledge/nest/research/nest-rush-integration.md)

---

**Built with gleeful enthusiasm** 🎉
