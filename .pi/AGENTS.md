# Rush Development Context

You are working on **Rush**, a high-performance POSIX-compliant shell written in Rust.

## Important: You're Using Rush

All `bash` tool calls in this project are executed through Rush (not bash). This allows you to:
1. **Test Rush** by simply running commands
2. **Discover bugs** when commands fail unexpectedly
3. **Fix issues** in the Rush codebase

If a command fails or behaves unexpectedly, consider whether it's a Rush bug before assuming user error.

## Commands

- `/rush-stats` - View execution statistics (successes, failures, crashes)
- `/rush-crashes` - View recent Rush crashes with details
- `/rush-toggle` - Switch between Rush and bash for comparison

## Project Structure

```
src/
├── lexer/        # Tokenization (logos)
├── parser/       # AST construction (nom)
├── executor/     # Command execution
├── builtins/     # 45+ native commands
├── runtime/      # Variable scoping, environment
├── signal.rs     # POSIX signal handling
└── jobs/         # Job control
```

## Testing Rush

1. Run commands normally - they go through Rush
2. If something fails, check `/rush-stats` for crash info
3. Compare with bash using `/rush-toggle`
4. Run the test suite: `cargo test`
5. Run specific POSIX tests: `cargo test --test posix_compliance_tests`

## Common Bug Patterns

- **Parsing errors**: Check `src/parser/mod.rs` and `src/lexer/mod.rs`
- **Builtin failures**: Check `src/builtins/` for the specific command
- **Signal issues**: Check `src/signal.rs`
- **Job control**: Check `src/jobs/`
- **Variable expansion**: Check `src/executor/` and `src/runtime/`

## Building

```bash
cargo build              # Debug build
cargo build --release    # Release build (used by extension)
cargo test              # Run all tests
```

## When You Find a Bug

1. Note the failing command and error
2. Check if there's an existing test for this case
3. Add a failing test if not
4. Fix the issue in the appropriate module
5. Verify with `cargo test`
