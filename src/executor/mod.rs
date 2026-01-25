pub mod pipeline;
pub mod value;

// Re-export Value type for convenience
pub use value::Value;

use crate::builtins::Builtins;
use crate::correction::Corrector;
use crate::glob_expansion;
use crate::parser::ast::*;
use crate::runtime::Runtime;
use crate::progress::ProgressIndicator;
use crate::signal::SignalHandler;
use crate::terminal::TerminalControl;
use anyhow::{anyhow, Result};
use std::process::Command as StdCommand;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use nix::unistd::{setpgid, getpid, Pid};
use std::os::unix::process::CommandExt;

pub struct Executor {
    runtime: Runtime,
    builtins: Builtins,
    corrector: Corrector,
    signal_handler: Option<SignalHandler>,
    terminal_control: TerminalControl,
    show_progress: bool,
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

impl Executor {
    pub fn new() -> Self {
        Self {
            runtime: Runtime::new(),
            builtins: Builtins::new(),
            corrector: Corrector::new(),
            signal_handler: None,
            show_progress: true, // Default to true for CLI usage
            terminal_control: TerminalControl::new(),
        }
    }

    /// Create executor without progress indicators (for embedded/TUI usage)
    pub fn new_embedded() -> Self {
        Self {
            runtime: Runtime::new(),
            builtins: Builtins::new(),
            corrector: Corrector::new(),
            signal_handler: None,
            terminal_control: TerminalControl::new(),
            show_progress: false,
        }
    }

    pub fn new_with_signal_handler(signal_handler: SignalHandler) -> Self {
        Self {
            runtime: Runtime::new(),
            builtins: Builtins::new(),
            corrector: Corrector::new(),
            signal_handler: Some(signal_handler),
            terminal_control: TerminalControl::new(),
            show_progress: true,
        }
    }

    pub fn execute(&mut self, statements: Vec<Statement>) -> Result<ExecutionResult> {
        let mut accumulated_stdout = String::new();
        let mut accumulated_stderr = String::new();
        let mut last_exit_code = 0;

        for statement in statements {
            // Check for signals before each statement
            if let Some(handler) = &self.signal_handler {
                if handler.should_shutdown() {
                    // Execute signal trap if set
                    let signal_num = handler.signal_number();
                    let trap_signal = match signal_num {
                        2 => Some(crate::builtins::trap::TrapSignal::Int),  // SIGINT
                        15 => Some(crate::builtins::trap::TrapSignal::Term), // SIGTERM
                        1 => Some(crate::builtins::trap::TrapSignal::Hup),   // SIGHUP
                        _ => None,
                    };
                    
                    if let Some(sig) = trap_signal {
                        let _ = self.execute_trap(sig);
                    }
                    
                    return Err(anyhow!("Interrupted by signal"));
                }
            }

            let result = self.execute_statement(statement)?;
            accumulated_stdout.push_str(&result.stdout());
            accumulated_stderr.push_str(&result.stderr);
            last_exit_code = result.exit_code;
            
            // Update $? after each statement
            self.runtime.set_last_exit_code(last_exit_code);

            // Execute ERR trap if command failed
            if last_exit_code != 0 {
                let _ = self.execute_trap(crate::builtins::trap::TrapSignal::Err);
            }

            // Check errexit option: exit if command failed
            if self.runtime.options.errexit && last_exit_code != 0 {
                return Ok(ExecutionResult {
                    output: Output::Text(accumulated_stdout),
                    stderr: accumulated_stderr,
                    exit_code: last_exit_code,
                    error: None,
                });
            }
        }

        Ok(ExecutionResult {
            output: Output::Text(accumulated_stdout),
            stderr: accumulated_stderr,
            exit_code: last_exit_code,
            error: None,
        })
    }

    pub fn execute_statement(&mut self, statement: Statement) -> Result<ExecutionResult> {
        match statement {
            Statement::Command(cmd) => self.execute_command(cmd),
            Statement::Pipeline(pipeline) => self.execute_pipeline(pipeline),
            Statement::ParallelExecution(parallel) => self.execute_parallel(parallel),
            Statement::Assignment(assignment) => self.execute_assignment(assignment),
            Statement::FunctionDef(func) => self.execute_function_def(func),
            Statement::IfStatement(if_stmt) => self.execute_if_statement(if_stmt),
            Statement::ForLoop(for_loop) => self.execute_for_loop(for_loop),
            Statement::WhileLoop(while_loop) => self.execute_while_loop(while_loop),
            Statement::UntilLoop(until_loop) => self.execute_until_loop(until_loop),
            Statement::MatchExpression(match_expr) => self.execute_match(match_expr),
            Statement::CaseStatement(case_stmt) => self.execute_case(case_stmt),
            Statement::ConditionalAnd(cond_and) => self.execute_conditional_and(cond_and),
            Statement::ConditionalOr(cond_or) => self.execute_conditional_or(cond_or),
            Statement::Subshell(statements) => self.execute_subshell(statements),
            Statement::BackgroundCommand(cmd) => self.execute_background(*cmd),
        }
    }

    fn execute_command(&mut self, command: Command) -> Result<ExecutionResult> {
        // Print command if xtrace is enabled
        if self.runtime.options.xtrace {
            let args_str = command.args.iter()
                .map(|arg| match arg {
                    Argument::Literal(s) | Argument::Variable(s) | Argument::BracedVariable(s) | 
                    Argument::CommandSubstitution(s) | Argument::Flag(s) | Argument::Path(s) => s.clone(),
                })
                .collect::<Vec<_>>()
                .join(" ");
            if args_str.is_empty() {
                eprintln!("+ {}", command.name);
            } else {
                eprintln!("+ {} {}", command.name, args_str);
            }
        }

        // Check if it's an alias and expand it
        let (command_name, command_args) = if let Some(alias_value) = self.runtime.get_alias(&command.name) {
            // Split the alias value into command and args
            let parts: Vec<&str> = alias_value.split_whitespace().collect();
            if parts.is_empty() {
                return Err(anyhow!("Empty alias expansion for '{}'", command.name));
            }
            
            // First part is the new command name
            let new_name = parts[0].to_string();
            
            // Remaining parts become additional arguments (prepended to original args)
            let mut new_args = Vec::new();
            for part in parts.iter().skip(1) {
                new_args.push(Argument::Literal(part.to_string()));
            }
            new_args.extend(command.args.clone());
            
            (new_name, new_args)
        } else {
            (command.name.clone(), command.args.clone())
        };

        // Check if it's a user-defined function first
        if self.runtime.get_function(&command_name).is_some() {
            let args = self.expand_and_resolve_arguments(&command_args)?;
            // Track last argument for $_
            if let Some(last) = args.last() {
                self.runtime.set_last_arg(last.clone());
            }
            return self.execute_user_function(&command_name, args);
        }

        // Check if it's a builtin command
        if self.builtins.is_builtin(&command_name) {
            let args = self.expand_and_resolve_arguments(&command_args)?;
            // Track last argument for $_
            if let Some(last) = args.last() {
                self.runtime.set_last_arg(last.clone());
            }
            let mut result = self.builtins.execute(&command_name, args, &mut self.runtime)?;

            // Handle redirects for builtins
            if !command.redirects.is_empty() {
                result = self.apply_redirects(result, &command.redirects)?;
            }

            self.runtime.set_last_exit_code(result.exit_code);
            return Ok(result);
        }

        // Execute external command with the potentially expanded command name and args
        let mut expanded_command = command;
        expanded_command.name = command_name;
        expanded_command.args = command_args;
        let result = self.execute_external_command(expanded_command)?;
        self.runtime.set_last_exit_code(result.exit_code);
        Ok(result)
    }

    fn apply_redirects(&self, mut result: ExecutionResult, redirects: &[Redirect]) -> Result<ExecutionResult> {
        use std::fs::{File, OpenOptions};
        use std::io::Write;
        use std::path::Path;
        
        // Helper to resolve paths relative to cwd
        let resolve_path = |target: &str| -> std::path::PathBuf {
            let path = Path::new(target);
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                self.runtime.get_cwd().join(target)
            }
        };
        
        for redirect in redirects {
            match &redirect.kind {
                RedirectKind::Stdout => {
                    if let Some(target) = &redirect.target {
                        let resolved = resolve_path(target);
                        let mut file = File::create(&resolved)
                            .map_err(|e| anyhow!("Failed to create '{}': {}", target, e))?;
                        file.write_all(result.stdout().as_bytes())
                            .map_err(|e| anyhow!("Failed to write to '{}': {}", target, e))?;
                        result.clear_stdout(); // Clear stdout as it's been redirected
                    }
                }
                RedirectKind::StdoutAppend => {
                    if let Some(target) = &redirect.target {
                        let resolved = resolve_path(target);
                        let mut file = OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&resolved)
                            .map_err(|e| anyhow!("Failed to open '{}': {}", target, e))?;
                        file.write_all(result.stdout().as_bytes())
                            .map_err(|e| anyhow!("Failed to write to '{}': {}", target, e))?;
                        result.clear_stdout(); // Clear stdout as it's been redirected
                    }
                }
                RedirectKind::Stdin => {
                    // Stdin redirect doesn't make sense for builtins that have already executed
                    // This would need to be handled before execution
                }
                RedirectKind::Stderr => {
                    if let Some(target) = &redirect.target {
                        let resolved = resolve_path(target);
                        let mut file = File::create(&resolved)
                            .map_err(|e| anyhow!("Failed to create '{}': {}", target, e))?;
                        file.write_all(result.stderr.as_bytes())
                            .map_err(|e| anyhow!("Failed to write to '{}': {}", target, e))?;
                        result.stderr.clear(); // Clear stderr as it's been redirected
                    }
                }
                RedirectKind::StderrToStdout => {
                    // Merge stderr into stdout
                    result.push_stdout(&result.stderr.clone());
                    result.stderr.clear();
                }
                RedirectKind::Both => {
                    if let Some(target) = &redirect.target {
                        let resolved = resolve_path(target);
                        let mut file = File::create(&resolved)
                            .map_err(|e| anyhow!("Failed to create '{}': {}", target, e))?;
                        // Clone file descriptor for both stdout and stderr
                        file.write_all(result.stdout().as_bytes())
                            .map_err(|e| anyhow!("Failed to write to '{}': {}", target, e))?;
                        file.write_all(result.stderr.as_bytes())
                            .map_err(|e| anyhow!("Failed to write to '{}': {}", target, e))?;
                        result.clear_stdout();
                        result.stderr.clear();
                    }
                }
            }
        }
        
        Ok(result)
    }

    fn execute_user_function(&mut self, name: &str, args: Vec<String>) -> Result<ExecutionResult> {
        // Get the function definition (we know it exists because we checked earlier)
        let func = self.runtime.get_function(name)
            .ok_or_else(|| anyhow!("Function '{}' not found", name))?
            .clone(); // Clone to avoid borrow issues

        // Check recursion depth
        self.runtime.push_call(name.to_string())
            .map_err(|e| anyhow!(e))?;

        // Create a new scope for the function
        self.runtime.push_scope();

        // Bind arguments to parameters
        for (i, param) in func.params.iter().enumerate() {
            let arg_value = args.get(i).cloned().unwrap_or_default();
            self.runtime.set_variable(param.name.clone(), arg_value);
        }

        // Set positional parameters ($1, $2, $#, $@, $*) for the function
        self.runtime.set_positional_params(args.clone());

        // Enter function context (allows return builtin)
        self.runtime.enter_function_context();

        // Execute function body
        let mut last_result = ExecutionResult::default();
        for statement in func.body {
            match self.execute_statement(statement) {
                Ok(stmt_result) => {
                    // Accumulate stdout from all statements
                    last_result.push_stdout(&stmt_result.stdout());
                    last_result.stderr.push_str(&stmt_result.stderr);
                    // Keep the last exit code
                    last_result.exit_code = stmt_result.exit_code;
                }
                Err(e) => {
                    // Check if this is a return signal
                    if let Some(return_signal) = e.downcast_ref::<crate::builtins::return_builtin::ReturnSignal>() {
                        // Early return from function
                        last_result.exit_code = return_signal.exit_code;
                        break;
                    } else {
                        // Some other error - propagate it
                        self.runtime.exit_function_context();
                        self.runtime.pop_scope();
                        self.runtime.pop_call();
                        return Err(e);
                    }
                }
            }
        }

        // Exit function context
        self.runtime.exit_function_context();

        // Clean up scope and call stack
        self.runtime.pop_scope();
        self.runtime.pop_call();

        Ok(last_result)
    }

    fn execute_external_command(&mut self, command: Command) -> Result<ExecutionResult> {
        let args = self.expand_and_resolve_arguments(&command.args)?;

        // Track last argument for $_
        if let Some(last) = args.last() {
            self.runtime.set_last_arg(last.clone());
        }

        // Set up command with redirects
        let mut cmd = StdCommand::new(&command.name);
        cmd.args(&args)
            .current_dir(self.runtime.get_cwd())
            .envs(self.runtime.get_env());

        // Handle redirections
        use std::fs::{File, OpenOptions};
        use std::process::Stdio;
        use std::path::Path;
        
        let mut stdout_redirect = false;
        let mut stderr_redirect = false;
        let mut stderr_to_stdout = false;
        let mut stdin_redirect = false;
        
        // Helper to resolve paths relative to cwd
        let resolve_path = |target: &str| -> std::path::PathBuf {
            let path = Path::new(target);
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                self.runtime.get_cwd().join(target)
            }
        };
        
        for redirect in &command.redirects {
            match &redirect.kind {
                RedirectKind::Stdout => {
                    if let Some(target) = &redirect.target {
                        let resolved = resolve_path(target);
                        let file = File::create(&resolved)
                            .map_err(|e| anyhow!("Failed to create '{}': {}", target, e))?;
                        cmd.stdout(Stdio::from(file));
                        stdout_redirect = true;
                    }
                }
                RedirectKind::StdoutAppend => {
                    if let Some(target) = &redirect.target {
                        let resolved = resolve_path(target);
                        let file = OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&resolved)
                            .map_err(|e| anyhow!("Failed to open '{}': {}", target, e))?;
                        cmd.stdout(Stdio::from(file));
                        stdout_redirect = true;
                    }
                }
                RedirectKind::Stdin => {
                    if let Some(target) = &redirect.target {
                        let resolved = resolve_path(target);
                        let file = File::open(&resolved)
                            .map_err(|e| anyhow!("Failed to open '{}': {}", target, e))?;
                        cmd.stdin(Stdio::from(file));
                        stdin_redirect = true;
                    }
                }
                RedirectKind::Stderr => {
                    if let Some(target) = &redirect.target {
                        let resolved = resolve_path(target);
                        let file = File::create(&resolved)
                            .map_err(|e| anyhow!("Failed to create '{}': {}", target, e))?;
                        cmd.stderr(Stdio::from(file));
                        stderr_redirect = true;
                    }
                }
                RedirectKind::StderrToStdout => {
                    // Redirect stderr to stdout
                    stderr_to_stdout = true;
                }
                RedirectKind::Both => {
                    if let Some(target) = &redirect.target {
                        let resolved = resolve_path(target);
                        let file = File::create(&resolved)
                            .map_err(|e| anyhow!("Failed to create '{}': {}", target, e))?;
                        // Clone file descriptor for both stdout and stderr
                        cmd.stdout(Stdio::from(file.try_clone()
                            .map_err(|e| anyhow!("Failed to clone file descriptor: {}", e))?));
                        cmd.stderr(Stdio::from(file));
                        stdout_redirect = true;
                        stderr_redirect = true;
                    }
                }
            }
        }
        
        // Set default stdin to inherit from parent if not redirected
        if !stdin_redirect {
            cmd.stdin(Stdio::inherit());
        }
        
        // For commands with no redirects, check if we should run in full interactive mode
        // This allows interactive programs (like editors, REPLs, claude) to work properly
        // NEVER inherit IO in embedded mode (TUI usage) - always pipe
        let should_inherit_io = self.show_progress && 
                                !stdout_redirect && !stderr_redirect && 
                                command.redirects.is_empty() &&
                                atty::is(atty::Stream::Stdout);
        
        // Set default piped outputs if not redirected
        if !stdout_redirect {
            if should_inherit_io {
                cmd.stdout(Stdio::inherit());
            } else {
                cmd.stdout(Stdio::piped());
            }
        }
        if !stderr_redirect && !stderr_to_stdout {
            if should_inherit_io {
                cmd.stderr(Stdio::inherit());
            } else {
                cmd.stderr(Stdio::piped());
            }
        } else if stderr_to_stdout && !stderr_redirect {
            // Redirect stderr to stdout for the process
            cmd.stderr(Stdio::piped());
        }

        // Use pre_exec to set the process group before the child executes
        // This is required for proper job control and signal handling
        unsafe {
            cmd.pre_exec(|| {
                // Put this process in its own process group (PGID = PID)
                let pid = getpid();
                setpgid(pid, pid).map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::Other, format!("setpgid failed: {}", e))
                })?;
                Ok(())
            });
        }

        // Spawn the command
        let mut child = cmd.spawn()
            .map_err(|e| {
                // If command not found, provide suggestions
                if e.kind() == std::io::ErrorKind::NotFound {
                    let builtin_names: Vec<String> = self.builtins.builtin_names();
                    let suggestions = self.corrector.suggest_command(&command.name, &builtin_names);
                    
                    let mut error_msg = format!("Command not found: '{}'", command.name);
                    
                    if !suggestions.is_empty() {
                        error_msg.push_str("\n\nDid you mean?");
                        for suggestion in suggestions.iter().take(3) {
                            let similarity = crate::correction::Corrector::similarity_percent(
                                suggestion.score,
                                &suggestion.text
                            );
                            error_msg.push_str(&format!(
                                "\n  {} ({}%, {})",
                                suggestion.text,
                                similarity,
                                suggestion.kind.label()
                            ));
                        }
                    }
                    
                    anyhow!(error_msg)
                } else {
                    anyhow!("Failed to execute '{}': {}", command.name, e)
                }
            })?;

        // Wait a bit to see if command completes quickly
        thread::sleep(Duration::from_millis(crate::progress::PROGRESS_THRESHOLD_MS));
        
        // Check if command is still running
        let progress = match child.try_wait() {
            Ok(Some(_)) => None, // Command already finished
            _ => {
                // Command still running, show progress indicator only if enabled
                if self.show_progress {
                    Some(ProgressIndicator::new(format!("Running {}", command.name)))
                } else {
                    None
                }
            }
        };

        // Wait for command to complete, checking for signals
        let (stdout_str, stderr_str, exit_code) = if should_inherit_io {
            // Interactive mode - IO is inherited, just wait for exit status
            loop {
                // Check for signals
                if let Some(handler) = &self.signal_handler {
                    if handler.should_shutdown() {
                        // Kill the child process
                        let _ = child.kill();
                        let _ = child.wait();
                        
                        // Stop progress indicator if it was started
                        if let Some(prog) = progress {
                            prog.stop();
                        }
                        
                        return Err(anyhow!("Command interrupted by signal"));
                    }
                }

                // Try to get the status
                match child.try_wait() {
                    Ok(Some(status)) => {
                        // Child finished
                        break (String::new(), String::new(), status.code().unwrap_or(1));
                    }
                    Ok(None) => {
                        // Still running, sleep briefly and check again
                        thread::sleep(Duration::from_millis(100));
                    }
                    Err(e) => {
                        return Err(anyhow!("Failed to check status for '{}': {}", command.name, e));
                    }
                }
            }
        } else {
            // Non-interactive mode - capture output
            let output = loop {
                // Check for signals
                if let Some(handler) = &self.signal_handler {
                    if handler.should_shutdown() {
                        // Kill the child process
                        let _ = child.kill();
                        let _ = child.wait();
                        
                        // Stop progress indicator if it was started (for interactive mode)
                        if let Some(prog) = progress {
                            prog.stop();
                        }
                        
                        return Err(anyhow!("Command interrupted by signal"));
                    }
                }

                // Try to get the output
                match child.try_wait() {
                    Ok(Some(_)) => {
                        // Child finished, get output
                        break child.wait_with_output()
                            .map_err(|e| anyhow!("Failed to wait for '{}': {}", command.name, e))?;
                    }
                    Ok(None) => {
                        // Still running, sleep briefly and check again
                        thread::sleep(Duration::from_millis(100));
                    }
                    Err(e) => {
                        return Err(anyhow!("Failed to check status for '{}': {}", command.name, e));
                    }
                }
            };

            // Stop progress indicator if it was started (for interactive mode)
            if let Some(prog) = progress {
                prog.stop();
            }

            // Handle stderr to stdout redirection in output
            let mut stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();
            
            if stderr_to_stdout && !stderr_str.is_empty() {
                stdout_str.push_str(&stderr_str);
            }

            (
                stdout_str,
                if stderr_to_stdout { String::new() } else { stderr_str },
                output.status.code().unwrap_or(1)
            )
        };

        Ok(ExecutionResult {
            output: Output::Text(stdout_str),
            stderr: stderr_str,
            exit_code,
            error: None,
        })
    }

    fn execute_pipeline(&mut self, pipeline: Pipeline) -> Result<ExecutionResult> {
        pipeline::execute_pipeline(pipeline, &mut self.runtime, &self.builtins)
    }

    fn execute_parallel(&mut self, parallel: ParallelExecution) -> Result<ExecutionResult> {
        if parallel.commands.is_empty() {
            return Ok(ExecutionResult::default());
        }

        // Clone necessary data for thread safety
        let builtins = Arc::new(self.builtins.clone());
        let corrector = Arc::new(self.corrector.clone());
        let runtime_snapshot = Arc::new(self.runtime.clone());

        let mut handles = Vec::new();

        for command in parallel.commands {
            let builtins = Arc::clone(&builtins);
            let corrector = Arc::clone(&corrector);
            let runtime_snapshot = Arc::clone(&runtime_snapshot);

            let handle = thread::spawn(move || {
                let result = if builtins.is_builtin(&command.name) {
                    // Execute builtin
                    let args = expand_and_resolve_arguments_static(&command.args, &runtime_snapshot)?;
                    
                    // We need a mutable runtime, but we can't safely share it across threads
                    // For now, create a temporary runtime for builtins in parallel execution
                    let mut temp_runtime = (*runtime_snapshot).clone();
                    builtins.execute(&command.name, args, &mut temp_runtime)
                } else {
                    // Execute external command
                    let args = expand_and_resolve_arguments_static(&command.args, &runtime_snapshot)?;

                    match StdCommand::new(&command.name)
                        .args(&args)
                        .current_dir(runtime_snapshot.get_cwd())
                        .envs(runtime_snapshot.get_env())
                        .output()
                    {
                        Ok(output) => Ok(ExecutionResult {
                            output: Output::Text(String::from_utf8_lossy(&output.stdout).to_string()),
                            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                            exit_code: output.status.code().unwrap_or(1),
                            error: None,
                        }),
                        Err(e) => {
                            if e.kind() == std::io::ErrorKind::NotFound {
                                let builtin_names: Vec<String> = builtins.builtin_names();
                                let suggestions = corrector.suggest_command(&command.name, &builtin_names);
                                
                                let mut error_msg = format!("Command not found: '{}'", command.name);
                                
                                if !suggestions.is_empty() {
                                    error_msg.push_str("\n\nDid you mean?");
                                    for suggestion in suggestions.iter().take(3) {
                                        let similarity = crate::correction::Corrector::similarity_percent(
                                            suggestion.score,
                                            &suggestion.text
                                        );
                                        error_msg.push_str(&format!(
                                            "\n  {} ({}%, {})",
                                            suggestion.text,
                                            similarity,
                                            suggestion.kind.label()
                                        ));
                                    }
                                }
                                
                                Err(anyhow!(error_msg))
                            } else {
                                Err(anyhow!("Failed to execute '{}': {}", command.name, e))
                            }
                        }
                    }
                };

                result
            });

            handles.push(handle);
        }

        // Join all threads and collect results
        let mut combined_stdout = String::new();
        let mut combined_stderr = String::new();
        let mut max_exit_code = 0;

        for handle in handles {
            match handle.join() {
                Ok(Ok(result)) => {
                    combined_stdout.push_str(&result.stdout());
                    combined_stderr.push_str(&result.stderr);
                    max_exit_code = max_exit_code.max(result.exit_code);
                }
                Ok(Err(e)) => {
                    combined_stderr.push_str(&format!("{}\n", e));
                    max_exit_code = max_exit_code.max(1);
                }
                Err(_) => {
                    combined_stderr.push_str("Thread panicked during parallel execution\n");
                    max_exit_code = max_exit_code.max(1);
                }
            }
        }

        Ok(ExecutionResult {
            output: Output::Text(combined_stdout),
            stderr: combined_stderr,
            exit_code: max_exit_code,
            error: None,
        })
    }

    fn execute_assignment(&mut self, assignment: Assignment) -> Result<ExecutionResult> {
        let value = self.evaluate_expression(assignment.value)?;
        self.runtime.set_variable_checked(assignment.name, value)?;
        Ok(ExecutionResult::default())
    }

    fn execute_function_def(&mut self, func: FunctionDef) -> Result<ExecutionResult> {
        self.runtime.define_function(func);
        Ok(ExecutionResult::default())
    }

    fn execute_if_statement(&mut self, if_stmt: IfStatement) -> Result<ExecutionResult> {
        let condition = self.evaluate_expression(if_stmt.condition)?;

        if self.is_truthy(&condition) {
            for statement in if_stmt.then_block {
                self.execute_statement(statement)?;
            }
        } else if let Some(else_block) = if_stmt.else_block {
            for statement in else_block {
                self.execute_statement(statement)?;
            }
        }

        Ok(ExecutionResult::default())
    }

    fn execute_for_loop(&mut self, for_loop: ForLoop) -> Result<ExecutionResult> {
        let iterable = self.evaluate_expression(for_loop.iterable)?;

        // For now, simple iteration over strings split by lines
        let items: Vec<String> = iterable.lines().map(|s| s.to_string()).collect();

        // Enter loop context for break/continue
        self.runtime.enter_loop();

        let mut accumulated_stdout = String::new();
        let mut accumulated_stderr = String::new();
        let mut last_exit_code = 0;

        let result = (|| -> Result<ExecutionResult> {
            for item in items {
                self.runtime
                    .set_variable(for_loop.variable.clone(), item);
                for statement in &for_loop.body {
                    match self.execute_statement(statement.clone()) {
                        Ok(result) => {
                            accumulated_stdout.push_str(&result.stdout());
                            accumulated_stderr.push_str(&result.stderr);
                            last_exit_code = result.exit_code;
                        }
                        Err(e) => {
                            // Check if this is a break signal
                            if let Some(break_signal) = e.downcast_ref::<crate::builtins::break_builtin::BreakSignal>() {
                                // First, add any accumulated output from the break signal itself
                                accumulated_stdout.push_str(&break_signal.accumulated_stdout);
                                accumulated_stderr.push_str(&break_signal.accumulated_stderr);

                                if break_signal.levels == 1 {
                                    // Break from this loop, return accumulated output
                                    return Ok(ExecutionResult {
                                        output: Output::Text(accumulated_stdout),
                                        stderr: accumulated_stderr,
                                        exit_code: last_exit_code,
                                        error: None,
                                    });
                                } else {
                                    // Propagate to outer loop with decreased level and accumulated output
                                    return Err(anyhow::Error::new(crate::builtins::break_builtin::BreakSignal {
                                        levels: break_signal.levels - 1,
                                        accumulated_stdout: accumulated_stdout.clone(),
                                        accumulated_stderr: accumulated_stderr.clone(),
                                    }));
                                }
                            }

                            // Check if this is a continue signal
                            if let Some(continue_signal) = e.downcast_ref::<crate::builtins::continue_builtin::ContinueSignal>() {
                                // First, add any accumulated output from the continue signal itself
                                accumulated_stdout.push_str(&continue_signal.accumulated_stdout);
                                accumulated_stderr.push_str(&continue_signal.accumulated_stderr);

                                if continue_signal.levels == 1 {
                                    // Continue in this loop - skip to next iteration
                                    break; // Break out of the statement loop, continue with next item
                                } else {
                                    // Propagate to outer loop with decreased level and accumulated output
                                    return Err(anyhow::Error::new(crate::builtins::continue_builtin::ContinueSignal {
                                        levels: continue_signal.levels - 1,
                                        accumulated_stdout: accumulated_stdout.clone(),
                                        accumulated_stderr: accumulated_stderr.clone(),
                                    }));
                                }
                            }

                            // Not a break or continue signal, propagate the error
                            return Err(e);
                        }
                    }
                }
            }
            Ok(ExecutionResult {
                output: Output::Text(accumulated_stdout),
                stderr: accumulated_stderr,
                exit_code: last_exit_code,
                error: None,
            })
        })();

        // Exit loop context
        self.runtime.exit_loop();

        result
    }

    fn execute_while_loop(&mut self, while_loop: WhileLoop) -> Result<ExecutionResult> {
        // Enter loop context for break/continue
        self.runtime.enter_loop();

        let mut accumulated_stdout = String::new();
        let mut accumulated_stderr = String::new();
        let mut last_exit_code = 0;

        let result = (|| -> Result<ExecutionResult> {
            loop {
                // Evaluate condition
                let mut condition_exit_code = 0;
                for statement in &while_loop.condition {
                    match self.execute_statement(statement.clone()) {
                        Ok(result) => {
                            condition_exit_code = result.exit_code;
                        }
                        Err(e) => return Err(e),
                    }
                }

                // While loop continues while condition is true (exit code 0)
                if condition_exit_code != 0 {
                    break;
                }

                // Execute body
                for statement in &while_loop.body {
                    match self.execute_statement(statement.clone()) {
                        Ok(result) => {
                            accumulated_stdout.push_str(&result.stdout());
                            accumulated_stderr.push_str(&result.stderr);
                            last_exit_code = result.exit_code;
                        }
                        Err(e) => {
                            // Check if this is a break signal
                            if let Some(break_signal) = e.downcast_ref::<crate::builtins::break_builtin::BreakSignal>() {
                                // First, add any accumulated output from the break signal itself
                                accumulated_stdout.push_str(&break_signal.accumulated_stdout);
                                accumulated_stderr.push_str(&break_signal.accumulated_stderr);

                                if break_signal.levels == 1 {
                                    // Break from this loop, return accumulated output
                                    return Ok(ExecutionResult {
                                        output: Output::Text(accumulated_stdout),
                                        stderr: accumulated_stderr,
                                        exit_code: last_exit_code,
                                        error: None,
                                    });
                                } else {
                                    // Propagate to outer loop with decreased level and accumulated output
                                    return Err(anyhow::Error::new(crate::builtins::break_builtin::BreakSignal {
                                        levels: break_signal.levels - 1,
                                        accumulated_stdout: accumulated_stdout.clone(),
                                        accumulated_stderr: accumulated_stderr.clone(),
                                    }));
                                }
                            }

                            // Check if this is a continue signal
                            if let Some(continue_signal) = e.downcast_ref::<crate::builtins::continue_builtin::ContinueSignal>() {
                                // First, add any accumulated output from the continue signal itself
                                accumulated_stdout.push_str(&continue_signal.accumulated_stdout);
                                accumulated_stderr.push_str(&continue_signal.accumulated_stderr);

                                if continue_signal.levels == 1 {
                                    // Continue in this loop - skip to next iteration
                                    break; // Break out of the statement loop, continue with next item
                                } else {
                                    // Propagate to outer loop with decreased level and accumulated output
                                    return Err(anyhow::Error::new(crate::builtins::continue_builtin::ContinueSignal {
                                        levels: continue_signal.levels - 1,
                                        accumulated_stdout: accumulated_stdout.clone(),
                                        accumulated_stderr: accumulated_stderr.clone(),
                                    }));
                                }
                            }

                            return Err(e);
                        }
                    }
                }
            }
            Ok(ExecutionResult {
                output: Output::Text(accumulated_stdout),
                stderr: accumulated_stderr,
                exit_code: last_exit_code,
                error: None,
            })
        })();

        self.runtime.exit_loop();
        result
    }

    fn execute_until_loop(&mut self, until_loop: UntilLoop) -> Result<ExecutionResult> {
        // Enter loop context for break/continue
        self.runtime.enter_loop();

        let mut accumulated_stdout = String::new();
        let mut accumulated_stderr = String::new();
        let mut last_exit_code = 0;

        let result = (|| -> Result<ExecutionResult> {
            loop {
                // Evaluate condition
                let mut condition_exit_code = 0;
                for statement in &until_loop.condition {
                    match self.execute_statement(statement.clone()) {
                        Ok(result) => {
                            condition_exit_code = result.exit_code;
                        }
                        Err(e) => return Err(e),
                    }
                }

                // Until loop continues until condition is true (exit code 0)
                // So we break when exit code is 0
                if condition_exit_code == 0 {
                    break;
                }

                // Execute body
                for statement in &until_loop.body {
                    match self.execute_statement(statement.clone()) {
                        Ok(result) => {
                            accumulated_stdout.push_str(&result.stdout());
                            accumulated_stderr.push_str(&result.stderr);
                            last_exit_code = result.exit_code;
                        }
                        Err(e) => {
                            // Check if this is a break signal
                            if let Some(break_signal) = e.downcast_ref::<crate::builtins::break_builtin::BreakSignal>() {
                                accumulated_stdout.push_str(&break_signal.accumulated_stdout);
                                accumulated_stderr.push_str(&break_signal.accumulated_stderr);
                                self.runtime.exit_loop();
                                return Ok(ExecutionResult {
                                    output: Output::Text(accumulated_stdout),
                                    stderr: accumulated_stderr,
                                    exit_code: last_exit_code,
                                    error: None,
                                });
                            }

                            // Check if this is a continue signal
                            if let Some(continue_signal) = e.downcast_ref::<crate::builtins::continue_builtin::ContinueSignal>() {
                                accumulated_stdout.push_str(&continue_signal.accumulated_stdout);
                                accumulated_stderr.push_str(&continue_signal.accumulated_stderr);
                                break; // Break inner loop to continue outer loop
                            }

                            return Err(e);
                        }
                    }
                }
            }
            Ok(ExecutionResult {
                output: Output::Text(accumulated_stdout),
                stderr: accumulated_stderr,
                exit_code: last_exit_code,
                error: None,
            })
        })();

        self.runtime.exit_loop();
        result
    }

    fn execute_match(&mut self, match_expr: MatchExpression) -> Result<ExecutionResult> {
        let value = self.evaluate_expression(match_expr.value)?;

        for arm in match_expr.arms {
            if self.pattern_matches(&arm.pattern, &value) {
                for statement in arm.body {
                    self.execute_statement(statement)?;
                }
                break;
            }
        }

        Ok(ExecutionResult::default())
    }
    fn execute_case(&mut self, case_stmt: CaseStatement) -> Result<ExecutionResult> {
        // Evaluate the word to match against
        let word_value = self.evaluate_expression(case_stmt.word)?;
        let word = word_value.trim();

        let mut last_exit_code = 0;
        let mut matched = false;

        // Try each case arm in order
        for arm in case_stmt.arms {
            // Check if any of the patterns match
            for pattern_str in &arm.patterns {
                if self.case_pattern_matches(pattern_str, word) {
                    matched = true;

                    // Execute the body statements
                    for statement in &arm.body {
                        let result = self.execute_statement(statement.clone())?;
                        last_exit_code = result.exit_code;
                    }

                    // Break from this arm after execution (POSIX: only first match executes)
                    break;
                }
            }

            // If we found a match, don't check remaining arms
            if matched {
                break;
            }
        }

        // POSIX: exit code is last command in executed list, or 0 if no match
        Ok(ExecutionResult {
            output: Output::Text(String::new()),
            stderr: String::new(),
            exit_code: if matched { last_exit_code } else { 0 },
            error: None,
        })
    }

    /// Match a pattern against a word for case statements
    /// Supports glob-style patterns: *, ?, [...]
    fn case_pattern_matches(&self, pattern: &str, word: &str) -> bool {
        // Use glob crate's Pattern for matching
        match glob::Pattern::new(pattern) {
            Ok(glob_pattern) => glob_pattern.matches(word),
            Err(_) => {
                // If pattern is invalid, fall back to literal match
                pattern == word
            }
        }
    }

    fn execute_conditional_and(&mut self, cond_and: ConditionalAnd) -> Result<ExecutionResult> {
        // Execute left side
        let left_result = self.execute_statement(*cond_and.left)?;
        self.runtime.set_last_exit_code(left_result.exit_code);
        
        // Only execute right side if left succeeded (exit code 0)
        if left_result.exit_code == 0 {
            let right_result = self.execute_statement(*cond_and.right)?;
            self.runtime.set_last_exit_code(right_result.exit_code);

            Ok(ExecutionResult {
                output: Output::Text(format!("{}{}", left_result.stdout(), right_result.stdout())),
                stderr: format!("{}{}", left_result.stderr, right_result.stderr),
                exit_code: right_result.exit_code,
                error: right_result.error,
            })
        } else {
            // Left failed, return its result
            Ok(left_result)
        }
    }

    fn execute_conditional_or(&mut self, cond_or: ConditionalOr) -> Result<ExecutionResult> {
        // Execute left side
        let left_result = self.execute_statement(*cond_or.left)?;
        self.runtime.set_last_exit_code(left_result.exit_code);
        
        // Only execute right side if left failed (exit code != 0)
        if left_result.exit_code != 0 {
            let right_result = self.execute_statement(*cond_or.right)?;
            self.runtime.set_last_exit_code(right_result.exit_code);

            Ok(ExecutionResult {
                output: Output::Text(format!("{}{}", left_result.stdout(), right_result.stdout())),
                stderr: format!("{}{}", left_result.stderr, right_result.stderr),
                exit_code: right_result.exit_code,
                error: right_result.error,
            })
        } else {
            // Left succeeded, return its result
            Ok(left_result)
        }
    }

    fn execute_subshell(&mut self, statements: Vec<Statement>) -> Result<ExecutionResult> {
        // Clone the runtime to create an isolated environment
        let mut child_runtime = self.runtime.clone();

        // Increment SHLVL in the subshell
        let current_shlvl = child_runtime
            .get_variable("SHLVL")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(1);
        child_runtime.set_variable("SHLVL".to_string(), (current_shlvl + 1).to_string());

        // Create a new executor with the cloned runtime
        let mut child_executor = Executor {
            runtime: child_runtime,
            builtins: self.builtins.clone(),
            corrector: self.corrector.clone(),
            signal_handler: None, // Subshells don't need their own signal handlers
            show_progress: self.show_progress, // Inherit progress setting from parent
            terminal_control: self.terminal_control.clone(),
        };

        // Execute all statements in the subshell
        let result = child_executor.execute(statements)?;

        // The subshell's runtime changes (variables, cwd) are discarded
        // Only the execution result (stdout, stderr, exit code) is returned
        Ok(result)
    }

    fn execute_background(&mut self, statement: Statement) -> Result<ExecutionResult> {
        use std::process::Stdio;

        // For background jobs, we need to spawn a separate process
        // First, let's get the command string for tracking
        let command_str = self.statement_to_string(&statement);

        // Only handle Command statements in background for now
        match statement {
            Statement::Command(command) => {
                // Check if it's a builtin - builtins can't run in background
                if self.builtins.is_builtin(&command.name) {
                    return Err(anyhow!("Builtin commands cannot be run in background"));
                }

                // Resolve arguments
                let args: Result<Vec<String>> = command
                    .args
                    .iter()
                    .map(|arg| self.resolve_argument(arg))
                    .collect();
                
                let args = args?;

                // Spawn the process
                let mut cmd = StdCommand::new(&command.name);
                cmd.args(&args)
                    .current_dir(self.runtime.get_cwd())
                    .envs(self.runtime.get_env())
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());

                // Use pre_exec to set the process group before the child executes
                unsafe {
                    cmd.pre_exec(|| {
                        // Put this process in its own process group (PGID = PID)
                        let pid = getpid();
                        setpgid(pid, pid).map_err(|e| {
                            std::io::Error::new(std::io::ErrorKind::Other, format!("setpgid failed: {}", e))
                        })?;
                        Ok(())
                    });
                }

                let child = cmd.spawn()
                    .map_err(|e| anyhow!("Failed to spawn background process '{}': {}", command.name, e))?;

                let pid = child.id();

                // Add to job manager
                let job_id = self.runtime.job_manager().add_job(pid, command_str);

                // Track last background PID for $!
                self.runtime.set_last_bg_pid(pid);

                // Return success with job information
                Ok(ExecutionResult::success(format!("[{}] {}\n", job_id, pid)))
            }
            _ => Err(anyhow!("Only simple commands can be run in background for now")),
        }
    }

    fn statement_to_string(&self, statement: &Statement) -> String {
        match statement {
            Statement::Command(cmd) => {
                let args_str = cmd.args.iter()
                    .map(|arg| match arg {
                        Argument::Literal(s) | Argument::Variable(s) | Argument::BracedVariable(s) | Argument::CommandSubstitution(s) | Argument::Flag(s) | Argument::Path(s) => s.clone(),
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                if args_str.is_empty() {
                    cmd.name.clone()
                } else {
                    format!("{} {}", cmd.name, args_str)
                }
            }
            Statement::WhileLoop(_) => "while loop".to_string(),
            Statement::UntilLoop(_) => "until loop".to_string(),
            _ => "complex command".to_string(),
        }
    }

    fn evaluate_expression(&mut self, expr: Expression) -> Result<String> {
        match expr {
            Expression::Literal(lit) => Ok(self.literal_to_string(lit)),
            Expression::Variable(name) => {
                // Strip single $ from variable name (use strip_prefix to remove only one $)
                let var_name = name.strip_prefix('$').unwrap_or(&name);

                // Handle special variables first
                if var_name == "$" {
                    return Ok(std::process::id().to_string());
                } else if var_name == "!" {
                    return Ok(self.runtime.get_last_bg_pid()
                        .map(|pid| pid.to_string())
                        .unwrap_or_default());
                } else if var_name == "-" {
                    return Ok(self.runtime.get_option_flags());
                } else if var_name == "_" {
                    return Ok(self.runtime.get_last_arg().to_string());
                } else if var_name == "#" {
                    return Ok(self.runtime.param_count().to_string());
                } else if var_name == "@" {
                    return Ok(self.runtime.get_positional_params().join(" "));
                } else if var_name == "*" {
                    return Ok(self.runtime.get_positional_params().join(" "));
                } else if var_name == "0" {
                    if let Some(val) = self.runtime.get_variable("0") {
                        return Ok(val);
                    } else {
                        return Ok("rush".to_string());
                    }
                } else if var_name == "?" {
                    return Ok(self.runtime.get_last_exit_code().to_string());
                } else if let Ok(index) = var_name.parse::<usize>() {
                    if index > 0 {
                        return Ok(self.runtime.get_positional_param(index).unwrap_or_default());
                    }
                }

                // Regular variable expansion
                // Use get_variable_checked to respect nounset option
                if self.runtime.options.nounset {
                    self.runtime.get_variable_checked(var_name)
                } else {
                    Ok(self.runtime
                        .get_variable(var_name)
                        .unwrap_or_default())
                }
            }
            Expression::VariableExpansion(expansion) => {
                self.runtime.expand_variable(&expansion)
            }
            Expression::CommandSubstitution(cmd) => {
                // Strip $( and )
                let cmd_str = cmd.trim_start_matches("$(").trim_end_matches(')');
                // TODO: Parse and execute the command
                Ok(cmd_str.to_string())
            }
            Expression::FunctionCall(call) => {
                // Evaluate arguments
                let mut args = Vec::new();
                for arg_expr in call.args {
                    args.push(self.evaluate_expression(arg_expr)?);
                }
                // Execute the function and return its stdout
                let result = self.execute_user_function(&call.name, args)?;
                Ok(result.stdout())
            }
            _ => Err(anyhow!("Expression evaluation not yet implemented")),
        }
    }

    fn resolve_argument(&mut self, arg: &Argument) -> Result<String> {
        match arg {
            Argument::Literal(s) => Ok(s.clone()),
            Argument::Variable(var) => {
                // Strip single $ from variable name (use strip_prefix to remove only one $)
                let var_name = var.strip_prefix('$').unwrap_or(var);

                // Handle special variables first
                if var_name == "$" {
                    // $$ - process ID of the shell
                    return Ok(std::process::id().to_string());
                } else if var_name == "!" {
                    // $! - PID of last background command
                    return Ok(self.runtime.get_last_bg_pid()
                        .map(|pid| pid.to_string())
                        .unwrap_or_default());
                } else if var_name == "-" {
                    // $- - current option flags
                    return Ok(self.runtime.get_option_flags());
                } else if var_name == "_" {
                    // $_ - last argument of previous command
                    return Ok(self.runtime.get_last_arg().to_string());
                } else if var_name == "#" {
                    // $# - number of positional parameters
                    return Ok(self.runtime.param_count().to_string());
                } else if var_name == "@" {
                    // $@ - all positional parameters as separate words
                    // For now, return as space-separated string (proper quoting handled later)
                    return Ok(self.runtime.get_positional_params().join(" "));
                } else if var_name == "*" {
                    // $* - all positional parameters
                    return Ok(self.runtime.get_positional_params().join(" "));
                } else if var_name == "0" {
                    // $0 - shell name or script name
                    if let Some(val) = self.runtime.get_variable("0") {
                        return Ok(val);
                    } else {
                        return Ok("rush".to_string());
                    }
                } else if let Ok(index) = var_name.parse::<usize>() {
                    // $1, $2, etc. - positional parameters
                    if index > 0 {
                        return Ok(self.runtime.get_positional_param(index).unwrap_or_default());
                    }
                }

                // Regular variable - just get its value
                Ok(self.runtime.get_variable(var_name).unwrap_or_default())
            }
            Argument::BracedVariable(braced_var) => {
                // Parse the braced variable expansion
                let expansion = self.parse_braced_var_expansion(braced_var)?;

                // Handle special variables in braced expansions
                if expansion.name == "$" {
                    // ${$} - process ID of the shell (no operators allowed)
                    return Ok(std::process::id().to_string());
                } else if expansion.name == "!" {
                    // ${!} - PID of last background command (no operators allowed)
                    return Ok(self.runtime.get_last_bg_pid()
                        .map(|pid| pid.to_string())
                        .unwrap_or_default());
                } else if expansion.name == "-" {
                    // ${-} - current option flags (no operators allowed)
                    return Ok(self.runtime.get_option_flags());
                } else if expansion.name == "_" {
                    // ${_} - last argument of previous command (no operators allowed)
                    return Ok(self.runtime.get_last_arg().to_string());
                } else if expansion.name == "#" {
                    // ${#} - number of positional parameters
                    return Ok(self.runtime.param_count().to_string());
                } else if expansion.name == "@" {
                    // ${@} - all positional parameters
                    return Ok(self.runtime.get_positional_params().join(" "));
                } else if expansion.name == "*" {
                    // ${*} - all positional parameters
                    return Ok(self.runtime.get_positional_params().join(" "));
                } else if expansion.name == "0" {
                    // ${0} - shell name or script name
                    if let Some(val) = self.runtime.get_variable("0") {
                        return Ok(val);
                    } else {
                        return Ok("rush".to_string());
                    }
                } else if let Ok(index) = expansion.name.parse::<usize>() {
                    // ${1}, ${2}, ${10}, etc. - positional parameters
                    if index > 0 {
                        // Check if positional param exists
                        if let Some(value) = self.runtime.get_positional_param(index) {
                            // Param exists - set it in temp runtime and apply operator
                            let mut temp_runtime = self.runtime.clone();
                            temp_runtime.set_variable(expansion.name.clone(), value.clone());
                            return temp_runtime.expand_variable(&expansion);
                        } else {
                            // Param doesn't exist - apply operator to None
                            let mut temp_runtime = self.runtime.clone();
                            // Don't set the variable - let it be unset so operators work correctly
                            return temp_runtime.expand_variable(&expansion);
                        }
                    }
                }

                // Expand it using the runtime
                self.runtime.expand_variable(&expansion)
            }
            Argument::CommandSubstitution(cmd) => {
                // Execute command substitution and return output
                Ok(self.execute_command_substitution(cmd)
                    .unwrap_or_else(|_| String::new()))
            }
            Argument::Flag(f) => Ok(f.clone()),
            Argument::Path(p) => Ok(p.clone()),
        }
    }

    fn parse_braced_var_expansion(&self, braced_var: &str) -> Result<VarExpansion> {
        // Remove ${ and } from the string
        let inner = braced_var.trim_start_matches("${").trim_end_matches('}');

        // Check for different operators in order
        if let Some(pos) = inner.find(":-") {
            let (name, default) = inner.split_at(pos);
            let default = &default[2..]; // Skip :-
            return Ok(VarExpansion {
                name: name.to_string(),
                operator: VarExpansionOp::UseDefault(default.to_string()),
            });
        }

        if let Some(pos) = inner.find(":=") {
            let (name, default) = inner.split_at(pos);
            let default = &default[2..]; // Skip :=
            return Ok(VarExpansion {
                name: name.to_string(),
                operator: VarExpansionOp::AssignDefault(default.to_string()),
            });
        }

        if let Some(pos) = inner.find(":?") {
            let (name, error_msg) = inner.split_at(pos);
            let error_msg = &error_msg[2..]; // Skip :?
            return Ok(VarExpansion {
                name: name.to_string(),
                operator: VarExpansionOp::ErrorIfUnset(error_msg.to_string()),
            });
        }

        if let Some(pos) = inner.find("##") {
            let (name, pattern) = inner.split_at(pos);
            let pattern = &pattern[2..]; // Skip ##
            return Ok(VarExpansion {
                name: name.to_string(),
                operator: VarExpansionOp::RemoveLongestPrefix(pattern.to_string()),
            });
        }

        if let Some(pos) = inner.find('#') {
            let (name, pattern) = inner.split_at(pos);
            let pattern = &pattern[1..]; // Skip #
            return Ok(VarExpansion {
                name: name.to_string(),
                operator: VarExpansionOp::RemoveShortestPrefix(pattern.to_string()),
            });
        }

        if let Some(pos) = inner.find("%%") {
            let (name, pattern) = inner.split_at(pos);
            let pattern = &pattern[2..]; // Skip %%
            return Ok(VarExpansion {
                name: name.to_string(),
                operator: VarExpansionOp::RemoveLongestSuffix(pattern.to_string()),
            });
        }

        if let Some(pos) = inner.find('%') {
            let (name, pattern) = inner.split_at(pos);
            let pattern = &pattern[1..]; // Skip %
            return Ok(VarExpansion {
                name: name.to_string(),
                operator: VarExpansionOp::RemoveShortestSuffix(pattern.to_string()),
            });
        }

        // No operator, just simple expansion
        Ok(VarExpansion {
            name: inner.to_string(),
            operator: VarExpansionOp::Simple,
        })
    }

    /// Expand globs and resolve arguments
    fn expand_and_resolve_arguments(&mut self, args: &[Argument]) -> Result<Vec<String>> {
        let mut expanded_args = Vec::new();

        for arg in args {
            // Determine if this argument should be subject to IFS splitting
            // Only unquoted variables and command substitutions should be split
            let should_split_ifs = matches!(
                arg,
                Argument::Variable(_) | Argument::BracedVariable(_) | Argument::CommandSubstitution(_)
            );
            
            // First resolve the argument (e.g., variable substitution)
            let resolved = self.resolve_argument(arg)?;

            if should_split_ifs {
                // Apply IFS splitting first
                let fields = self.runtime.split_by_ifs(&resolved);
                
                // Then check each field for glob patterns
                for field in fields {
                    if glob_expansion::should_expand_glob(field) {
                        match glob_expansion::expand_globs(field, self.runtime.get_cwd()) {
                            Ok(matches) => {
                                expanded_args.extend(matches);
                            }
                            Err(e) => {
                                // If glob expansion fails (no matches), return the error
                                return Err(anyhow!(e));
                            }
                        }
                    } else {
                        // Not a glob pattern, just add the field
                        expanded_args.push(field.to_string());
                    }
                }
            } else {
                // No IFS splitting for quoted arguments - just check for glob expansion
                if glob_expansion::should_expand_glob(&resolved) {
                    match glob_expansion::expand_globs(&resolved, self.runtime.get_cwd()) {
                        Ok(matches) => {
                            expanded_args.extend(matches);
                        }
                        Err(e) => {
                            // If glob expansion fails (no matches), return the error
                            return Err(anyhow!(e));
                        }
                    }
                } else {
                    // Not a glob pattern, just add the resolved value
                    expanded_args.push(resolved);
                }
            }
        }

        Ok(expanded_args)
    }

    /// Execute a command substitution and return its stdout, trimmed
    fn execute_command_substitution(&self, cmd_str: &str) -> Result<String> {
        use crate::lexer::Lexer;
        use crate::parser::Parser;
        
        // Extract command from $(...) or `...`
        let command = if cmd_str.starts_with("$(") && cmd_str.ends_with(')') {
            &cmd_str[2..cmd_str.len() - 1]
        } else if cmd_str.starts_with('`') && cmd_str.ends_with('`') {
            &cmd_str[1..cmd_str.len() - 1]
        } else {
            cmd_str
        };
        
        // Parse and execute the command
        let tokens = Lexer::tokenize(command)
            .map_err(|e| anyhow!("Failed to tokenize command substitution: {}", e))?;
        let mut parser = Parser::new(tokens);
        let statements = parser.parse()
            .map_err(|e| anyhow!("Failed to parse command substitution: {}", e))?;
        
        // Create a new executor with the same runtime (but cloned to avoid borrow issues)
        let mut sub_executor = Executor {
            runtime: self.runtime.clone(),
            builtins: self.builtins.clone(),
            corrector: self.corrector.clone(),
            signal_handler: None,
            show_progress: false, // Don't show progress for substitutions
            terminal_control: self.terminal_control.clone(),
        };
        
        // Execute the command and capture output
        let result = sub_executor.execute(statements)?;
        
        // Return stdout with trailing newlines trimmed (bash behavior)
        Ok(result.stdout().trim_end().to_string())
    }

    fn literal_to_string(&self, lit: Literal) -> String {
        match lit {
            Literal::String(s) => s,
            Literal::Integer(n) => n.to_string(),
            Literal::Float(f) => f.to_string(),
            Literal::Boolean(b) => b.to_string(),
        }
    }

    fn is_truthy(&self, value: &str) -> bool {
        !value.is_empty() && value != "0" && value != "false"
    }

    fn pattern_matches(&self, pattern: &Pattern, value: &str) -> bool {
        match pattern {
            Pattern::Identifier(id) => id == value,
            Pattern::Literal(lit) => self.literal_to_string(lit.clone()) == value,
            Pattern::Wildcard => true,
        }
    }

    pub fn runtime_mut(&mut self) -> &mut Runtime {
        &mut self.runtime
    }

    /// Execute a trap handler for the given signal
    /// Returns Ok(()) if trap was executed successfully or if no trap is set
    /// Returns Err if trap execution failed
    pub fn execute_trap(&mut self, signal: crate::builtins::trap::TrapSignal) -> Result<()> {
        // Get the trap command for this signal
        let trap_command = match self.runtime.get_trap(signal) {
            Some(cmd) => cmd.clone(),
            None => return Ok(()), // No trap set, nothing to do
        };

        // Empty command means ignore the signal
        if trap_command.is_empty() {
            return Ok(());
        }

        // Execute the trap command
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let tokens = Lexer::tokenize(&trap_command)
            .map_err(|e| anyhow!("Failed to tokenize trap command: {}", e))?;
        let mut parser = Parser::new(tokens);
        let statements = parser.parse()
            .map_err(|e| anyhow!("Failed to parse trap command: {}", e))?;

        // Execute the trap (errors are logged but don't stop execution)
        match self.execute(statements) {
            Ok(_) => Ok(()),
            Err(e) => {
                // Print error but don't fail - traps should be resilient
                eprintln!("trap: error executing {} handler: {}", signal.to_string(), e);
                Ok(())
            }
        }
    }

    /// Execute the EXIT trap if one is set
    /// This should be called before the shell exits
    pub fn execute_exit_trap(&mut self) {
        let _ = self.execute_trap(crate::builtins::trap::TrapSignal::Exit);
    }

    /// Source a file by executing its contents line by line
    /// Used for .rushrc and .rush_profile files
    pub fn source_file(&mut self, path: &std::path::Path) -> Result<()> {
        use std::fs;
        use std::io::{BufRead, BufReader};
        
        // Check if file exists
        if !path.exists() {
            return Ok(()); // Silently ignore missing config files
        }

        // Read file
        let file = fs::File::open(path)
            .map_err(|e| anyhow!("Failed to open '{}': {}", path.display(), e))?;
        let reader = BufReader::new(file);

        // Execute each line
        for (line_num, line) in reader.lines().enumerate() {
            let line = line?;
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Execute the line
            match self.execute_line_internal(line) {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("{}:{}: {}", path.display(), line_num + 1, e);
                    // Continue executing other lines even if one fails
                }
            }
        }

        Ok(())
    }

    /// Internal helper to execute a single line
    fn execute_line_internal(&mut self, line: &str) -> Result<ExecutionResult> {
        use crate::lexer::Lexer;
        use crate::parser::Parser;
        
        let tokens = Lexer::tokenize(line)?;
        let mut parser = Parser::new(tokens);
        let statements = parser.parse()?;
        self.execute(statements)
    }
}

fn resolve_argument_static(arg: &Argument, runtime: &Runtime) -> String {
    match arg {
        Argument::Literal(s) => s.clone(),
        Argument::Variable(var) => {
            let var_name = var.trim_start_matches('$');
            runtime
                .get_variable(var_name)
                .unwrap_or_else(|| var.clone())
        }
        Argument::BracedVariable(var) => {
            // Strip ${ and } from variable name
            let var_name = var.trim_start_matches("${").trim_end_matches('}');
            runtime
                .get_variable(var_name)
                .unwrap_or_else(|| var.clone())
        }
        Argument::CommandSubstitution(cmd) => {
            // For parallel execution, we need to execute command substitution
            // Create a minimal executor for this
            use crate::lexer::Lexer;
            use crate::parser::Parser;
            
            let command = if cmd.starts_with("$(") && cmd.ends_with(')') {
                &cmd[2..cmd.len() - 1]
            } else if cmd.starts_with('`') && cmd.ends_with('`') {
                &cmd[1..cmd.len() - 1]
            } else {
                cmd.as_str()
            };
            
            // Try to execute the command substitution
            if let Ok(tokens) = Lexer::tokenize(command) {
                let mut parser = Parser::new(tokens);
                if let Ok(statements) = parser.parse() {
                    let mut sub_executor = Executor {
                        runtime: runtime.clone(),
                        builtins: Builtins::new(),
                        corrector: Corrector::new(),
                        signal_handler: None,
                        show_progress: false,
                        terminal_control: TerminalControl::new(),
                    };
                    if let Ok(result) = sub_executor.execute(statements) {
                        return result.stdout().trim_end().to_string();
                    }
                }
            }
            
            // If execution failed, return empty string
            String::new()
        }
        Argument::Flag(f) => f.clone(),
        Argument::Path(p) => p.clone(),
    }
}

// Helper function for parallel execution with glob expansion
fn expand_and_resolve_arguments_static(args: &[Argument], runtime: &Runtime) -> Result<Vec<String>> {
    let mut expanded_args = Vec::new();

    for arg in args {
        let resolved = resolve_argument_static(arg, runtime);

        if glob_expansion::should_expand_glob(&resolved) {
            match glob_expansion::expand_globs(&resolved, runtime.get_cwd()) {
                Ok(matches) => {
                    expanded_args.extend(matches);
                }
                Err(e) => {
                    return Err(anyhow!(e));
                }
            }
        } else {
            expanded_args.push(resolved);
        }
    }

    Ok(expanded_args)
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub output: Output,
    pub stderr: String,
    pub exit_code: i32,
    /// Optional typed error information
    pub error: Option<String>,
}

/// Output can be either traditional text or structured data
#[derive(Debug, Clone)]
pub enum Output {
    Text(String),
    Structured(serde_json::Value),
}

impl Default for ExecutionResult {
    fn default() -> Self {
        Self {
            output: Output::Text(String::new()),
            stderr: String::new(),
            exit_code: 0,
            error: None,
        }
    }
}

impl Output {
    /// Get the text representation of this output
    pub fn as_text(&self) -> String {
        match self {
            Output::Text(s) => s.clone(),
            Output::Structured(v) => {
                // Convert JSON value to pretty-printed string
                serde_json::to_string_pretty(v).unwrap_or_else(|_| String::new())
            }
        }
    }
}

impl ExecutionResult {
    pub fn success(text: String) -> Self {
        Self {
            output: Output::Text(text),
            stderr: String::new(),
            exit_code: 0,
            error: None,
        }
    }

    pub fn error(stderr: String) -> Self {
        Self {
            output: Output::Text(String::new()),
            stderr,
            exit_code: 1,
            error: None,
        }
    }

    // /// Create an error result from a typed RushError
    // pub fn error_typed(error: crate::error::RushError) -> Self {
    //     let stderr = if crate::error::should_output_json_errors() {
    //         error.to_json()
    //     } else {
    //         error.to_text()
    //     };
    //
    //     Self {
    //         output: Output::Text(String::new()),
    //         stderr,
    //         exit_code: error.exit_code,
    //         error: Some(error),
    //     }
    // }

    pub fn stdout(&self) -> String {
        self.output.as_text()
    }

    /// Get mutable reference to stdout text (only works for Text output)
    pub fn stdout_mut(&mut self) -> Option<&mut String> {
        match &mut self.output {
            Output::Text(s) => Some(s),
            Output::Structured(_) => None,
        }
    }

    /// Clear stdout content (only works for Text output)
    pub fn clear_stdout(&mut self) {
        if let Output::Text(s) = &mut self.output {
            s.clear();
        }
    }

    /// Append to stdout (only works for Text output)
    pub fn push_stdout(&mut self, text: &str) {
        if let Output::Text(s) = &mut self.output {
            s.push_str(text);
        }
    }
}
