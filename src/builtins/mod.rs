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
mod help;
mod ls;
mod mkdir;
mod undo;
mod jobs;
mod set;
mod alias;
mod test;
mod type_builtin;

type BuiltinFn = fn(&[String], &mut Runtime) -> Result<ExecutionResult>;

#[derive(Clone)]
pub struct Builtins {
    commands: HashMap<String, BuiltinFn>,
}

impl Default for Builtins {
    fn default() -> Self {
        Self::new()
    }
}

impl Builtins {
    pub fn new() -> Self {
        let mut commands: HashMap<String, BuiltinFn> = HashMap::new();

        commands.insert("cd".to_string(), builtin_cd);
        commands.insert("pwd".to_string(), builtin_pwd);
        commands.insert("echo".to_string(), builtin_echo);
        commands.insert("exit".to_string(), builtin_exit);
        commands.insert("export".to_string(), builtin_export);
        commands.insert("source".to_string(), builtin_source);
        commands.insert("cat".to_string(), cat::builtin_cat);
        commands.insert("find".to_string(), find::builtin_find);
        commands.insert("ls".to_string(), ls::builtin_ls);
        commands.insert("mkdir".to_string(), mkdir::builtin_mkdir);
        commands.insert("git-status".to_string(), git_status::builtin_git_status);
        commands.insert("grep".to_string(), grep::builtin_grep);
        commands.insert("undo".to_string(), undo::builtin_undo);
        commands.insert("jobs".to_string(), jobs::builtin_jobs);
        commands.insert("fg".to_string(), jobs::builtin_fg);
        commands.insert("bg".to_string(), jobs::builtin_bg);
        commands.insert("set".to_string(), set::builtin_set);
        commands.insert("alias".to_string(), alias::builtin_alias);
        commands.insert("unalias".to_string(), alias::builtin_unalias);
        commands.insert("test".to_string(), test::builtin_test);
        commands.insert("[".to_string(), test::builtin_bracket);
        commands.insert("help".to_string(), help::builtin_help);
        commands.insert("type".to_string(), type_builtin::builtin_type);

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
        if name == "cat" {
            if let Some(stdin_data) = stdin {
                return cat::builtin_cat_with_stdin(&args, runtime, stdin_data);
            }
        }
        
        // Special handling for grep with stdin
        if name == "grep" {
            if let Some(stdin_data) = stdin {
                return grep::builtin_grep_with_stdin(&args, runtime, stdin_data);
            }
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

// TODO: Implement builtin_source properly with executor access
#[allow(dead_code)]
fn builtin_source(args: &[String], runtime: &mut Runtime) -> Result<ExecutionResult> {
    if args.is_empty() {
        return Err(anyhow!("source: usage: source <file>"));
    }

    use std::fs;
    use std::io::{BufRead, BufReader};
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::executor::Executor;

    let file_path = &args[0];
    let path = if file_path.starts_with('~') {
        let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
        home.join(file_path.trim_start_matches("~/"))
    } else {
        PathBuf::from(file_path)
    };

    // Make path absolute if relative
    let path = if path.is_absolute() {
        path
    } else {
        runtime.get_cwd().join(path)
    };

    if !path.exists() {
        return Err(anyhow!("source: {}: No such file or directory", file_path));
    }

    // Read and execute file
    let file = fs::File::open(&path)
        .map_err(|e| anyhow!("source: Failed to open '{}': {}", path.display(), e))?;
    let reader = BufReader::new(file);

    // We need an executor to run the commands, but we can't access it from here
    // So we'll return the file contents as a special marker that main.rs can handle
    // For now, execute line by line in a basic way
    for (line_num, line) in reader.lines().enumerate() {
        let line = line?;
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse and execute - we need to do this carefully
        // since we don't have access to executor here
        match Lexer::tokenize(line) {
            Ok(tokens) => {
                let mut parser = Parser::new(tokens);
                match parser.parse() {
                    Ok(statements) => {
                        // Create temporary executor with current runtime
                        let mut executor = Executor::new();
                        // Copy runtime state (this is not ideal but works for source)
                        *executor.runtime_mut() = runtime.clone();
                        
                        match executor.execute(statements) {
                            Ok(result) => {
                                // Copy back runtime state to preserve variable changes
                                *runtime = executor.runtime_mut().clone();
                                // Print any output
                                if !result.stdout.is_empty() {
                                    print!("{}", result.stdout);
                                }
                                if !result.stderr.is_empty() {
                                    eprint!("{}", result.stderr);
                                }
                            }
                            Err(e) => {
                                eprintln!("{}:{}: {}", path.display(), line_num + 1, e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("{}:{}: Parse error: {}", path.display(), line_num + 1, e);
                    }
                }
            }
            Err(e) => {
                eprintln!("{}:{}: Tokenize error: {}", path.display(), line_num + 1, e);
            }
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
