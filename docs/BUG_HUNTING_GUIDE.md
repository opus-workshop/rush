# Bug Hunting Guide for Rush Shell

A systematic approach to finding bugs in Rush before using it as your daily shell.

## Philosophy

Don't try to test everything. Instead:
1. Do what you normally do
2. Notice when things break
3. Try to reproduce the break
4. File a bead issue if reproducible

This guide helps you be systematic about #1.

---

## Quick Start: 30-Minute Bug Hunt

Run through these common scenarios quickly:

```bash
# Build fresh
cargo build --release

# Start Rush
./target/release/rush

# Test basic operations (5 min)
pwd
cd /tmp
cd ~
ls
ls -la
echo "hello world"
cat README.md
cat README.md | grep Rush

# Test pipelines (5 min)
ls | wc -l
find . -name "*.rs" | grep main
cat Cargo.toml | grep name | head -5

# Test redirects (5 min)
echo "test" > /tmp/rush_test.txt
cat /tmp/rush_test.txt
echo "more" >> /tmp/rush_test.txt
cat /tmp/rush_test.txt
rm /tmp/rush_test.txt

# Test variables (5 min)
export FOO=bar
echo $FOO
echo "value: $FOO"
cd $(pwd)

# Test job control (5 min)
sleep 10 &
jobs
# Press Ctrl-C a few times
# Type random commands

# Test error handling (5 min)
lksjdflkjsdf  # command not found
ls --invalid-flag
cat /nonexistent/file
```

If anything breaks, note it and continue. File issues later.

---

## Day 1: Basic Shell Operations

### Hour 1: Navigation

```bash
# Test directory navigation
cd ~
cd /
cd /usr/local/bin
cd ../..
cd ~-  # Should fail gracefully (not implemented)
cd     # Should go to $HOME

# Test with spaces in paths
mkdir "/tmp/rush test dir"
cd "/tmp/rush test dir"
pwd
cd ~
rmdir "/tmp/rush test dir"

# Test with special characters
cd ~/.ssh  # If exists
cd ..

# What to watch for:
# - Does cd always update PWD correctly?
# - Do error messages make sense?
# - Can you cd to symlinks?
```

### Hour 2: File Operations

```bash
# Test ls variations
ls
ls -l
ls -la
ls -lah
ls *.rs
ls src/*.rs
ls nonexistent  # Should show error

# Test cat variations
cat Cargo.toml
cat README.md
cat large_file.txt  # If you have one
cat binary_file     # Should detect and warn
cat file1 file2     # Multiple files

# Test grep
grep "Rush" README.md
grep -i "rush" README.md
grep -r "TODO" src/
grep "nonexistent" file  # Should show "no matches"

# Test find
find . -name "*.rs"
find . -type f
find . -name "Cargo.toml"
find /nonexistent -name "*.rs"  # Should handle gracefully

# What to watch for:
# - Does output look right?
# - Are colors working?
# - Does it handle missing files?
# - Does performance feel good?
```

### Hour 3: Pipelines

```bash
# Simple pipes
ls | wc -l
cat Cargo.toml | grep name
echo "hello" | cat

# Multi-stage pipes
find . -name "*.rs" | grep main | wc -l
ls -la | grep "^d" | wc -l
cat README.md | grep Rush | head -3

# Pipes with errors
false | echo "after false"
ls nonexistent | grep foo  # How does it handle?

# What to watch for:
# - Do all stages execute?
# - Is output correct?
# - What happens when middle stages fail?
# - Check $? exit code
```

---

## Day 2: Advanced Features

### Hour 1: Redirections

```bash
# Output redirection
echo "test" > /tmp/out.txt
cat /tmp/out.txt

# Append
echo "more" >> /tmp/out.txt
cat /tmp/out.txt

# Stderr redirect
ls nonexistent 2> /tmp/err.txt
cat /tmp/err.txt

# Combined redirect
ls nonexistent &> /tmp/both.txt
cat /tmp/both.txt

# Redirect and pipe
cat Cargo.toml | grep name > /tmp/names.txt
cat /tmp/names.txt

# Clean up
rm /tmp/*.txt

# What to watch for:
# - Are files created correctly?
# - Does append work?
# - Is stderr separated properly?
# - Can you redirect in pipelines?
```

### Hour 2: Variables & Expansion

```bash
# Basic variables
export NAME="Rush"
echo $NAME
echo "Hello $NAME"
echo 'Single quotes: $NAME'  # Should be literal

# Command substitution
echo "Current dir: $(pwd)"
echo "File count: $(ls | wc -l)"
echo "Nested: $(echo $(pwd))"

# Variable expansion
export VAR="test"
echo ${VAR}
echo ${VAR:-default}  # Should use VAR
echo ${UNSET:-default}  # Should use default
unset VAR
echo ${VAR:-default}  # Should use default

# Globs
echo *.rs
echo src/*.rs
ls *.{rs,toml}  # Brace expansion
echo *  # Should expand
echo "*"  # Should be literal

# What to watch for:
# - Do variables expand in strings?
# - Does command substitution work?
# - Are globs expanding?
# - Does quoting work correctly?
```

### Hour 3: Subshells & Functions

```bash
# Subshells
(cd /tmp && pwd)
pwd  # Should still be original dir

(export SUBVAR=value; echo $SUBVAR)
echo $SUBVAR  # Should be empty

# Nested subshells
(echo outer; (echo inner; pwd); pwd)

# Functions (if supported)
fn greet(name) {
    echo "Hello, $name"
}
greet "Rush"

# What to watch for:
# - Do subshells isolate variables?
# - Does cd in subshell not affect parent?
# - Do nested subshells work?
# - Can you define and call functions?
```

---

## Day 3: Edge Cases & Stress Testing

### Hour 1: Error Conditions

```bash
# Missing commands
nonexistentcommand
asdfasdf

# Invalid syntax
echo "unclosed string
ls |  # Pipe to nothing
cd too many args

# Permission errors
cat /etc/sudoers  # If not root
cd /root          # If not root

# Large files (create test file first)
dd if=/dev/zero of=/tmp/large bs=1M count=100
cat /tmp/large  # Should use mmap
cat /tmp/large | head
rm /tmp/large

# What to watch for:
# - Are error messages helpful?
# - Does Rush recover from errors?
# - Does it crash on bad input?
# - Can you keep using shell after error?
```

### Hour 2: Signal Handling

```bash
# Ctrl-C during command
sleep 30
# Press Ctrl-C - should cancel and return to prompt

# Ctrl-C during pipeline
find / -name "*.rs" | grep test
# Press Ctrl-C - should cancel both stages

# Background job interruption
sleep 60 &
jobs
# Press Ctrl-C - should not kill background job
jobs
kill %1

# Multiple Ctrl-C
# Press Ctrl-C 5 times rapidly
# Should remain stable

# What to watch for:
# - Does Ctrl-C always work?
# - Do background jobs survive Ctrl-C?
# - Are child processes cleaned up?
# - Does shell remain usable after signals?
```

### Hour 3: Performance & Limits

```bash
# Many files
mkdir /tmp/manyfiles
cd /tmp/manyfiles
for i in {1..1000}; do touch file_$i.txt; done
ls
ls | wc -l
find . -name "*.txt" | wc -l
cd ~
rm -rf /tmp/manyfiles

# Deep directory trees
mkdir -p /tmp/deep/a/b/c/d/e/f/g/h/i/j
cd /tmp/deep/a/b/c/d/e/f/g/h/i/j
pwd
cd ~
rm -rf /tmp/deep

# Long pipelines
echo "start" | cat | cat | cat | cat | cat | cat | cat | cat | cat | cat

# Large command history
# Type many commands (50+)
# Press up arrow repeatedly
# Search history

# What to watch for:
# - Does performance stay good?
# - Any memory leaks over time?
# - Do long pipelines work?
# - Is history search fast?
```

---

## Day 4: Integration with Real Workflows

### Scenario: Git Workflow

```bash
cd ~/your-git-repo

# Basic git commands
git status
git log
git branch
git diff

# With Rush features
git status | grep modified
git branch | grep feature
git log | head -10

# Complex git operations
git log --oneline | grep "feat:" | wc -l
git diff | grep "^+" | wc -l

# What to watch for:
# - Does git output look correct?
# - Do colors work?
# - Can you pipe git output?
```

### Scenario: Development Workflow

```bash
cd ~/your-rust-project

# Cargo commands
cargo build
cargo test
cargo check

# Piped cargo output
cargo test 2>&1 | grep -i fail
cargo build 2>&1 | grep error

# File searching
find . -name "*.rs" | grep test
grep -r "TODO" src/ | wc -l
cat Cargo.toml | grep dependencies

# What to watch for:
# - Does cargo work normally?
# - Can you capture stderr?
# - Is search fast?
```

### Scenario: System Administration

```bash
# Process management
ps aux | grep rush
ps aux | head

# Disk usage
du -sh *
df -h

# Network (if applicable)
curl https://example.com
curl -s https://api.github.com | grep login

# System info
uname -a
env | grep PATH
env | sort

# What to watch for:
# - Do system commands work?
# - Can you process their output?
# - Any issues with external tools?
```

---

## Day 5: Rush-Specific Features

### Test: History

```bash
# Search history
# Press Ctrl-R and type "cargo"
# Should fuzzy search

# Navigate history
# Press up arrow
# Press down arrow
# Press up multiple times

# History command
history
history | tail -20
history | grep git

# What to watch for:
# - Is history saved between sessions?
# - Does fuzzy search work?
# - Are timestamps correct?
```

### Test: Completion

```bash
# Command completion
ca<TAB>  # Should complete to cat/cargo/etc
pw<TAB>  # Should complete to pwd

# Path completion
ls ~/Doc<TAB>  # Should complete Documents
cat ./src/ma<TAB>  # Should complete

# Git completion
git chec<TAB>  # Should complete checkout
git checkout ma<TAB>  # Should complete branch names

# Cargo completion
cargo bu<TAB>  # Should complete build

# What to watch for:
# - Does Tab work?
# - Are completions relevant?
# - Is it fast?
# - Does it handle multiple matches?
```

### Test: Aliases

```bash
# Create aliases
alias ll='ls -la'
alias gs='git status'
alias ..='cd ..'

# Use aliases
ll
gs
..

# List aliases
alias

# Remove alias
unalias ll
ll  # Should fail

# What to watch for:
# - Do aliases expand correctly?
# - Can you override commands?
# - Does unalias work?
```

### Test: Undo

```bash
# Create test file
echo "original" > /tmp/test_undo.txt
cat /tmp/test_undo.txt

# Track and modify
echo "modified" > /tmp/test_undo.txt
cat /tmp/test_undo.txt

# Undo
undo last
cat /tmp/test_undo.txt  # Should be "original"

# Delete and undo
rm /tmp/test_undo.txt
ls /tmp/test_undo.txt  # Should fail
undo last
ls /tmp/test_undo.txt  # Should exist

# Clean up
rm /tmp/test_undo.txt

# What to watch for:
# - Does undo restore correctly?
# - Can you undo deletes?
# - Does it handle edge cases?
```

---

## Continuous Testing: Daily Use Checklist

Once you're using Rush daily, watch for these patterns:

### Morning Checklist (2 min)
```bash
# Open new Rush shell
rush

# Quick sanity check
pwd
ls
git status  # If in a repo
history | tail -5

# If anything looks wrong, note it
```

### During Work (ongoing)
- Note any error messages that are confusing
- Notice if any command is slower than expected
- Watch for any weird output formatting
- Check if Ctrl-C always works
- Verify background jobs behave correctly

### End of Day (2 min)
```bash
# Check history saved correctly
history | tail -20

# Note any crashes or hangs during the day
# File issues for reproducible bugs

# Exit cleanly
exit
```

---

## Bug Reporting Template

When you find a bug, capture:

```markdown
## Bug: [Short description]

**What I did:**
```bash
cd /tmp
echo "test" | grep foo
```

**What I expected:**
Should print nothing and exit with code 1

**What actually happened:**
[Error message or wrong behavior]

**Environment:**
- Rush version: [commit hash from git log]
- OS: macOS/Linux
- Shell started as: interactive/script/login

**Can reproduce:**
- [ ] Always
- [ ] Sometimes
- [ ] Once

**Impact:**
- [ ] Blocking (can't use Rush)
- [ ] Annoying (workaround exists)
- [ ] Minor (barely noticeable)
```

---

## Common Bug Categories

### 1. Parse Errors
**Symptoms:** Syntax that should work doesn't parse
**Test:** Try similar bash syntax
**Report:** Include exact command that fails

### 2. Execution Errors
**Symptoms:** Parsed correctly but executes wrong
**Test:** Compare with bash output
**Report:** Show diff between expected/actual

### 3. Performance Issues
**Symptoms:** Command is slow
**Test:** Compare timing with bash
**Report:** Include file counts, sizes

### 4. Memory Leaks
**Symptoms:** Rush gets slower over time
**Test:** Run `ps aux | grep rush` periodically
**Report:** Show memory growth over time

### 5. Crash/Panic
**Symptoms:** Rush exits unexpectedly
**Test:** Try to reproduce
**Report:** Include panic message if any

### 6. Zombie Processes
**Symptoms:** Child processes don't die
**Test:** Run `ps aux` after command
**Report:** Show orphaned process tree

---

## What NOT to Report

Don't file bugs for:
- Features that aren't implemented (check docs first)
- Bash-isms that Rush doesn't claim to support
- "It would be nice if..." (use beads for features)
- Performance being "only" 50x faster instead of 200x
- Cosmetic issues unless they affect usability

---

## Automated Bug Hunting

Run the test suite before daily use:

```bash
# Full test suite
cargo test

# Integration tests
cargo test --test '*'

# Specific test
cargo test test_pipeline

# With output
cargo test -- --nocapture

# If tests fail, you found a regression!
```

---

## The Real Test

The ultimate bug hunt is simple:

**Use Rush for all your work for one week.**

Keep your old shell (zsh/bash) ready to switch back. When something breaks:

1. Note what broke
2. Try to reproduce it
3. File a bead if reproducible
4. Switch back to old shell to keep working
5. Fix the bug
6. Try Rush again

Repeat until you can go a full week without needing to switch back.

That's when Rush is ready to be your daily shell.
