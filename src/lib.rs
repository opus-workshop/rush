// Library interface for Rush shell
// This allows benchmarks and tests to access internal modules

pub mod lexer;
pub mod parser;
pub mod executor;
pub mod runtime;
pub mod builtins;
pub mod completion;
pub mod history;
pub mod context;
pub mod output;
pub mod git;
