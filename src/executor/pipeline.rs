use super::{ExecutionResult, Output, Executor, CallStack};
use crate::builtins::Builtins;
use crate::correction::Corrector;
use crate::executor::suggestions::SuggestionEngine;
use crate::glob_expansion;
use crate::parser::ast::*;
use crate::runtime::Runtime;
use crate::terminal::TerminalControl;
use anyhow::{anyhow, Result};
use std::io::Write;
use std::process::{Command as StdCommand, Stdio};

/// Execute a pipeline of commands with proper streaming and error handling
///
/// This implementation supports:
/// - Multi-stage pipelines with proper data streaming
/// - SIGPIPE handling (broken pipe errors)
/// - Proper exit code propagation (last command's exit code)
/// - Works with both builtins and external commands
/// - pipefail option: pipeline fails if any command fails
/// - PIPESTATUS array tracking each command's exit code
pub fn execute_pipeline(
    pipeline: Pipeline,
    runtime: &mut Runtime,
    builtins: &Builtins,
) -> Result<ExecutionResult> {
    // Use elements if available, fall back to commands-only for backward compat
    if !pipeline.elements.is_empty() {
        return execute_pipeline_elements(&pipeline.elements, runtime, builtins);
    }

    if pipeline.commands.is_empty() {
        return Ok(ExecutionResult::default());
    }

    if pipeline.commands.len() == 1 {
        // Single command, execute normally
        let result = execute_single_command(&pipeline.commands[0], runtime, builtins)?;
        runtime.set_last_exit_code(result.exit_code);
        // Set PIPESTATUS for single command
        runtime.set_pipestatus(vec![result.exit_code]);
        return Ok(result);
    }

    // Multi-command pipeline with streaming
    let mut previous_output = Vec::new();
    let mut first_failed_exit_code = None;
    let mut combined_stderr = String::new();
    let mut pipestatus = Vec::new();

    for (i, command) in pipeline.commands.iter().enumerate() {
        let is_first = i == 0;
        let is_last = i == pipeline.commands.len() - 1;

        let result = execute_pipeline_command(
            command,
            runtime,
            builtins,
            if is_first {
                None
            } else {
                Some(&previous_output)
            },
        )?;

        // Track exit code for PIPESTATUS
        pipestatus.push(result.exit_code);

        // Track first non-zero exit code for pipefail
        if runtime.options.pipefail && result.exit_code != 0 && first_failed_exit_code.is_none() {
            first_failed_exit_code = Some(result.exit_code);
        }

        // Accumulate stderr
        if !result.stderr.is_empty() {
            combined_stderr.push_str(&result.stderr);
        }

        if is_last {
            // Determine the exit code based on pipefail option
            let pipeline_exit_code = if runtime.options.pipefail {
                if let Some(code) = first_failed_exit_code {
                    code
                } else {
                    result.exit_code
                }
            } else {
                result.exit_code
            };

            // Set $? to the pipeline's exit code
            runtime.set_last_exit_code(pipeline_exit_code);
            // Set PIPESTATUS array
            runtime.set_pipestatus(pipestatus);

            return Ok(ExecutionResult {
                output: Output::Text(result.stdout()),
                stderr: combined_stderr,
                exit_code: pipeline_exit_code,
                error: None,
            });
        }

        previous_output = result.stdout().into_bytes();
    }

    Ok(ExecutionResult::default())
}

/// Execute pipeline using the new elements representation which supports subshells.
fn execute_pipeline_elements(
    elements: &[PipelineElement],
    runtime: &mut Runtime,
    builtins: &Builtins,
) -> Result<ExecutionResult> {
    if elements.is_empty() {
        return Ok(ExecutionResult::default());
    }

    if elements.len() == 1 {
        let result = execute_element(&elements[0], runtime, builtins, None)?;
        runtime.set_last_exit_code(result.exit_code);
        runtime.set_pipestatus(vec![result.exit_code]);
        return Ok(result);
    }

    let mut previous_output = Vec::new();
    let mut first_failed_exit_code = None;
    let mut combined_stderr = String::new();
    let mut pipestatus = Vec::new();

    for (i, element) in elements.iter().enumerate() {
        let is_first = i == 0;
        let is_last = i == elements.len() - 1;

        let result = execute_element(
            element,
            runtime,
            builtins,
            if is_first { None } else { Some(&previous_output) },
        )?;

        pipestatus.push(result.exit_code);

        if runtime.options.pipefail && result.exit_code != 0 && first_failed_exit_code.is_none() {
            first_failed_exit_code = Some(result.exit_code);
        }

        if !result.stderr.is_empty() {
            combined_stderr.push_str(&result.stderr);
        }

        if is_last {
            let pipeline_exit_code = if runtime.options.pipefail {
                first_failed_exit_code.unwrap_or(result.exit_code)
            } else {
                result.exit_code
            };

            runtime.set_last_exit_code(pipeline_exit_code);
            runtime.set_pipestatus(pipestatus);

            return Ok(ExecutionResult {
                output: Output::Text(result.stdout()),
                stderr: combined_stderr,
                exit_code: pipeline_exit_code,
                error: None,
            });
        }

        previous_output = result.stdout().into_bytes();
    }

    Ok(ExecutionResult::default())
}

/// Execute a single pipeline element, which can be a command or a subshell.
fn execute_element(
    element: &PipelineElement,
    runtime: &mut Runtime,
    builtins: &Builtins,
    stdin: Option<&[u8]>,
) -> Result<ExecutionResult> {
    match element {
        PipelineElement::Command(cmd) => {
            execute_pipeline_command(cmd, runtime, builtins, stdin)
        }
        PipelineElement::Subshell(statements) => {
            execute_subshell_in_pipeline(statements, runtime, builtins, stdin)
        }
    }
}

/// Execute a subshell as part of a pipeline.
/// Creates a child executor with cloned runtime and passes stdin data.
fn execute_subshell_in_pipeline(
    statements: &[Statement],
    runtime: &mut Runtime,
    builtins: &Builtins,
    stdin: Option<&[u8]>,
) -> Result<ExecutionResult> {
    use crate::correction::Corrector;
    use crate::terminal::TerminalControl;

    // Create isolated child runtime
    let mut child_runtime = runtime.clone();
    let current_shlvl = child_runtime
        .get_variable("SHLVL")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(1);
    child_runtime.set_variable("SHLVL".to_string(), (current_shlvl + 1).to_string());

    // If there's stdin data, set it as the content of stdin for the subshell
    // For builtins like `cat` that read stdin, we need to make this available
    if let Some(input_data) = stdin {
        // Set the piped input as a special variable the executor can access
        child_runtime.set_variable(
            "_PIPE_STDIN".to_string(),
            String::from_utf8_lossy(input_data).to_string(),
        );
    }

    let mut child_executor = Executor {
        runtime: child_runtime,
        builtins: builtins.clone(),
        corrector: Corrector::new(),
        suggestion_engine: SuggestionEngine::new(),
        signal_handler: None,
        show_progress: false,
        terminal_control: TerminalControl::new(),
        call_stack: CallStack::new(),
        profile_data: None,
        enable_profiling: false,
    };

    // Execute the subshell statements
    match child_executor.execute(statements.to_vec()) {
        Ok(result) => Ok(result),
        Err(e) => {
            if let Some(exit_signal) = e.downcast_ref::<crate::builtins::exit_builtin::ExitSignal>() {
                Ok(ExecutionResult {
                    exit_code: exit_signal.exit_code,
                    ..ExecutionResult::default()
                })
            } else {
                Err(e)
            }
        }
    }
}

fn execute_pipeline_command(
    command: &Command,
    runtime: &mut Runtime,
    builtins: &Builtins,
    stdin: Option<&[u8]>,
) -> Result<ExecutionResult> {
    // Check if it's a builtin
    if builtins.is_builtin(&command.name) {
        let args = resolve_and_expand_arguments(&command.args, runtime);

        // Use execute_with_stdin to properly handle piped input
        builtins.execute_with_stdin(&command.name, args, runtime, stdin)
    } else {
        execute_external_pipeline_command(command, runtime, stdin)
    }
}

fn execute_external_pipeline_command(
    command: &Command,
    runtime: &Runtime,
    stdin: Option<&[u8]>,
) -> Result<ExecutionResult> {
    let args = resolve_and_expand_arguments(&command.args, runtime);

    let mut cmd = StdCommand::new(&command.name);
    cmd.args(&args)
        .current_dir(runtime.get_cwd())
        .envs(runtime.get_env())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if stdin.is_some() {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::inherit());
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| anyhow!("Failed to spawn '{}': {}", command.name, e))?;

    // Write stdin data if provided
    if let Some(input) = stdin {
        if let Some(mut stdin_handle) = child.stdin.take() {
            // Handle SIGPIPE - if the process exits before reading all input,
            // we don't want to fail the entire pipeline
            stdin_handle.write_all(input).or_else(|e| {
                if e.kind() == std::io::ErrorKind::BrokenPipe {
                    // Process closed pipe, that's OK
                    Ok(())
                } else {
                    Err(e)
                }
            }).map_err(|e| anyhow!("Failed to write to stdin of '{}': {}", command.name, e))?;
        }
    }

    let output = child
        .wait_with_output()
        .map_err(|e| anyhow!("Failed to wait for '{}': {}", command.name, e))?;

    Ok(ExecutionResult {
        output: Output::Text(String::from_utf8_lossy(&output.stdout).to_string()),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(1),
        error: None,
    })
}

fn execute_single_command(
    command: &Command,
    runtime: &mut Runtime,
    builtins: &Builtins,
) -> Result<ExecutionResult> {
    execute_pipeline_command(command, runtime, builtins, None)
}

fn resolve_argument(arg: &Argument, runtime: &Runtime) -> String {
    match arg {
        Argument::Literal(s) => {
            if s.contains("$(") || s.contains('`') {
                super::expand_command_substitutions_in_string_static(s, runtime)
            } else {
                s.clone()
            }
        }
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
            // For pipelines, we need to execute command substitution
            // Create a minimal executor for this
            use crate::lexer::Lexer;
            use crate::parser::Parser;
            use crate::executor::Executor;
            use crate::builtins::Builtins;
            use crate::correction::Corrector;
            
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
                        suggestion_engine: SuggestionEngine::new(),
                        signal_handler: None,
                        terminal_control: crate::terminal::TerminalControl::new(),
                        show_progress: false,
                        profile_data: None,
                        enable_profiling: false,
                        call_stack: CallStack::new(),
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
        Argument::Path(p) => super::expand_tilde(p),
        Argument::Glob(g) => g.clone(),
    }
}

/// Resolve arguments and expand globs for pipeline commands.
/// This ensures glob expansion works in pipeline stages just like in regular commands.
fn resolve_and_expand_arguments(args: &[Argument], runtime: &Runtime) -> Vec<String> {
    let mut expanded = Vec::new();
    for arg in args {
        let should_expand = matches!(
            arg,
            Argument::Glob(_) | Argument::Path(_) | Argument::Variable(_)
            | Argument::BracedVariable(_) | Argument::CommandSubstitution(_)
        );
        let resolved = resolve_argument(arg, runtime);
        if should_expand && glob_expansion::should_expand_glob(&resolved) {
            match glob_expansion::expand_globs(&resolved, runtime.get_cwd()) {
                Ok(matches) => expanded.extend(matches),
                Err(_) => expanded.push(resolved),
            }
        } else {
            expanded.push(resolved);
        }
    }
    expanded
}
