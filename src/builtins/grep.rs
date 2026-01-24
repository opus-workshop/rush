use crate::executor::ExecutionResult;
use crate::executor::Output;
use crate::runtime::Runtime;
use anyhow::{anyhow, Result};
use grep_matcher::Matcher;
use grep_regex::RegexMatcherBuilder;
use grep_searcher::sinks::UTF8;
use grep_searcher::{SearcherBuilder, BinaryDetection};
use ignore::WalkBuilder;
use std::cell::Cell;
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn builtin_grep(args: &[String], runtime: &mut Runtime) -> Result<ExecutionResult> {
    let config = parse_args(args)?;

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut found_any = false;

    // Build the regex matcher
    let matcher = RegexMatcherBuilder::new()
        .case_insensitive(config.ignore_case)
        .build(&config.pattern)
        .map_err(|e| anyhow!("Invalid regex pattern: {}", e))?;

    if config.recursive {
        // Recursive search through directories
        for path in &config.paths {
            let walker = WalkBuilder::new(path)
                .git_ignore(config.respect_gitignore)
                .hidden(!config.search_hidden)
                .build();

            for entry in walker {
                match entry {
                    Ok(entry) => {
                        if entry.file_type().is_some_and(|ft| ft.is_file()) && search_file(&matcher, entry.path(), &config, &mut stdout, &mut stderr)? {
                            found_any = true;
                        }
                    }
                    Err(e) => {
                        writeln!(&mut stderr, "Error walking directory: {}", e)?;
                    }
                }
            }
        }
    } else {
        // Search specific files
        for path in &config.paths {
            let path_buf = if path.is_absolute() {
                path.clone()
            } else {
                runtime.get_cwd().join(path)
            };

            if path_buf.is_dir() {
                writeln!(&mut stderr, "grep: {}: Is a directory", path.display())?;
                continue;
            }

            if !path_buf.exists() {
                writeln!(&mut stderr, "grep: {}: No such file or directory", path.display())?;
                continue;
            }

            if search_file(&matcher, &path_buf, &config, &mut stdout, &mut stderr)? {
                found_any = true;
            }
        }
    }

    let exit_code = if found_any { 0 } else { 1 };

    Ok(ExecutionResult {
        output: Output::Text(String::from_utf8_lossy(&stdout).to_string()),
        stderr: String::from_utf8_lossy(&stderr).to_string(),
        exit_code,
        error: None,
    })
}

/// Execute grep with stdin data (for pipelines)
pub fn builtin_grep_with_stdin(args: &[String], _runtime: &mut Runtime, stdin_data: &[u8]) -> Result<ExecutionResult> {
    let config = parse_args(args)?;

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    // Build the regex matcher
    let matcher = RegexMatcherBuilder::new()
        .case_insensitive(config.ignore_case)
        .build(&config.pattern)
        .map_err(|e| anyhow!("Invalid regex pattern: {}", e))?;

    // Search through stdin data
    let found = search_stdin(&matcher, stdin_data, &config, &mut stdout, &mut stderr)?;

    let exit_code = if found { 0 } else { 1 };

    Ok(ExecutionResult {
        output: Output::Text(String::from_utf8_lossy(&stdout).to_string()),
        stderr: String::from_utf8_lossy(&stderr).to_string(),
        exit_code,
        error: None,
    })
}

/// Search through stdin data
fn search_stdin(
    matcher: &impl Matcher,
    stdin_data: &[u8],
    config: &GrepConfig,
    stdout: &mut Vec<u8>,
    _stderr: &mut Vec<u8>,
) -> Result<bool> {
    let found = Cell::new(false);

    let mut searcher = SearcherBuilder::new()
        .binary_detection(BinaryDetection::quit(b'\x00'))
        .line_number(true) // Always track line numbers so lnum is valid
        .invert_match(config.invert_match)
        .build();

    let result = searcher.search_slice(
        matcher,
        stdin_data,
        UTF8(|lnum, line| {
            found.set(true);

            if config.show_line_numbers {
                write!(stdout, "{}:", lnum)?;
            }

            // Write line with color highlighting if enabled
            if config.color && !config.invert_match {
                if let Some(m) = matcher.find(line.as_bytes())
                    .map_err(|e| std::io::Error::other(e.to_string()))? {
                    write_colored_line(stdout, line, m.start(), m.end())
                        .map_err(|e| std::io::Error::other(e.to_string()))?;
                } else {
                    write!(stdout, "{}", line)?;
                }
            } else {
                write!(stdout, "{}", line)?;
            }

            Ok(true)
        }),
    );

    match result {
        Ok(_) => Ok(found.get()),
        Err(e) => Err(anyhow!("Error searching stdin: {}", e)),
    }
}

fn search_file(
    matcher: &impl Matcher,
    path: &Path,
    config: &GrepConfig,
    stdout: &mut Vec<u8>,
    stderr: &mut Vec<u8>,
) -> Result<bool> {
    let found = Cell::new(false);

    let mut searcher = SearcherBuilder::new()
        .binary_detection(BinaryDetection::quit(b'\x00'))
        .line_number(true) // Always track line numbers so lnum is valid
        .invert_match(config.invert_match)
        .build();

    let result = searcher.search_path(
        matcher,
        path,
        UTF8(|lnum, line| {
            found.set(true);

            if config.show_line_numbers {
                write!(stdout, "{}:", lnum)?;
            }

            if config.show_filename {
                write!(stdout, "{}:", path.display())?;
            }

            // Write line with color highlighting if enabled (only for normal matches, not inverted)
            if config.color && !config.invert_match {
                // Find first match for highlighting
                let match_result = matcher.find(line.as_bytes())
                    .map_err(|e| std::io::Error::other(e.to_string()))?;
                if let Some(m) = match_result {
                    write_colored_line(stdout, line, m.start(), m.end())
                        .map_err(|e| std::io::Error::other(e.to_string()))?;
                } else {
                    write!(stdout, "{}", line)?;
                }
            } else {
                write!(stdout, "{}", line)?;
            }

            Ok(true)
        }),
    );

    match result {
        Ok(_) => Ok(found.get()),
        Err(e) => {
            writeln!(stderr, "Error searching {}: {}", path.display(), e)?;
            Ok(false)
        }
    }
}

fn write_colored_line(
    stdout: &mut Vec<u8>,
    line: &str,
    match_start: usize,
    match_end: usize,
) -> Result<()> {
    // ANSI color codes for red highlighting
    const RED: &str = "\x1b[1;31m";
    const RESET: &str = "\x1b[0m";

    write!(
        stdout,
        "{}{}{}{}{}",
        &line[..match_start],
        RED,
        &line[match_start..match_end],
        RESET,
        &line[match_end..]
    )?;

    Ok(())
}

#[derive(Debug)]
struct GrepConfig {
    pattern: String,
    paths: Vec<PathBuf>,
    ignore_case: bool,
    show_line_numbers: bool,
    recursive: bool,
    invert_match: bool,
    respect_gitignore: bool,
    search_hidden: bool,
    show_filename: bool,
    color: bool,
}

impl Default for GrepConfig {
    fn default() -> Self {
        Self {
            pattern: String::new(),
            paths: vec![PathBuf::from(".")],
            ignore_case: false,
            show_line_numbers: true,
            recursive: false,
            invert_match: false,
            respect_gitignore: true,
            search_hidden: false,
            show_filename: false,
            color: true,
        }
    }
}

fn parse_args(args: &[String]) -> Result<GrepConfig> {
    let mut config = GrepConfig::default();
    let mut i = 0;

    // Parse flags
    while i < args.len() {
        let arg = &args[i];

        if !arg.starts_with('-') {
            break;
        }

        match arg.as_str() {
            "-i" | "--ignore-case" => {
                config.ignore_case = true;
            }
            "-n" | "--line-number" => {
                config.show_line_numbers = true;
            }
            "-N" | "--no-line-number" => {
                config.show_line_numbers = false;
            }
            "-r" | "-R" | "--recursive" => {
                config.recursive = true;
            }
            "-v" | "--invert-match" => {
                config.invert_match = true;
            }
            "-H" | "--with-filename" => {
                config.show_filename = true;
            }
            "-h" | "--no-filename" => {
                config.show_filename = false;
            }
            "--color" => {
                config.color = true;
            }
            "--no-color" => {
                config.color = false;
            }
            "--hidden" => {
                config.search_hidden = true;
            }
            "--no-ignore" => {
                config.respect_gitignore = false;
            }
            "--help" => {
                return Err(anyhow!(
                    "Usage: grep [OPTIONS] PATTERN [PATH...]\n\
                     \n\
                     OPTIONS:\n\
                     -i, --ignore-case       Case insensitive search\n\
                     -n, --line-number       Show line numbers (default)\n\
                     -N, --no-line-number    Don't show line numbers\n\
                     -r, --recursive         Recursively search directories\n\
                     -v, --invert-match      Select non-matching lines\n\
                     -H, --with-filename     Show filename (default for multiple files)\n\
                     -h, --no-filename       Don't show filename\n\
                     --color                 Colorize output (default)\n\
                     --no-color              Don't colorize output\n\
                     --hidden                Search hidden files\n\
                     --no-ignore             Don't respect .gitignore\n\
                     --help                  Show this help message"
                ));
            }
            _ => {
                return Err(anyhow!("Unknown option: {}", arg));
            }
        }

        i += 1;
    }

    // Get pattern
    if i >= args.len() {
        return Err(anyhow!("grep: missing pattern argument"));
    }
    config.pattern = args[i].clone();
    i += 1;

    // Get paths (default to current directory)
    if i < args.len() {
        config.paths = args[i..].iter().map(PathBuf::from).collect();

        // Auto-enable filename display for multiple files
        if config.paths.len() > 1 {
            config.show_filename = true;
        }
    }

    Ok(config)
}

// Tests are in tests/grep_integration_test.rs
