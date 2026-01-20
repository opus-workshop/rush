mod lexer;
mod parser;
mod executor;
mod runtime;
mod builtins;
mod completion;
mod history;
mod context;
mod output;
mod git;
mod undo;
mod correction;
mod progress;
mod signal;

use completion::Completer;
use executor::Executor;
use lexer::Lexer;
use parser::Parser;
use signal::SignalHandler;
use reedline::{Prompt, PromptHistorySearch, PromptHistorySearchStatus, Reedline, Signal};
use anyhow::Result;
use std::sync::{Arc, RwLock};
use std::env;
use std::fs;
use std::borrow::Cow;
use std::io::{BufRead, BufReader};

fn main() -> Result<()> {
    // Setup signal handlers early
    let signal_handler = SignalHandler::new();
    if let Err(e) = signal_handler.setup() {
        eprintln!("Warning: Failed to setup signal handlers: {}", e);
    }

    let args: Vec<String> = env::args().collect();

    // Check for -c flag for non-interactive command execution
    if args.len() >= 3 && args[1] == "-c" {
        return run_command(&args[2], signal_handler);
    }

    // Show help for invalid usage
    if args.len() > 1 && (args[1] == "-h" || args[1] == "--help") {
        print_help();
        return Ok(());
    }

    // Check if a script file is provided
    if args.len() >= 2 && !args[1].starts_with('-') {
        let script_path = &args[1];
        let script_args = args[2..].to_vec();
        return run_script(script_path, script_args, signal_handler);
    }

    // Run interactive mode
    run_interactive(signal_handler)
}

fn run_script(script_path: &str, script_args: Vec<String>, signal_handler: SignalHandler) -> Result<()> {
    // Read the script file
    let script_content = fs::read_to_string(script_path)
        .map_err(|e| anyhow::anyhow!("Failed to read script '{}': {}", script_path, e))?;

    let mut executor = Executor::new_with_signal_handler(signal_handler.clone());

    // Set up script arguments as $1, $2, etc.
    for (i, arg) in script_args.iter().enumerate() {
        executor.runtime_mut().set_variable((i + 1).to_string(), arg.clone());
    }

    // Set $0 to script name
    executor.runtime_mut().set_variable("0".to_string(), script_path.to_string());

    // Execute the script line by line
    let mut last_exit_code = 0;
    for (line_num, line) in script_content.lines().enumerate() {
        // Check for signals
        if signal_handler.should_shutdown() {
            eprintln!("\nScript interrupted by signal");
            std::process::exit(signal_handler.exit_code());
        }

        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Skip shebang line
        if line_num == 0 && line.starts_with("#!") {
            continue;
        }

        match execute_line_with_context(line, &mut executor, script_path, line_num + 1) {
            Ok(result) => {
                if !result.stdout.is_empty() {
                    print!("{}", result.stdout);
                }
                if !result.stderr.is_empty() {
                    eprint!("{}", result.stderr);
                }
                last_exit_code = result.exit_code;
            }
            Err(e) => {
                eprintln!("{}:{}: Error: {}", script_path, line_num + 1, e);
                std::process::exit(1);
            }
        }
    }

    std::process::exit(last_exit_code);
}

fn run_command(command: &str, signal_handler: SignalHandler) -> Result<()> {
    let mut executor = Executor::new_with_signal_handler(signal_handler.clone());

    match execute_line(command, &mut executor) {
        Ok(result) => {
            if !result.stdout.is_empty() {
                print!("{}", result.stdout);
            }
            if !result.stderr.is_empty() {
                eprint!("{}", result.stderr);
            }

            // Check if interrupted by signal
            if signal_handler.should_shutdown() {
                std::process::exit(signal_handler.exit_code());
            }

            // Exit with the command's exit code
            std::process::exit(result.exit_code);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

/// Custom prompt that displays current directory with home directory shortening
struct RushPrompt;

impl RushPrompt {
    fn new() -> Self {
        Self
    }

    fn get_prompt_indicator(&self) -> String {
        let cwd = if let Ok(cwd) = env::current_dir() {
            // Shorten home directory to ~
            if let Some(home) = dirs::home_dir() {
                if let Ok(suffix) = cwd.strip_prefix(&home) {
                    if suffix.as_os_str().is_empty() {
                        "~".to_string()
                    } else {
                        format!("~/{}", suffix.display())
                    }
                } else {
                    cwd.display().to_string()
                }
            } else {
                cwd.display().to_string()
            }
        } else {
            "?".to_string()
        };

        format!("{}> ", cwd)
    }
}

impl Prompt for RushPrompt {
    fn render_prompt_left(&self) -> Cow<str> {
        Cow::Owned(self.get_prompt_indicator())
    }

    fn render_prompt_right(&self) -> Cow<str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _prompt_mode: reedline::PromptEditMode) -> Cow<str> {
        Cow::Borrowed("")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<str> {
        Cow::Borrowed("> ")
    }

    fn render_prompt_history_search_indicator(
        &self,
        history_search: PromptHistorySearch,
    ) -> Cow<str> {
        let prefix = match history_search.status {
            PromptHistorySearchStatus::Passing => "",
            PromptHistorySearchStatus::Failing => "failing ",
        };

        Cow::Owned(format!(
            "({}reverse-search: {}) ",
            prefix, history_search.term
        ))
    }
}

fn run_interactive(signal_handler: SignalHandler) -> Result<()> {
    if atty::is(atty::Stream::Stdin) {
        run_interactive_with_reedline(signal_handler)
    } else {
        run_non_interactive(signal_handler)
    }
}

fn run_interactive_with_reedline(signal_handler: SignalHandler) -> Result<()> {
    println!("Rush v0.1.0 - A Modern Shell in Rust");
    println!("Type 'exit' to quit\n");

    let mut executor = Executor::new_with_signal_handler(signal_handler.clone());

    // Create completer with shared builtins and runtime
    let builtins = Arc::new(builtins::Builtins::new());
    let runtime = Arc::new(RwLock::new(runtime::Runtime::new()));
    let completer = Box::new(Completer::new(builtins.clone(), runtime.clone()));

    let mut line_editor = Reedline::create()
        .with_completer(completer);
    let prompt = RushPrompt::new();

    loop {
        // Check for signals before reading next line
        if signal_handler.should_shutdown() {
            println!("\nExiting due to signal...");
            std::process::exit(signal_handler.exit_code());
        }

        let sig = line_editor.read_line(&prompt);

        match sig {
            Ok(Signal::Success(buffer)) => {
                let line = buffer.trim();

                if line.is_empty() {
                    continue;
                }

                match execute_line(line, &mut executor) {
                    Ok(result) => {
                        if !result.stdout.is_empty() {
                            print!("{}", result.stdout);
                        }
                        if !result.stderr.is_empty() {
                            eprintln!("{}", result.stderr);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                    }
                }
            }
            Ok(Signal::CtrlC) => {
                // Reedline handles Ctrl-C in interactive mode
                // Reset signal handler state
                signal_handler.reset();
                continue;
            }
            Ok(Signal::CtrlD) => {
                break;
            }
            Err(e) => {
                eprintln!("Error reading line: {}", e);
                break;
            }
        }
    }

    Ok(())
}

fn run_non_interactive(signal_handler: SignalHandler) -> Result<()> {
    let mut executor = Executor::new_with_signal_handler(signal_handler.clone());
    let stdin = std::io::stdin();
    let reader = BufReader::new(stdin.lock());

    for line in reader.lines() {
        // Check for signals
        if signal_handler.should_shutdown() {
            eprintln!("\nInterrupted by signal");
            std::process::exit(signal_handler.exit_code());
        }

        let line = line?;
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        match execute_line(line, &mut executor) {
            Ok(result) => {
                if !result.stdout.is_empty() {
                    print!("{}", result.stdout);
                }
                if !result.stderr.is_empty() {
                    eprint!("{}", result.stderr);
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                // Continue on error in non-interactive mode
            }
        }
    }

    Ok(())
}

fn print_help() {
    println!("Rush v0.1.0 - A Modern Shell in Rust");
    println!();
    println!("Usage:");
    println!("  rush                Start interactive shell");
    println!("  rush <script.rush>  Execute a Rush script file");
    println!("  rush -c <command>   Execute command and exit");
    println!("  rush -h, --help     Show this help message");
    println!();
    println!("Examples:");
    println!("  rush script.rush");
    println!("  rush script.rush arg1 arg2");
    println!("  rush -c \"echo hello\"");
    println!("  rush -c \"ls -la\"");
    println!("  rush -c \"cat file.txt | grep pattern\"");
}

fn execute_line(line: &str, executor: &mut Executor) -> Result<executor::ExecutionResult> {
    // Tokenize
    let tokens = Lexer::tokenize(line)?;

    // Parse
    let mut parser = Parser::new(tokens);
    let statements = parser.parse()?;

    // Execute
    executor.execute(statements)
}

fn execute_line_with_context(
    line: &str,
    executor: &mut Executor,
    script_path: &str,
    line_num: usize,
) -> Result<executor::ExecutionResult> {
    execute_line(line, executor).map_err(|e| {
        anyhow::anyhow!("{}", e)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_echo() {
        let mut executor = Executor::new();
        let result = execute_line("echo hello", &mut executor).unwrap();
        assert_eq!(result.stdout, "hello\n");
    }

    #[test]
    fn test_execute_pwd() {
        let mut executor = Executor::new();
        let result = execute_line("pwd", &mut executor).unwrap();
        assert!(!result.stdout.is_empty());
    }

    #[test]
    fn test_script_arguments() {
        use std::fs;
        use std::io::Write;
        
        // Create a temporary script
        let script_path = "/tmp/rush_test_args.rush";
        let mut file = fs::File::create(script_path).unwrap();
        writeln!(file, "#!/usr/bin/env rush").unwrap();
        writeln!(file, "echo $1").unwrap();
        writeln!(file, "echo $2").unwrap();
        
        // Test would go here, but requires running the binary
        // This is more of an integration test
        
        // Cleanup
        fs::remove_file(script_path).ok();
    }

    #[test]
    fn test_execute_line_with_context() {
        let mut executor = Executor::new();
        let result = execute_line_with_context("echo test", &mut executor, "test.rush", 1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().stdout, "test\n");
    }
}
