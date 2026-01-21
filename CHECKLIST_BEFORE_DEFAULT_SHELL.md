# Checklist Before Making Rush Your Default Shell

## Critical (Must Work)

### Basic Shell Functionality
- [ ] Basic commands work: `pwd`, `ls`, `cd`, `echo`
- [ ] Tab completion works
- [ ] Command history works (up/down arrows)
- [ ] Ctrl-C interrupts commands without crashing
- [ ] Ctrl-D exits shell cleanly
- [ ] Environment variables work: `$HOME`, `$USER`, `$PATH`

### Development Tools
- [ ] **claude-code works** (most important!)
- [ ] git commands work: `git status`, `git log`, `git commit`
- [ ] cargo works: `cargo build`, `cargo test`, `cargo run`
- [ ] npm/node work (if you use them)
- [ ] python/pip work (if you use them)

### Advanced Shell Features
- [ ] Pipes work: `ls | grep rush`
- [ ] Redirects work: `echo test > file.txt`
- [ ] Command substitution works: `echo $(pwd)`
- [ ] Background jobs work: `sleep 5 &`
- [ ] Conditionals work: `ls && echo success`

### File Operations
- [ ] File paths with spaces work: `cat "file with spaces.txt"`
- [ ] Tilde expansion works: `cd ~/Documents`
- [ ] Wildcards work: `ls *.rs`

## Important (Should Work)

### Configuration
- [ ] Create `~/.rush_profile` for login shell initialization
- [ ] Create `~/.rushrc` for interactive shell initialization
- [ ] Set up PATH and other env vars in config files
- [ ] Test sourcing config files: `source ~/.rushrc`

### Integration
- [ ] Can run scripts: `./script.sh`
- [ ] Shebang scripts work: `#!/usr/bin/env rush`
- [ ] Can pipe into rush: `echo "pwd" | rush`
- [ ] Non-interactive mode works for automation

### Quality of Life
- [ ] Prompt looks good and updates with directory changes
- [ ] Error messages are clear
- [ ] No random crashes or panics
- [ ] Performance is acceptable (commands don't lag)

## Nice to Have (Optional)

- [ ] Syntax highlighting (not implemented yet)
- [ ] Custom prompt configuration
- [ ] Aliases (not implemented yet)
- [ ] Functions (not implemented yet)

## Testing Steps

### 1. Test in Rush (Not as Default Yet)
```bash
cd /Users/asher/knowledge/rush
./target/release/rush

# Run through the checklist above
# Exit with: exit
```

### 2. Test Claude Code
```bash
./target/release/rush
claude
# Try reading files, running commands, etc.
exit
```

### 3. Test a Full Work Session
```bash
./target/release/rush

# Do your normal development work for 30 minutes
# - Edit files
# - Run tests
# - Make git commits
# - Use claude-code

exit
```

## If Everything Works

Then proceed with:
1. Create config files (`~/.rush_profile`, `~/.rushrc`)
2. Install rush to `/usr/local/bin/rush`
3. Add rush to `/etc/shells`
4. Run `chsh -s /usr/local/bin/rush`
5. Open new terminal - rush should be your shell!

## Rollback Plan

If you switch and it doesn't work:
```bash
# Option 1: Change back in current terminal
chsh -s /bin/zsh

# Option 2: From another terminal (iTerm2, VS Code terminal)
chsh -s $(cat ~/.backup_shell)

# Option 3: Recovery mode (if terminal won't start)
# Boot to recovery mode (Cmd+R on restart)
# Open Terminal
# Mount your drive: mount -uw /
# Edit: nano /etc/passwd
# Change your shell back to /bin/zsh
```

## Current Status

Rush has:
✅ Non-TTY mode
✅ Signal handling
✅ File redirection
✅ Subshells
✅ Exit codes
✅ Variable expansion
✅ Wildcard expansion
✅ Command substitution
✅ Job control
✅ Error recovery
✅ Login shell init
✅ Shell options (set -e, -u, -x, etc.)
✅ 365+ passing tests

Ready for testing!
