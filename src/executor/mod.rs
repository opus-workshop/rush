pub mod pipeline;

use crate::builtins::Builtins;
use crate::correction::Corrector;
use crate::parser::ast::*;
use crate::runtime::Runtime;
use crate::progress::ProgressIndicator;
use crate::signal::SignalHandler;
use anyhow::{anyhow, Result};
use std::process::Command as StdCommand;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub struct Executor {
    runtime: Runtime,
    builtins: Builtins,
    corrector: Corrector,
    signal_handler: Option<SignalHandler>,
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
        }
    }

    pub fn new_with_signal_handler(signal_handler: SignalHandler) -> Self {
        Self {
            runtime: Runtime::new(),
            builtins: Builtins::new(),
            corrector: Corrector::new(),
            signal_handler: Some(signal_handler),
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
                    return Err(anyhow!("Interrupted by signal"));
                }
            }

            let result = self.execute_statement(statement)?;
            accumulated_stdout.push_str(&result.stdout);
            accumulated_stderr.push_str(&result.stderr);
            last_exit_code = result.exit_code;
            
            // Update $? after each statement
            self.runtime.set_last_exit_code(last_exit_code);
        }

        Ok(ExecutionResult {
            stdout: accumulated_stdout,
            stderr: accumulated_stderr,
            exit_code: last_exit_code,
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
            Statement::MatchExpression(match_expr) => self.execute_match(match_expr),
            Statement::ConditionalAnd(cond_and) => self.execute_conditional_and(cond_and),
            Statement::ConditionalOr(cond_or) => self.execute_conditional_or(cond_or),
            Statement::Subshell(statements) => self.execute_subshell(statements),
        }
    }

    fn execute_command(&mut self, command: Command) -> Result<ExecutionResult> {
        // Check if it's a user-defined function first
        if self.runtime.get_function(&command.name).is_some() {
            let args: Vec<String> = command
                .args
                .iter()
                .map(|arg| self.resolve_argument(arg))
                .collect();
            return self.execute_user_function(&command.name, args);
        }

        // Check if it's a builtin command
        if self.builtins.is_builtin(&command.name) {
            let args: Vec<String> = command
                .args
                .iter()
                .map(|arg| self.resolve_argument(arg))
                .collect();
            let mut result = self.builtins.execute(&command.name, args, &mut self.runtime)?;
            
            // Handle redirects for builtins
            if !command.redirects.is_empty() {
                result = self.apply_redirects(result, &command.redirects)?;
            }
            
            self.runtime.set_last_exit_code(result.exit_code);
            return Ok(result);
        }

        // Execute external command
        let result = self.execute_external_command(command)?;
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
                        file.write_all(result.stdout.as_bytes())
                            .map_err(|e| anyhow!("Failed to write to '{}': {}", target, e))?;
                        result.stdout.clear(); // Clear stdout as it's been redirected
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
                        file.write_all(result.stdout.as_bytes())
                            .map_err(|e| anyhow!("Failed to write to '{}': {}", target, e))?;
                        result.stdout.clear(); // Clear stdout as it's been redirected
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
                    result.stdout.push_str(&result.stderr);
                    result.stderr.clear();
                }
                RedirectKind::Both => {
                    if let Some(target) = &redirect.target {
                        let resolved = resolve_path(target);
                        let mut file = File::create(&resolved)
                            .map_err(|e| anyhow!("Failed to create '{}': {}", target, e))?;
                        file.write_all(result.stdout.as_bytes())
                            .map_err(|e| anyhow!("Failed to write to '{}': {}", target, e))?;
                        file.write_all(result.stderr.as_bytes())
                            .map_err(|e| anyhow!("Failed to write to '{}': {}", target, e))?;
                        result.stdout.clear();
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

        // Execute function body
        let mut last_result = ExecutionResult::default();
        for statement in func.body {
            let stmt_result = self.execute_statement(statement)?;
            // Accumulate stdout from all statements
            last_result.stdout.push_str(&stmt_result.stdout);
            last_result.stderr.push_str(&stmt_result.stderr);
            // Keep the last exit code
            last_result.exit_code = stmt_result.exit_code;
        }

        // Clean up scope and call stack
        self.runtime.pop_scope();
        self.runtime.pop_call();

        Ok(last_result)
    }

    fn execute_external_command(&self, command: Command) -> Result<ExecutionResult> {
        let args: Vec<String> = command
            .args
            .iter()
            .map(|arg| self.resolve_argument(arg))
            .collect();

        
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
        
        // Set default piped outputs if not redirected
        if !stdout_redirect {
            cmd.stdout(std::process::Stdio::piped());
        }
        if !stderr_redirect && !stderr_to_stdout {
            cmd.stderr(std::process::Stdio::piped());
        } else if stderr_to_stdout && !stderr_redirect {
            // Redirect stderr to stdout for the process
            cmd.stderr(std::process::Stdio::piped());
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
                // Command still running, show progress indicator
                Some(ProgressIndicator::new(format!("Running {}", command.name)))
            }
        };

        // Wait for command to complete, checking for signals
        let output = loop {
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

        // Stop progress indicator if it was started
        if let Some(prog) = progress {
            prog.stop();
        }

        // Handle stderr to stdout redirection in output
        let mut stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();
        
        if stderr_to_stdout && !stderr_str.is_empty() {
            stdout_str.push_str(&stderr_str);
        }

        Ok(ExecutionResult {
            stdout: stdout_str,
            stderr: if stderr_to_stdout { String::new() } else { stderr_str },
            exit_code: output.status.code().unwrap_or(1),
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
                    let args: Vec<String> = command
                        .args
                        .iter()
                        .map(|arg| resolve_argument_static(arg, &runtime_snapshot))
                        .collect();
                    
                    // We need a mutable runtime, but we can't safely share it across threads
                    // For now, create a temporary runtime for builtins in parallel execution
                    let mut temp_runtime = (*runtime_snapshot).clone();
                    builtins.execute(&command.name, args, &mut temp_runtime)
                } else {
                    // Execute external command
                    let args: Vec<String> = command
                        .args
                        .iter()
                        .map(|arg| resolve_argument_static(arg, &runtime_snapshot))
                        .collect();

                    match StdCommand::new(&command.name)
                        .args(&args)
                        .current_dir(runtime_snapshot.get_cwd())
                        .envs(runtime_snapshot.get_env())
                        .output()
                    {
                        Ok(output) => Ok(ExecutionResult {
                            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                            exit_code: output.status.code().unwrap_or(1),
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
                    combined_stdout.push_str(&result.stdout);
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
            stdout: combined_stdout,
            stderr: combined_stderr,
            exit_code: max_exit_code,
        })
    }

    fn execute_assignment(&mut self, assignment: Assignment) -> Result<ExecutionResult> {
        let value = self.evaluate_expression(assignment.value)?;
        self.runtime.set_variable(assignment.name, value);
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

        for item in items {
            self.runtime
                .set_variable(for_loop.variable.clone(), item);
            for statement in &for_loop.body {
                self.execute_statement(statement.clone())?;
            }
        }

        Ok(ExecutionResult::default())
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

    fn execute_conditional_and(&mut self, cond_and: ConditionalAnd) -> Result<ExecutionResult> {
        // Execute left side
        let left_result = self.execute_statement(*cond_and.left)?;
        self.runtime.set_last_exit_code(left_result.exit_code);
        
        // Only execute right side if left succeeded (exit code 0)
        if left_result.exit_code == 0 {
            let right_result = self.execute_statement(*cond_and.right)?;
            self.runtime.set_last_exit_code(right_result.exit_code);
            
            Ok(ExecutionResult {
                stdout: format!("{}{}", left_result.stdout, right_result.stdout),
                stderr: format!("{}{}", left_result.stderr, right_result.stderr),
                exit_code: right_result.exit_code,
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
                stdout: format!("{}{}", left_result.stdout, right_result.stdout),
                stderr: format!("{}{}", left_result.stderr, right_result.stderr),
                exit_code: right_result.exit_code,
            })
        } else {
            // Left succeeded, return its result
            Ok(left_result)
        }
    }

    fn execute_subshell(&mut self, statements: Vec<Statement>) -> Result<ExecutionResult> {
        // Clone the runtime to create an isolated environment
        let child_runtime = self.runtime.clone();

        // Create a new executor with the cloned runtime
        let mut child_executor = Executor {
            runtime: child_runtime,
            builtins: self.builtins.clone(),
            corrector: self.corrector.clone(),
            signal_handler: None, // Subshells don't need their own signal handlers
        };

        // Execute all statements in the subshell
        let result = child_executor.execute(statements)?;

        // The subshell's runtime changes (variables, cwd) are discarded
        // Only the execution result (stdout, stderr, exit code) is returned
        Ok(result)
    }

    fn evaluate_expression(&mut self, expr: Expression) -> Result<String> {
        match expr {
            Expression::Literal(lit) => Ok(self.literal_to_string(lit)),
            Expression::Variable(name) => self
                .runtime
                .get_variable(&name)
                .ok_or_else(|| anyhow!("Variable '{}' not found", name)),
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
                Ok(result.stdout)
            }
            _ => Err(anyhow!("Expression evaluation not yet implemented")),
        }
    }

    fn resolve_argument(&self, arg: &Argument) -> String {
        match arg {
            Argument::Literal(s) => s.clone(),
            Argument::Variable(var) => {
                // Strip $ from variable name
                let var_name = var.trim_start_matches('$');
                self.runtime
                    .get_variable(var_name)
                    .unwrap_or_else(|| var.clone())
            }
            Argument::Flag(f) => f.clone(),
            Argument::Path(p) => p.clone(),
        }
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
}

// Helper function for parallel execution
fn resolve_argument_static(arg: &Argument, runtime: &Runtime) -> String {
    match arg {
        Argument::Literal(s) => s.clone(),
        Argument::Variable(var) => {
            let var_name = var.trim_start_matches('$');
            runtime
                .get_variable(var_name)
                .unwrap_or_else(|| var.clone())
        }
        Argument::Flag(f) => f.clone(),
        Argument::Path(p) => p.clone(),
    }
}

#[derive(Debug, Clone, Default)]
pub struct ExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl ExecutionResult {
    pub fn success(stdout: String) -> Self {
        Self {
            stdout,
            stderr: String::new(),
            exit_code: 0,
        }
    }

    pub fn error(stderr: String) -> Self {
        Self {
            stdout: String::new(),
            stderr,
            exit_code: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use crate::lexer::Lexer;

    #[test]
    fn test_parallel_execution() {
        let mut executor = Executor::new();
        
        // Parse parallel execution: echo hello ||| echo world
        let tokens = Lexer::tokenize("echo hello ||| echo world").unwrap();
        let mut parser = Parser::new(tokens);
        let statements = parser.parse().unwrap();
        
        let result = executor.execute(statements).unwrap();
        
        // Both outputs should be present (order may vary due to parallel execution)
        assert!(result.stdout.contains("hello"));
        assert!(result.stdout.contains("world"));
        assert_eq!(result.exit_code, 0);
    }
}
