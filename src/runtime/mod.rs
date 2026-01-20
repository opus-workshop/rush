use crate::parser::ast::FunctionDef;
use crate::history::History;
use crate::undo::UndoManager;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

#[derive(Clone)]
pub struct Runtime {
    variables: HashMap<String, String>,
    functions: HashMap<String, FunctionDef>,
    cwd: PathBuf,
    scopes: Vec<HashMap<String, String>>,
    call_stack: Vec<String>,
    max_call_depth: usize,
    history: History,
    undo_manager: UndoManager,
}

impl Runtime {
    pub fn new() -> Self {
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let undo_manager = UndoManager::new().unwrap_or_else(|e| {
            eprintln!("Warning: Failed to initialize undo manager: {}", e);
            // Create a disabled undo manager as fallback
            panic!("Cannot create undo manager");
        });

        let mut runtime = Self {
            variables: HashMap::new(),
            functions: HashMap::new(),
            cwd,
            scopes: Vec::new(),
            call_stack: Vec::new(),
            max_call_depth: 100,
            history: History::default(),
            undo_manager,
        };
        
        // Initialize $? to 0
        runtime.set_last_exit_code(0);
        runtime
    }

    pub fn set_variable(&mut self, name: String, value: String) {
        // If we're in a function scope, set the variable in the current scope
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, value);
        } else {
            // Otherwise set in global scope
            self.variables.insert(name, value);
        }
    }

    pub fn get_variable(&self, name: &str) -> Option<String> {
        // Check scopes from most recent to oldest
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value.clone());
            }
        }
        // Fall back to global variables
        self.variables.get(name).cloned()
    }

    /// Set the last exit code (stored in $? variable)
    pub fn set_last_exit_code(&mut self, code: i32) {
        self.variables.insert("?".to_string(), code.to_string());
    }

    /// Get the last exit code (from $? variable)
    pub fn get_last_exit_code(&self) -> i32 {
        self.variables
            .get("?")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    }

    pub fn define_function(&mut self, func: FunctionDef) {
        self.functions.insert(func.name.clone(), func);
    }

    pub fn get_function(&self, name: &str) -> Option<&FunctionDef> {
        self.functions.get(name)
    }

    /// Get all user-defined function names
    pub fn get_function_names(&self) -> Vec<String> {
        self.functions.keys().cloned().collect()
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

    // Scope management for function calls
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    // Call stack management
    pub fn push_call(&mut self, name: String) -> Result<(), String> {
        if self.call_stack.len() >= self.max_call_depth {
            return Err(format!("Maximum recursion depth exceeded ({})", self.max_call_depth));
        }
        self.call_stack.push(name);
        Ok(())
    }

    pub fn pop_call(&mut self) {
        self.call_stack.pop();
    }

    // History management
    pub fn history(&self) -> &History {
        &self.history
    }

    pub fn history_mut(&mut self) -> &mut History {
        &mut self.history
    }

    pub fn load_history(&mut self) -> Result<(), String> {
        self.history.load().map_err(|e| e.to_string())
    }

    pub fn add_to_history(&mut self, command: String) -> Result<(), String> {
        self.history.add(command).map_err(|e| e.to_string())
    }

    // Undo manager access
    pub fn undo_manager(&self) -> &UndoManager {
        &self.undo_manager
    }

    pub fn undo_manager_mut(&mut self) -> &mut UndoManager {
        &mut self.undo_manager
    }
}
