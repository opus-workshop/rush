use crate::parser::ast::FunctionDef;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

pub struct Runtime {
    variables: HashMap<String, String>,
    functions: HashMap<String, FunctionDef>,
    cwd: PathBuf,
}

impl Runtime {
    pub fn new() -> Self {
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));

        Self {
            variables: HashMap::new(),
            functions: HashMap::new(),
            cwd,
        }
    }

    pub fn set_variable(&mut self, name: String, value: String) {
        self.variables.insert(name, value);
    }

    pub fn get_variable(&self, name: &str) -> Option<String> {
        self.variables.get(name).cloned()
    }

    pub fn define_function(&mut self, func: FunctionDef) {
        self.functions.insert(func.name.clone(), func);
    }

    pub fn get_function(&self, name: &str) -> Option<&FunctionDef> {
        self.functions.get(name)
    }

    pub fn set_cwd(&mut self, path: PathBuf) {
        self.cwd = path;
    }

    pub fn get_cwd(&self) -> &PathBuf {
        &self.cwd
    }

    pub fn get_env(&self) -> HashMap<String, String> {
        env::vars().collect()
    }

    pub fn set_env(&self, key: &str, value: &str) {
        env::set_var(key, value);
    }
}
