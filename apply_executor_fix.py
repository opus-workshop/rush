#!/usr/bin/env python3
"""
Applies executor fixes for piping into compound commands.
Bean 211.7
"""

def fix_executor_mod(content):
    # Add BraceGroup handling in execute_statement
    content = content.replace(
        '''Statement::Subshell(statements) => self.execute_subshell(statements),
            Statement::BackgroundCommand(cmd) => self.execute_background(*cmd),
        }
    }

    fn execute_command''',
        '''Statement::Subshell(statements) => self.execute_subshell(statements),
            Statement::BackgroundCommand(cmd) => self.execute_background(*cmd),
            Statement::BraceGroup(statements) => self.execute_brace_group(statements),
        }
    }

    fn execute_command'''
    )
    
    # Add execute_brace_group method (after execute_subshell ends)
    content = content.replace(
        '''// The subshell's runtime changes (variables, cwd) are discarded
        // Only the execution result (stdout, stderr, exit code) is returned
        Ok(result)
    }

    fn execute_background''',
        '''// The subshell's runtime changes (variables, cwd) are discarded
        // Only the execution result (stdout, stderr, exit code) is returned
        Ok(result)
    }

    /// Execute a brace group { commands; }
    /// Unlike subshells, brace groups execute in the current shell context.
    /// Variable changes, directory changes, etc. persist after execution.
    fn execute_brace_group(&mut self, statements: Vec<Statement>) -> Result<ExecutionResult> {
        // Execute statements in current context (not isolated like subshell)
        self.execute(statements)
    }

    fn execute_background'''
    )
    
    return content

def fix_executor_pipeline(content):
    # Update stage_name match
    content = content.replace(
        '''let stage_name = match element {
                PipelineElement::Command(cmd) => cmd.name.clone(),
                PipelineElement::Subshell(_) => "subshell".to_string(),
            };
            let is_builtin = match element {
                PipelineElement::Command(cmd) => builtins.is_builtin(&cmd.name),
                PipelineElement::Subshell(_) => false,
            };''',
        '''let stage_name = match element {
                PipelineElement::Command(cmd) => cmd.name.clone(),
                PipelineElement::Subshell(_) => "subshell".to_string(),
                PipelineElement::CompoundCommand(stmt) => match stmt.as_ref() {
                    Statement::WhileLoop(_) => "while".to_string(),
                    Statement::UntilLoop(_) => "until".to_string(),
                    Statement::ForLoop(_) => "for".to_string(),
                    Statement::IfStatement(_) => "if".to_string(),
                    Statement::CaseStatement(_) => "case".to_string(),
                    Statement::BraceGroup(_) => "brace_group".to_string(),
                    _ => "compound".to_string(),
                },
            };
            let is_builtin = match element {
                PipelineElement::Command(cmd) => builtins.is_builtin(&cmd.name),
                PipelineElement::Subshell(_) | PipelineElement::CompoundCommand(_) => false,
            };'''
    )
    
    # Update execute_element to handle CompoundCommand
    content = content.replace(
        '''/// Execute a single pipeline element, which can be a command or a subshell.
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
}''',
        '''/// Execute a single pipeline element, which can be a command, subshell, or compound command.
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
        PipelineElement::CompoundCommand(stmt) => {
            execute_compound_in_pipeline(stmt, runtime, builtins, stdin)
        }
    }
}

/// Execute a compound command (while, until, for, if, case, brace group) as part of a pipeline.
/// The compound command receives stdin from the pipe and its output goes to stdout.
fn execute_compound_in_pipeline(
    statement: &Statement,
    runtime: &mut Runtime,
    builtins: &Builtins,
    stdin: Option<&[u8]>,
) -> Result<ExecutionResult> {
    use crate::correction::Corrector;
    use crate::terminal::TerminalControl;

    // Set up piped input as a special variable the executor can access
    let mut child_runtime = runtime.clone();
    if let Some(input_data) = stdin {
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

    // Execute the compound command
    match child_executor.execute(vec![statement.clone()]) {
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
}'''
    )
    
    return content

def main():
    # Fix executor/mod.rs
    with open('src/executor/mod.rs', 'r') as f:
        content = f.read()
    
    fixed = fix_executor_mod(content)
    
    with open('src/executor/mod.rs', 'w') as f:
        f.write(fixed)
    
    print("Fixed src/executor/mod.rs")
    
    # Fix executor/pipeline.rs
    with open('src/executor/pipeline.rs', 'r') as f:
        content = f.read()
    
    fixed = fix_executor_pipeline(content)
    
    with open('src/executor/pipeline.rs', 'w') as f:
        f.write(fixed)
    
    print("Fixed src/executor/pipeline.rs")

if __name__ == '__main__':
    main()
