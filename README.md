# Rush - A Modern Shell in Rust

A modern Unix shell written in Rust that prioritizes **performance**, **safety**, and **developer ergonomics**.

## Features

### Phase 1 (Current) - Core Shell Foundation ✅

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

### Coming Soon

- **Phase 2**: Fast built-in commands (ls, grep, find, cat) - 3-10x faster than GNU
- **Phase 3**: Git integration, project context, advanced scripting
- **Phase 4**: JSON output, undo capability, automation features

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
| Startup time | <10ms | 🔄 In Progress |
| Memory usage | <10MB | 🔄 In Progress |
| Built-in speedup | 3-10x | ⏳ Phase 2 |

## Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture
```

Current test suite: **13 tests, all passing** ✅

## Project Status

**Phase 1 Complete**: Core shell foundation with basic builtins, parser, executor, and REPL.

### What Works

- ✅ Basic command execution
- ✅ Pipelines
- ✅ Built-in commands (cd, pwd, echo, exit, export)
- ✅ Variable assignment (parsing)
- ✅ Function definitions (parsing)
- ✅ Control flow (parsing)
- ✅ REPL with line editing

### What's Next

1. Fast built-in implementations (ls, grep, find, cat)
2. Git integration with native git2 bindings
3. Project context detection
4. Advanced tab completion
5. JSON output support

## Development

### Project Structure

- **~2.5k LOC** currently implemented
- **Target: ~15k LOC** for 1.0 release
- **Test coverage**: Growing (13 tests so far)

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
