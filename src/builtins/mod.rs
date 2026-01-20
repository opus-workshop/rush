use crate::executor::ExecutionResult;
use crate::runtime::Runtime;
use crate::correction::Corrector;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::env;

mod cat;
mod find;
mod git_status;
mod grep;
mod ls;
mod undo;

type BuiltinFn = fn(&[String], &mut Runtime) -> Result<ExecutionResult>;

#[derive(Clone)]
pub struct Builtins {
    commands: HashMap<String, BuiltinFn>,
}

impl Builtins {
    pub fn new() -> Self {
        let mut commands: HashMap<String, BuiltinFn> = HashMap::new();

        commands.insert("cd".to_string(), builtin_cd);
        commands.insert("pwd".to_string(), builtin_pwd);
        commands.insert("echo".to_string(), builtin_echo);
        commands.insert("exit".to_string(), builtin_exit);
        commands.insert("export".to_string(), builtin_export);
        commands.insert("cat".to_string(), cat::builtin_cat);
        commands.insert("find".to_string(), find::builtin_find);
        commands.insert("ls".to_string(), ls::builtin_ls);
        commands.insert("git-status".to_string(), git_status::builtin_git_status);
        commands.insert("grep".to_string(), grep::builtin_grep);
        commands.insert("undo".to_string(), undo::builtin_undo);

        Self { commands }
    }

    pub fn is_builtin(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }

    pub fn builtin_names(&self) -> Vec<String> {
        self.commands.keys().cloned().collect()
    }

    pub fn execute(
        &self,
        name: &str,
        args: Vec<String>,
        runtime: &mut Runtime,
    ) -> Result<ExecutionResult> {
        if let Some(func) = self.commands.get(name) {
            func(&args, runtime)
        } else {
            Err(anyhow!("Builtin '{}' not found", name))
        }
    }

    /// Execute a builtin with optional stdin data
    pub fn execute_with_stdin(
        &self,
        name: &str,
        args: Vec<String>,
        runtime: &mut Runtime,
        stdin: Option<&[u8]>,
    ) -> Result<ExecutionResult> {
        // Special handling for cat with stdin
        if name == "cat" && stdin.is_some() {
            return cat::builtin_cat_with_stdin(&args, runtime, stdin.unwrap());
        }
        
        // Special handling for grep with stdin
        if name == "grep" && stdin.is_some() {
            return grep::builtin_grep_with_stdin(&args, runtime, stdin.unwrap());
        }
        
        // For other builtins, use regular execute
        self.execute(name, args, runtime)
    }
}

fn builtin_cd(args: &[String], runtime: &mut Runtime) -> Result<ExecutionResult> {
    let target = if args.is_empty() {
        dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?
    } else {
        let path = &args[0];
        if path == "-" {
            // TODO: Implement previous directory tracking
            runtime.get_cwd().clone()
        } else if path.starts_with('~') {
            let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
            home.join(path.trim_start_matches("~/"))
        } else {
            PathBuf::from(path)
        }
    };

    let absolute = if target.is_absolute() {
        target
    } else {
        runtime.get_cwd().join(target)
    };

    if !absolute.exists() {
        // Provide path suggestions
        let corrector = Corrector::new();
        let suggestions = corrector.suggest_path(&absolute, runtime.get_cwd());
        
        let mut error_msg = format!("cd: no such file or directory: {:?}", absolute);
        
        if !suggestions.is_empty() {
            error_msg.push_str("\n\nDid you mean?");
            for suggestion in suggestions.iter().take(3) {
                let similarity = Corrector::similarity_percent(suggestion.score, &suggestion.text);
                error_msg.push_str(&format!(
                    "\n  {} ({}%, {})",
                    suggestion.text,
                    similarity,
                    suggestion.kind.label()
                ));
            }
        }
        
        return Err(anyhow!(error_msg));
    }

    if !absolute.is_dir() {
        return Err(anyhow!("cd: not a directory: {:?}", absolute));
    }

    // Update runtime's cwd
    runtime.set_cwd(absolute.clone());
    
    // Also update the process's actual current directory so other parts can see it
    env::set_current_dir(&absolute)?;
    
    Ok(ExecutionResult::success(String::new()))
}

fn builtin_pwd(_args: &[String], runtime: &mut Runtime) -> Result<ExecutionResult> {
    let cwd = runtime.get_cwd();
    Ok(ExecutionResult::success(
        cwd.to_string_lossy().to_string() + "\n",
    ))
}

fn builtin_echo(args: &[String], _runtime: &mut Runtime) -> Result<ExecutionResult> {
    let output = args.join(" ") + "\n";
    Ok(ExecutionResult::success(output))
}

fn builtin_exit(args: &[String], _runtime: &mut Runtime) -> Result<ExecutionResult> {
    let code = if args.is_empty() {
        0
    } else {
        args[0].parse::<i32>().unwrap_or(0)
    };

    std::process::exit(code);
}

fn builtin_export(args: &[String], runtime: &mut Runtime) -> Result<ExecutionResult> {
    if args.is_empty() {
        return Err(anyhow!("export: usage: export VAR=value"));
    }

    for arg in args {
        if let Some((key, value)) = arg.split_once('=') {
            runtime.set_env(key, value);
            runtime.set_variable(key.to_string(), value.to_string());
        } else {
            return Err(anyhow!("export: invalid syntax: {}", arg));
        }
    }

    Ok(ExecutionResult::success(String::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo() {
        let mut runtime = Runtime::new();
        let result = builtin_echo(&["hello".to_string(), "world".to_string()], &mut runtime)
            .unwrap();
        assert_eq!(result.stdout, "hello world\n");
    }

    #[test]
    fn test_pwd() {
        let mut runtime = Runtime::new();
        let result = builtin_pwd(&[], &mut runtime).unwrap();
        assert!(!result.stdout.is_empty());
    }
}
