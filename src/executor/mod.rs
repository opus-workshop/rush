pub mod pipeline;

use crate::builtins::Builtins;
use crate::parser::ast::*;
use crate::runtime::Runtime;
use anyhow::{anyhow, Result};
use std::process::{Command as StdCommand, Stdio};

pub struct Executor {
    runtime: Runtime,
    builtins: Builtins,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            runtime: Runtime::new(),
            builtins: Builtins::new(),
        }
    }

    pub fn execute(&mut self, statements: Vec<Statement>) -> Result<ExecutionResult> {
        let mut last_result = ExecutionResult::default();

        for statement in statements {
            last_result = self.execute_statement(statement)?;
        }

        Ok(last_result)
    }

    pub fn execute_statement(&mut self, statement: Statement) -> Result<ExecutionResult> {
        match statement {
            Statement::Command(cmd) => self.execute_command(cmd),
            Statement::Pipeline(pipeline) => self.execute_pipeline(pipeline),
            Statement::Assignment(assignment) => self.execute_assignment(assignment),
            Statement::FunctionDef(func) => self.execute_function_def(func),
            Statement::IfStatement(if_stmt) => self.execute_if_statement(if_stmt),
            Statement::ForLoop(for_loop) => self.execute_for_loop(for_loop),
            Statement::MatchExpression(match_expr) => self.execute_match(match_expr),
        }
    }

    fn execute_command(&mut self, command: Command) -> Result<ExecutionResult> {
        // Check if it's a builtin command
        if self.builtins.is_builtin(&command.name) {
            let args: Vec<String> = command
                .args
                .iter()
                .map(|arg| self.resolve_argument(arg))
                .collect();
            return self.builtins.execute(&command.name, args, &mut self.runtime);
        }

        // Execute external command
        self.execute_external_command(command)
    }

    fn execute_external_command(&self, command: Command) -> Result<ExecutionResult> {
        let args: Vec<String> = command
            .args
            .iter()
            .map(|arg| self.resolve_argument(arg))
            .collect();

        let output = StdCommand::new(&command.name)
            .args(&args)
            .current_dir(self.runtime.get_cwd())
            .envs(self.runtime.get_env())
            .output()
            .map_err(|e| anyhow!("Failed to execute '{}': {}", command.name, e))?;

        Ok(ExecutionResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(1),
        })
    }

    fn execute_pipeline(&mut self, pipeline: Pipeline) -> Result<ExecutionResult> {
        pipeline::execute_pipeline(pipeline, &mut self.runtime, &self.builtins)
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

    fn evaluate_expression(&self, expr: Expression) -> Result<String> {
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
