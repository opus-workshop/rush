# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

### Added
- POSIX-compliant shell with 45+ built-in commands
- Daemon mode with pre-forked worker pool for sub-millisecond dispatch
- AI agent optimized builtins with `--json` output
- Built-in `git_status`, `git_log`, `git_diff` commands
- Built-in `find`, `grep`, `ls`, `cat` with JSON output
- HTTP `fetch` builtin for API calls
- Job control (bg, fg, jobs, wait)
- Command history with file persistence
- Tab completion
- Signal handling (SIGINT, SIGTSTP, SIGCHLD, SIGTERM)
- Variable expansion, command substitution, arithmetic expansion
- Here documents and here strings
- Functions with local variables
- Comprehensive test suite (500+ tests)
