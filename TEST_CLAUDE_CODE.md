# Testing Claude Code in Rush

## Quick Test

Run rush and try these commands to verify claude-code works:

```bash
# Start rush
./target/release/rush

# Test 1: Basic commands
pwd
ls
echo "Hello from Rush!"

# Test 2: Git integration
git status
git log --oneline -5

# Test 3: Environment variables
echo $HOME
echo $USER
echo $SHELL

# Test 4: Try running claude
claude --version

# Test 5: Tab completion (press TAB)
git <TAB>
cargo <TAB>

# Test 6: Command history (press UP arrow)
# Should show previous commands

# Test 7: Ctrl-C handling
sleep 10
# Press Ctrl-C - should return to prompt

# Exit rush when done
exit
```

## Full Claude Code Test

Once basic commands work, test a full claude-code session:

```bash
./target/release/rush

# Start claude-code in rush
claude

# Try these in the claude session:
# - List files: ask "what files are in this directory?"
# - Read a file: ask "read src/main.rs"
# - Run a command: ask "run cargo test --lib"
# - Git operations: ask "what's the git status?"
```

## Things to Watch For

✅ **Must work:**
- Basic command execution
- Tab completion
- Command history (arrow keys)
- Ctrl-C (SIGINT) handling
- Environment variables
- Git commands
- Cargo commands
- Claude/claude-code execution

⚠️ **Known issues to check:**
- Does claude-code's interactive input work?
- Do prompts render correctly?
- Can you interrupt long-running commands?
- Does command substitution work: `echo $(pwd)`

## Rollback if Issues

If anything doesn't work:
```bash
# Just exit rush
exit

# You'll be back in your normal shell
# No changes have been made to your system yet
```

## Report Issues

If you find problems, document:
1. What command you ran
2. What happened vs what you expected
3. Any error messages
4. Whether Ctrl-C works to recover
