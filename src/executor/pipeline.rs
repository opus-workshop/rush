use super::ExecutionResult;
use crate::builtins::Builtins;
use crate::parser::ast::*;
use crate::runtime::Runtime;
use anyhow::{anyhow, Result};
use std::process::{Command as StdCommand, Stdio};

pub fn execute_pipeline(
    pipeline: Pipeline,
    runtime: &mut Runtime,
    builtins: &Builtins,
) -> Result<ExecutionResult> {
    if pipeline.commands.is_empty() {
        return Ok(ExecutionResult::default());
    }

    if pipeline.commands.len() == 1 {
        // Single command, execute normally
        return execute_single_command(&pipeline.commands[0], runtime, builtins);
    }

    // Multi-command pipeline
    let mut previous_output = Vec::new();

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

        if is_last {
            return Ok(result);
        }

        previous_output = result.stdout.into_bytes();
    }

    Ok(ExecutionResult::default())
}

fn execute_pipeline_command(
    command: &Command,
    runtime: &mut Runtime,
    builtins: &Builtins,
    stdin: Option<&[u8]>,
) -> Result<ExecutionResult> {
    // Check if it's a builtin
    if builtins.is_builtin(&command.name) {
        let args: Vec<String> = command
            .args
            .iter()
            .map(|arg| resolve_argument(arg, runtime))
            .collect();

        // If there's stdin, add it as an argument (simplified for now)
        let result = builtins.execute(&command.name, args, runtime)?;

        if let Some(input) = stdin {
            // For builtins that accept stdin, we'd handle it here
            // For now, just pass through
            Ok(result)
        } else {
            Ok(result)
        }
    } else {
        execute_external_pipeline_command(command, runtime, stdin)
    }
}

fn execute_external_pipeline_command(
    command: &Command,
    runtime: &Runtime,
    stdin: Option<&[u8]>,
) -> Result<ExecutionResult> {
    let args: Vec<String> = command
        .args
        .iter()
        .map(|arg| resolve_argument(arg, runtime))
        .collect();

    let mut cmd = StdCommand::new(&command.name);
    cmd.args(&args)
        .current_dir(runtime.get_cwd())
        .envs(runtime.get_env());

    if let Some(input) = stdin {
        cmd.stdin(Stdio::piped());
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| anyhow!("Failed to spawn '{}': {}", command.name, e))?;

    if let Some(input) = stdin {
        use std::io::Write;
        if let Some(mut stdin_handle) = child.stdin.take() {
            stdin_handle
                .write_all(input)
                .map_err(|e| anyhow!("Failed to write to stdin: {}", e))?;
        }
    }

    let output = child
        .wait_with_output()
        .map_err(|e| anyhow!("Failed to wait for '{}': {}", command.name, e))?;

    Ok(ExecutionResult {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(1),
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
