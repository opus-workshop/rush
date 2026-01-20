# Login Shell Initialization

This document describes Rush's login shell initialization behavior and configuration file system.

## Overview

Rush supports POSIX-style shell initialization with profile files for login shells and RC files for interactive shells. This allows users to customize their environment and set up aliases, functions, and environment variables.

## Shell Types

### Login Shell

A login shell is the first shell you get when you log into a system. Rush detects login shells in two ways:

1. **Automatic Detection**: When the first character of `argv[0]` is `-` (e.g., `-rush`)
2. **Explicit Flag**: Using the `--login` or `-l` flag

```bash
rush --login          # Explicitly start as login shell
-rush                 # Started as login shell by system
```

### Interactive Shell

An interactive shell is a shell where you can type commands interactively. Rush automatically detects if it's running in interactive mode by checking if stdin is a TTY.

## Configuration Files

### ~/.rush_profile

Sourced when Rush starts as a **login shell**. This file is typically used for:

- Setting environment variables (`export PATH=$PATH:/custom/bin`)
- Setting up the terminal (`export TERM=xterm-256color`)
- Loading system-wide settings
- One-time initialization tasks

Example `~/.rush_profile`:

```bash
# Set up PATH
export PATH=$HOME/bin:/usr/local/bin:$PATH

# Set environment variables
export EDITOR=vim
export VISUAL=vim

# Set up language and locale
export LANG=en_US.UTF-8

# Custom greeting
echo "Welcome to Rush Shell!"
```

### ~/.rushrc

Sourced for **all interactive shells** (including login shells). This file is typically used for:

- Defining aliases
- Setting shell options
- Defining functions
- Setting up prompt customization

Example `~/.rushrc`:

```bash
# Aliases
export LS_ALIAS=ls -la
export GREP_ALIAS=grep --color=auto

# Functions
fn greet(name) {
    echo "Hello, $name!"
}

# Shell history settings (when implemented)
# export HISTSIZE=10000
# export HISTFILE=$HOME/.rush_history
```

## Initialization Order

When Rush starts, it initializes in the following order:

1. **Set Environment Variables**:
   - `$SHELL` - Path to the Rush executable
   - `$TERM` - Terminal type (if not already set)
   - `$USER` - Username (if not already set)
   - `$HOME` - Home directory (if not already set)

2. **Login Shell** (if `--login` or argv[0] starts with `-`):
   - Source `~/.rush_profile` (if it exists)

3. **Interactive Shell** (if stdin is a TTY):
   - Source `~/.rushrc` (if it exists)

4. **Start Shell**:
   - Enter interactive mode with REPL
   - OR execute the provided command/script

## Command-Line Flags

### --login, -l

Forces Rush to behave as a login shell, sourcing `~/.rush_profile`.

```bash
rush --login
rush -l
```

### --no-rc, --norc

Skips sourcing all configuration files (both `~/.rush_profile` and `~/.rushrc`).

```bash
rush --no-rc           # Start without loading config files
rush --login --no-rc   # Login shell but skip config files
```

### -c command

Execute a command and exit. Does not source config files.

```bash
rush -c "echo hello"
rush -c "ls -la | grep txt"
```

## The source Builtin

Rush provides a `source` builtin command to execute commands from a file in the current shell context. This is useful for:

- Loading configuration files manually
- Reloading configuration after changes
- Sourcing utility scripts

```bash
source ~/.rushrc               # Reload rushrc
source ~/scripts/aliases.rush  # Load custom aliases
source ~/.rush_profile         # Reload profile
```

### Features

- **Tilde Expansion**: `source ~/.rushrc` expands `~` to home directory
- **Relative Paths**: Resolved relative to current working directory
- **Error Handling**: Continues executing even if individual lines fail
- **Comments**: Lines starting with `#` are ignored
- **Empty Lines**: Blank lines are skipped

### Syntax

```bash
source <file>
source ~/config.rush
source /absolute/path/to/file.rush
source relative/path/to/file.rush
```

## Environment Variables

Rush automatically sets the following environment variables if they are not already defined:

### $SHELL

Path to the Rush executable. Used by other programs to determine the user's shell.

```bash
echo $SHELL  # /usr/local/bin/rush
```

### $TERM

Terminal type. Defaults to `xterm-256color` if not set.

```bash
echo $TERM  # xterm-256color
```

### $USER

Current username. Derived from `$LOGNAME` or system user information.

```bash
echo $USER  # yourusername
```

### $HOME

Home directory path. Derived from system home directory.

```bash
echo $HOME  # /home/yourusername
```

## Best Practices

### Separate Concerns

- Put **environment variables** in `~/.rush_profile`
- Put **interactive settings** (aliases, functions) in `~/.rushrc`

### Keep It Fast

Configuration files are sourced on every shell start. Keep them fast by:

- Avoiding expensive operations
- Using conditional logic to skip unnecessary work
- Moving rarely-used functions to separate files

### Use Comments

Document your configuration files well:

```bash
# ~/.rushrc - Rush shell interactive configuration

# Aliases for common operations
export LS_ALIAS=ls -lah
export GREP_ALIAS=grep --color=auto

# Development shortcuts
export DEV_DIR=$HOME/projects
```

### Test Configuration

Test your configuration files before using them:

```bash
# Test without loading your real config
rush --no-rc -c "source ~/test_config.rush"
```

## Compatibility Notes

### POSIX Shells (bash, zsh)

Rush's initialization is inspired by POSIX shells but has some differences:

- **bash**: Uses `~/.bash_profile` or `~/.profile` for login, `~/.bashrc` for interactive
- **zsh**: Uses `~/.zprofile` for login, `~/.zshrc` for interactive
- **rush**: Uses `~/.rush_profile` for login, `~/.rushrc` for interactive

### Migration from Other Shells

If migrating from bash or zsh, you can:

1. Copy relevant settings from `~/.bash_profile` to `~/.rush_profile`
2. Copy relevant settings from `~/.bashrc` to `~/.rushrc`
3. Adjust for Rush syntax differences (especially function definitions)

## Examples

### Complete Login Profile

```bash
# ~/.rush_profile - Login shell initialization

# Path configuration
export PATH=$HOME/bin:$HOME/.local/bin:/usr/local/bin:$PATH

# Language and locale
export LANG=en_US.UTF-8
export LC_ALL=en_US.UTF-8

# Editor configuration
export EDITOR=vim
export VISUAL=vim

# XDG directories
export XDG_CONFIG_HOME=$HOME/.config
export XDG_DATA_HOME=$HOME/.local/share
export XDG_CACHE_HOME=$HOME/.cache

# Development environment
export RUST_BACKTRACE=1
export CARGO_HOME=$HOME/.cargo

# Less pager configuration
export LESS=-R
export LESS_TERMCAP_mb=$'\E[1;31m'
export LESS_TERMCAP_md=$'\E[1;36m'
export LESS_TERMCAP_me=$'\E[0m'

# Source .rushrc for interactive login shells
if [ -f ~/.rushrc ]; then
    source ~/.rushrc
fi
```

### Complete Interactive RC

```bash
# ~/.rushrc - Interactive shell configuration

# Aliases
export LS_ALIAS=ls -lah --color=auto
export GREP_ALIAS=grep --color=auto
export EGREP_ALIAS=egrep --color=auto

# Git aliases (when git command is available)
export G_ALIAS=git
export GS_ALIAS=git status
export GA_ALIAS=git add
export GC_ALIAS=git commit

# Directory shortcuts
export PROJ=$HOME/projects
export DOC=$HOME/Documents
export DL=$HOME/Downloads

# Functions
fn mkcd(dir) {
    mkdir -p $dir && cd $dir
}

fn extract(file) {
    if [ -f $file ]; then
        # Extraction logic here
        echo "Extracting $file..."
    fi
}

# Welcome message
echo "Rush shell ready. Type 'exit' to quit."
```

## Troubleshooting

### Config File Not Loading

1. Check file exists: `ls -la ~/.rushrc ~/.rush_profile`
2. Check file permissions: `chmod 644 ~/.rushrc ~/.rush_profile`
3. Check for syntax errors: `rush --no-rc -c "source ~/.rushrc"`

### Variables Not Set

1. Verify export statement: `export VAR=value` not `VAR=value`
2. Check if config file is being sourced (add `echo "Loading rushrc"` at top)
3. Use `--login` flag if you need login shell behavior

### Slow Startup

1. Profile your config files by adding timing statements
2. Remove expensive operations
3. Consider lazy-loading functions and aliases

## Future Enhancements

Planned features for future versions:

- `~/.rush_logout` for logout cleanup
- `$RUSHOPTS` for shell option configuration
- `~/.config/rush/rushrc` for XDG-compliant configuration
- Per-directory `.rushrc` files (similar to `.envrc`)
