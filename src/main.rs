mod lexer;
mod parser;
mod executor;
mod runtime;
mod builtins;
mod completion;
mod history;
mod context;
mod output;
mod git;

use executor::Executor;
use lexer::Lexer;
use parser::Parser;
use reedline::{DefaultPrompt, Reedline, Signal};
use anyhow::Result;

fn main() -> Result<()> {
    println!("Rush v0.1.0 - A Modern Shell in Rust");
    println!("Type 'exit' to quit\n");

    let mut executor = Executor::new();
    let mut line_editor = Reedline::create();
    let prompt = DefaultPrompt::default();

    loop {
        let sig = line_editor.read_line(&prompt);

        match sig {
            Ok(Signal::Success(buffer)) => {
                let line = buffer.trim();

                if line.is_empty() {
                    continue;
                }

                match execute_line(line, &mut executor) {
                    Ok(result) => {
                        if !result.stdout.is_empty() {
                            print!("{}", result.stdout);
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

fn execute_line(line: &str, executor: &mut Executor) -> Result<executor::ExecutionResult> {
    // Tokenize
    let tokens = Lexer::tokenize(line)?;

    // Parse
    let mut parser = Parser::new(tokens);
    let statements = parser.parse()?;

    // Execute
    executor.execute(statements)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_echo() {
        let mut executor = Executor::new();
        let result = execute_line("echo hello", &mut executor).unwrap();
        assert_eq!(result.stdout, "hello\n");
    }

    #[test]
    fn test_execute_pwd() {
        let mut executor = Executor::new();
        let result = execute_line("pwd", &mut executor).unwrap();
        assert!(!result.stdout.is_empty());
    }
}
