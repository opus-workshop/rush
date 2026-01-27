#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod arithmetic;
mod benchmark;
mod builtins;
mod completion;
mod context;
mod correction;
mod executor;
#[cfg(feature = "git-builtins")]
mod git;
mod glob_expansion;
mod history;
mod jobs;
mod lexer;
mod output;
mod parser;
mod progress;
mod runtime;
mod signal;
mod terminal;
mod undo;

use anyhow::Result;
use completion::Completer;
use executor::Executor;
use lexer::Lexer;
use parser::Parser;
use reedline::{Prompt, PromptHistorySearch, PromptHistorySearchStatus, Reedline, Signal};
use signal::SignalHandler;
use std::borrow::Cow;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::sync::{Arc, RwLock};
use nix::unistd::{setpgid, getpid};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    // Fast path: detect -c flag early and skip all expensive initialization.
    // This avoids: process group setup, signal handler thread, daemon probe,
    // init_environment_variables, and whoami calls — saving ~5-8ms.
    {
        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "-c" if i + 1 < args.len() => {
                    fast_execute_c(&args[i + 1]);
                    // fast_execute_c never returns (calls process::exit)
                }
                "--benchmark" if i + 1 < args.len() => {
                    // Handle benchmark mode
                    let mode = match args[i + 1].as_str() {
                        "quick" => benchmark::BenchmarkMode::Quick,
                        "full" => benchmark::BenchmarkMode::Full,
                        "compare" => benchmark::BenchmarkMode::Compare,
                        _ => {
                            eprintln!("Invalid benchmark mode: {}. Use 'quick', 'full', or 'compare'", args[i + 1]);
                            std::process::exit(1);
                        }
                    };
                    if let Err(e) = benchmark::run_benchmark(mode) {
                        eprintln!("Benchmark error: {}", e);
                        std::process::exit(1);
                    }
                    std::process::exit(0);
                }
                "--login" | "-l" | "--no-rc" | "--norc" => { i += 1; }
                _ => { i += 1; }
            }
        }
    }

    // Full initialization for interactive / script modes
    // Put the shell in its own process group for proper job control
    let shell_pid = getpid();
    if let Err(e) = setpgid(shell_pid, shell_pid) {
        // Non-fatal warning - continue anyway
        eprintln!("Warning: Failed to set shell process group: {}", e);
    }

    // Setup signal handlers early
    let signal_handler = SignalHandler::new();
    if let Err(e) = signal_handler.setup() {
        eprintln!("Warning: Failed to setup signal handlers: {}", e);
    }

    // Parse flags
    let mut is_login_shell = false;
    let mut skip_rc = false;
    let mut filtered_args = Vec::new();

    // Check if invoked as login shell (argv[0] starts with -)
    if let Some(arg0) = args.first() {
        if arg0.starts_with('-') || arg0.ends_with("/-rush") {
            is_login_shell = true;
        }
    }

    // Parse command-line flags
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--login" | "-l" => {
                is_login_shell = true;
                i += 1;
            }
            "--no-rc" | "--norc" => {
                skip_rc = true;
                i += 1;
            }
            _ => {
                filtered_args.push(args[i].clone());
                i += 1;
            }
        }
    }

    // Show help for invalid usage
    if !filtered_args.is_empty() && (filtered_args[0] == "-h" || filtered_args[0] == "--help") {
        print_help();
        return Ok(());
    }

    // Check if a script file is provided
    if !filtered_args.is_empty() && !filtered_args[0].starts_with('-') {
        let script_path = &filtered_args[0];
        let script_args = filtered_args[1..].to_vec();
        return run_script(script_path, script_args, signal_handler);
    }

    // Run interactive mode (possibly as login shell)
    run_interactive_with_init(signal_handler, is_login_shell, skip_rc)
}

fn run_script(
    script_path: &str,
    script_args: Vec<String>,
    signal_handler: SignalHandler,
) -> Result<()> {
    // Initialize environment variables
    init_environment_variables()?;
    
    // Read the script file
    let script_content = fs::read_to_string(script_path)
        .map_err(|e| anyhow::anyhow!("Failed to read script '{}': {}", script_path, e))?;

    let mut executor = Executor::new_with_signal_handler(signal_handler.clone());

    // Set runtime variables from environment
    init_runtime_variables(executor.runtime_mut());

    // Set up positional parameters ($1, $2, etc.) and $#, $@, $*
    executor.runtime_mut().set_positional_params(script_args.clone());

    // Set $0 to script name
    executor
        .runtime_mut()
        .set_variable("0".to_string(), script_path.to_string());

    // Execute the script line by line
    let mut last_exit_code = 0;
    for (line_num, line) in script_content.lines().enumerate() {
        // Check for signals
        if signal_handler.should_shutdown() {
            eprintln!("\nScript interrupted by signal");
            std::process::exit(signal_handler.exit_code());
        }

        // Check for SIGCHLD and reap any zombie processes
        if signal_handler.sigchld_received() {
            executor.runtime_mut().job_manager().reap_zombies();
            signal_handler.clear_sigchld();
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
                let stdout_text = result.stdout();
                if !stdout_text.is_empty() {
                    print!("{}", stdout_text);
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
    // Try to use daemon if available
    if let Ok(mut client) = rush::daemon::DaemonClient::new() {
        if client.is_daemon_running() {
            // Use daemon for execution
            let args = vec!["-c".to_string(), command.to_string()];
            match client.execute_command(&args) {
                Ok(exit_code) => {
                    std::process::exit(exit_code);
                }
                Err(e) => {
                    eprintln!("Daemon error: {}, falling back to direct execution", e);
                    // Fall through to direct execution
                }
            }
        }
    }

    // Fall back to direct execution
    // Initialize environment variables
    init_environment_variables()?;

    let mut executor = Executor::new_with_signal_handler(signal_handler.clone());

    // Set runtime variables from environment
    init_runtime_variables(executor.runtime_mut());

    match execute_line(command, &mut executor) {
        Ok(result) => {
            let stdout_text = result.stdout();
            if !stdout_text.is_empty() {
                print!("{}", stdout_text);
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

fn run_interactive_with_init(
    signal_handler: SignalHandler,
    is_login: bool,
    skip_rc: bool,
) -> Result<()> {
    // Initialize environment variables
    init_environment_variables()?;

    // Create executor early so we can source files
    let mut executor = Executor::new_with_signal_handler(signal_handler.clone());

    // Set runtime variables from environment
    init_runtime_variables(executor.runtime_mut());

    // Source profile files based on login shell and flags
    if is_login && !skip_rc {
        // Login shell: source ~/.rush_profile
        if let Some(home) = dirs::home_dir() {
            let profile = home.join(".rush_profile");
            if let Err(e) = executor.source_file(&profile) {
                eprintln!("Warning: Error sourcing ~/.rush_profile: {}", e);
            }
        }
    }

    // Interactive shell: source ~/.rushrc (unless --no-rc)
    if !skip_rc {
        if let Some(home) = dirs::home_dir() {
            let rushrc = home.join(".rushrc");
            if let Err(e) = executor.source_file(&rushrc) {
                eprintln!("Warning: Error sourcing ~/.rushrc: {}", e);
            }
        }
    }

    // Now run interactive mode
    if atty::is(atty::Stream::Stdin) {
        run_interactive_with_reedline(signal_handler)
    } else {
        run_non_interactive(signal_handler)
    }
}

fn init_environment_variables() -> Result<()> {
    // Set $SHELL only if not already set (avoids expensive current_exe() readlink)
    if env::var("SHELL").is_err() {
        if let Ok(exe) = env::current_exe() {
            env::set_var("SHELL", exe);
        }
    }

    // Set $TERM if not already set
    if env::var("TERM").is_err() {
        env::set_var("TERM", "xterm-256color");
    }

    // Set $USER if not already set (avoids expensive whoami syscall)
    if env::var("USER").is_err() {
        if let Ok(user) = env::var("LOGNAME") {
            env::set_var("USER", user);
        } else if let Some(user) = whoami::username_os().to_str() {
            env::set_var("USER", user);
        }
    }

    // Set $HOME if not already set
    if env::var("HOME").is_err() {
        if let Some(home) = dirs::home_dir() {
            env::set_var("HOME", home);
        }
    }

    Ok(())
}

fn init_runtime_variables(runtime: &mut runtime::Runtime) {
    // Set runtime variables from environment
    if let Ok(shell) = env::var("SHELL") {
        runtime.set_variable("SHELL".to_string(), shell);
    }
    if let Ok(term) = env::var("TERM") {
        runtime.set_variable("TERM".to_string(), term);
    }
    if let Ok(user) = env::var("USER") {
        runtime.set_variable("USER".to_string(), user);
    }
    if let Ok(home) = env::var("HOME") {
        runtime.set_variable("HOME".to_string(), home);
    }

    // Set PATH from environment (required for command execution)
    if let Ok(path) = env::var("PATH") {
        runtime.set_variable("PATH".to_string(), path);
    }

    // Set PWD to current working directory
    if let Ok(pwd) = env::current_dir() {
        runtime.set_variable("PWD".to_string(), pwd.to_string_lossy().to_string());
    }

    // Set PPID (parent process ID) - readonly on Unix systems
    #[cfg(unix)]
    {
        use std::os::unix::process::parent_id;
        runtime.set_variable("PPID".to_string(), parent_id().to_string());
        runtime.mark_readonly("PPID".to_string());
    }

    // Set SHLVL (shell nesting level)
    // Read from environment, default to 0, then increment by 1
    let shlvl = env::var("SHLVL")
        .ok()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0)
        + 1;
    runtime.set_variable("SHLVL".to_string(), shlvl.to_string());
    // Also update environment variable for child processes
    env::set_var("SHLVL", shlvl.to_string());
}

fn run_interactive_with_reedline(signal_handler: SignalHandler) -> Result<()> {
    println!("Rush v0.1.0 - A Modern Shell in Rust");
    println!("Type 'exit' to quit\n");

    let mut executor = Executor::new_with_signal_handler(signal_handler.clone());

    // Create completer with shared builtins and runtime
    let builtins = Arc::new(builtins::Builtins::new());
    let runtime = Arc::new(RwLock::new(runtime::Runtime::new()));
    let completer = Box::new(Completer::new(builtins.clone(), runtime.clone()));

    let mut line_editor = Reedline::create().with_completer(completer);
    let prompt = RushPrompt::new();

    loop {
        // Check for signals before reading next line
        if signal_handler.should_shutdown() {
            println!("\nExiting due to signal...");
            std::process::exit(signal_handler.exit_code());
        }

        // Check for SIGCHLD and reap any zombie processes
        if signal_handler.sigchld_received() {
            executor.runtime_mut().job_manager().reap_zombies();
            signal_handler.clear_sigchld();
        }

        // Update job statuses and cleanup completed jobs
        executor.runtime_mut().job_manager().update_all_jobs();

        // Print notifications for completed jobs
        let jobs = executor.runtime_mut().job_manager().list_jobs();
        for job in jobs {
            if job.status == jobs::JobStatus::Done {
                println!("[{}] Done\t\t{}", job.id, job.command);
            } else if job.status == jobs::JobStatus::Terminated {
                println!("[{}] Terminated\t{}", job.id, job.command);
            }
        }

        // Cleanup completed/terminated jobs
        executor.runtime_mut().job_manager().cleanup_jobs();

        let sig = line_editor.read_line(&prompt);

        match sig {
            Ok(Signal::Success(buffer)) => {
                let line = buffer.trim();

                if line.is_empty() {
                    continue;
                }

                match execute_line(line, &mut executor) {
                    Ok(result) => {
                        let stdout_text = result.stdout();
                        if !stdout_text.is_empty() {
                            print!("{}", stdout_text);
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

        // Check for SIGCHLD and reap any zombie processes
        if signal_handler.sigchld_received() {
            executor.runtime_mut().job_manager().reap_zombies();
            signal_handler.clear_sigchld();
        }

        let line = line?;
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        match execute_line(line, &mut executor) {
            Ok(result) => {
                let stdout_text = result.stdout();
                if !stdout_text.is_empty() {
                    print!("{}", stdout_text);
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
    println!("  rush --login        Start as login shell (sources ~/.rush_profile)");
    println!("  rush --no-rc        Skip sourcing config files");
    println!("  rush <script.rush>  Execute a Rush script file");
    println!("  rush -c <command>   Execute command and exit");
    println!("  rush --benchmark <mode> Run benchmarks (quick, full, compare)");
    println!("  rush -h, --help     Show this help message");
    println!();
    println!("Examples:");
    println!("  rush script.rush");
    println!("  rush script.rush arg1 arg2");
    println!("  rush -c \"echo hello\"");
    println!("  rush -c \"ls -la\"");
    println!("  rush -c \"cat file.txt | grep pattern\"");
    println!("  rush --login        # Start login shell");
    println!("  rush --benchmark quick   # Run quick benchmark (5-second smoke test)");
    println!("  rush --benchmark full    # Run comprehensive benchmark suite");
    println!();
    println!("Config Files:");
    println!("  ~/.rush_profile     Sourced on login shells");
    println!("  ~/.rushrc           Sourced on interactive shells");
}

fn execute_line(line: &str, executor: &mut Executor) -> Result<executor::ExecutionResult> {
    // Tokenize
    let tokens = Lexer::tokenize(line)?;

    // Parse
    let mut parser = Parser::new(tokens);
    let statements = parser.parse()?;

    // Execute — catch ExitSignal at top level so `exit` terminates the shell
    match executor.execute(statements) {
        Ok(result) => Ok(result),
        Err(e) => {
            if let Some(exit_signal) = e.downcast_ref::<builtins::exit_builtin::ExitSignal>() {
                std::process::exit(exit_signal.exit_code);
            }
            Err(e)
        }
    }
}

/// Fast path for `rush -c "command"` execution.
///
/// Skips all expensive initialization:
/// - NO daemon client probe (saves 2-4ms from UnixStream::connect)
/// - NO signal handler thread spawn (saves 0.5-1ms)
/// - NO process group setup via setpgid (saves 0.2-0.5ms)
/// - NO init_environment_variables (saves 0.3-0.5ms from whoami, current_exe)
///
/// This function never returns — it always calls std::process::exit.
fn fast_execute_c(cmd: &str) -> ! {
    // Reset SIGPIPE to default so piped commands work correctly.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    let tokens = match Lexer::tokenize(cmd) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("rush: {}", e);
            std::process::exit(2);
        }
    };

    let mut parser = Parser::new(tokens);
    let statements = match parser.parse() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("rush: {}", e);
            std::process::exit(2);
        }
    };

    let mut executor = Executor::new();

    // Minimal runtime init: just PATH and PWD so commands can be found
    if let Ok(path) = env::var("PATH") {
        executor.runtime_mut().set_variable("PATH".to_string(), path);
    }
    if let Ok(pwd) = env::current_dir() {
        executor
            .runtime_mut()
            .set_variable("PWD".to_string(), pwd.to_string_lossy().to_string());
    }
    if let Ok(home) = env::var("HOME") {
        executor
            .runtime_mut()
            .set_variable("HOME".to_string(), home);
    }

    match executor.execute(statements) {
        Ok(result) => {
            let stdout_text = result.stdout();
            if !stdout_text.is_empty() {
                print!("{}", stdout_text);
            }
            if !result.stderr.is_empty() {
                eprint!("{}", result.stderr);
            }
            std::process::exit(result.exit_code);
        }
        Err(e) => {
            if let Some(exit_signal) = e.downcast_ref::<builtins::exit_builtin::ExitSignal>() {
                std::process::exit(exit_signal.exit_code);
            }
            eprintln!("rush: {}", e);
            std::process::exit(1);
        }
    }
}

fn execute_line_with_context(
    line: &str,
    executor: &mut Executor,
    _script_path: &str,
    _line_num: usize,
) -> Result<executor::ExecutionResult> {
    execute_line(line, executor).map_err(|e| anyhow::anyhow!("{}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_echo() {
        let mut executor = Executor::new();
        let result = execute_line("echo hello", &mut executor).unwrap();
        assert_eq!(result.stdout(), "hello\n");
    }

    #[test]
    fn test_execute_pwd() {
        let mut executor = Executor::new();
        let result = execute_line("pwd", &mut executor).unwrap();
        assert!(!result.stdout().is_empty());
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
        assert_eq!(result.unwrap().stdout(), "test\n");
    }
}
