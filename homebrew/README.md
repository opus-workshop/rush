# Homebrew Tap for Rush

This directory contains the Homebrew formula for [Rush](https://github.com/opus-workshop/rush), a high-performance POSIX-compliant shell written in Rust.

## Installation

```bash
# Add the tap
brew tap opus-workshop/rush https://github.com/opus-workshop/rush

# Install Rush
brew install rush
```

## Usage

After installation, you can run Rush:

```bash
rush                      # Start interactive shell
rush -c "echo hello"      # Run a command
rush script.sh            # Run a script
```

## Setting as Default Shell

To use Rush as your default shell:

```bash
# Add to allowed shells
echo "$(brew --prefix)/bin/rush" | sudo tee -a /etc/shells

# Change your shell
chsh -s "$(brew --prefix)/bin/rush"
```

## Daemon Mode

For ultra-fast startup (~0.4ms), use daemon mode:

```bash
rushd start               # Start the daemon
rush -c "ls"              # Commands use the daemon
rushd stop                # Stop the daemon
```

## Updating

```bash
brew update
brew upgrade rush
```

## Uninstalling

```bash
brew uninstall rush
brew untap opus-workshop/rush
```
