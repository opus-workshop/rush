use crate::executor::ExecutionResult;
use crate::runtime::Runtime;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::PathBuf;

type BuiltinFn = fn(&[String], &mut Runtime) -> Result<ExecutionResult>;

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

        Self { commands }
    }

    pub fn is_builtin(&self, name: &str) -> bool {
        self.commands.contains_key(name)
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
        return Err(anyhow!("cd: no such file or directory: {:?}", absolute));
    }

    if !absolute.is_dir() {
        return Err(anyhow!("cd: not a directory: {:?}", absolute));
    }

    runtime.set_cwd(absolute);
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
