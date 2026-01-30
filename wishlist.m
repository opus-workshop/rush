Wishlist: What Rush needs to replace your Daily Driver

Rush is currently optimized for machines (AI agents, CI pipelines). To replace Bash/Zsh for humans, it likely needs these features found in modern shells like Fish or Nushell:
1. The "Quality of Life" Tier (Fish/Zsh features)

    Phantom Text Autosuggestion: As you type gi, it should grey-text suggest git commit -m "fix bug" based on your history (like Fish/Zsh). Rush lists this as "In Progress."

    Syntax Highlighting: Red text for invalid commands, green for valid ones, and highlighted file paths before you even hit enter.

    Fuzzy History Search (Ctrl+R): A built-in, rich UI to search past commands (like fzf) without needing an external plugin.

2. The "Safety & Modernity" Tier

    Native Windows Support: Currently, Rush requires WSL on Windows. A native .exe that works with PowerShell seamlessly would be a huge adoption driver.

    Structured Configuration: Instead of writing a complex .rushrc shell script, a rush.toml file for configuring aliases, paths, and colors would be more "Rust-like" and less error-prone.

    Undo Capability: The README mentions this as "In Progress," but an undo command that reverses a mkdir, cp, or mv operation would be a killer feature Bash doesn't have.

3. The "AI & Data" Tier (The unique opportunity)

    "Pipe to LLM" Operator: Imagine a native operator like |? that sends output to a local LLM.

        Example: git status |? "write a commit message for this"

    Natural Language to Command: A keybinding that lets you type "find all rust files changed yesterday" and Rush inserts find . -name "*.rs" -mtime -1 into your prompt.

    Structured Tables: While Rush supports JSON output, a visual table viewer (like Nushell's) that lets you interactively sort/filter output (e.g., clicking the "Size" column header in ls) would be powerful.