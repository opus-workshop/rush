use crate::parser::ast::FunctionDef;
use crate::parser::ast::{VarExpansion, VarExpansionOp};
use crate::history::History;
use crate::undo::UndoManager;
use crate::jobs::JobManager;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use anyhow::{anyhow, Result};

/// Shell options that control execution behavior
#[derive(Clone, Default)]
pub struct ShellOptions {
    pub errexit: bool,      // Exit on error (set -e)
    pub pipefail: bool,     // Pipeline fails if any command fails (set -o pipefail)
    pub nounset: bool,      // Error on undefined variable (set -u)
    pub xtrace: bool,       // Print commands before executing (set -x)
    pub noclobber: bool,    // Prevent overwriting files (set -C)
    pub verbose: bool,      // Print input lines as they are read (set -v)
}

/// Runtime environment for the shell
#[derive(Clone)]
pub struct Runtime {
    variables: HashMap<String, String>,
    functions: HashMap<String, FunctionDef>,
    aliases: HashMap<String, String>,
    cwd: PathBuf,
    scopes: Vec<HashMap<String, String>>,
    call_stack: Vec<String>,
    max_call_depth: usize,
    history: Option<History>,  // Lazy initialization
    undo_manager: Option<UndoManager>,  // Lazy initialization
    job_manager: JobManager,
    pub options: ShellOptions,
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime {
    pub fn new() -> Self {
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));

        let mut runtime = Self {
            variables: HashMap::new(),
            functions: HashMap::new(),
            aliases: HashMap::new(),
            cwd,
            scopes: Vec::new(),
            call_stack: Vec::new(),
            max_call_depth: 100,
            history: None,  // Lazy initialization
            undo_manager: None,  // Lazy initialization
            job_manager: JobManager::new(),
            options: ShellOptions::default(),
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

    /// Get variable with nounset option check
    pub fn get_variable_checked(&self, name: &str) -> Result<String> {
        match self.get_variable(name) {
            Some(value) => Ok(value),
            None => {
                if self.options.nounset {
                    Err(anyhow!("{}: unbound variable", name))
                } else {
                    Ok(String::new())
                }
            }
        }
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

    // Alias management
    pub fn set_alias(&mut self, name: String, value: String) {
        self.aliases.insert(name, value);
    }

    pub fn get_alias(&self, name: &str) -> Option<&String> {
        self.aliases.get(name)
    }

    pub fn remove_alias(&mut self, name: &str) -> bool {
        self.aliases.remove(name).is_some()
    }

    pub fn get_all_aliases(&self) -> &HashMap<String, String> {
        &self.aliases
    }

    pub fn set_cwd(&mut self, path: PathBuf) {
        self.cwd = path;
    }

    pub fn get_cwd(&self) -> &PathBuf {
        &self.cwd
    }

    // Shell options management
    pub fn set_option(&mut self, option: &str, value: bool) -> Result<()> {
        match option {
            "e" | "errexit" => self.options.errexit = value,
            "u" | "nounset" => self.options.nounset = value,
            "x" | "xtrace" => self.options.xtrace = value,
            "pipefail" => self.options.pipefail = value,
            _ => return Err(anyhow!("Unknown option: {}", option)),
        }
        Ok(())
    }

    pub fn get_option(&self, option: &str) -> Result<bool> {
        match option {
            "e" | "errexit" => Ok(self.options.errexit),
            "u" | "nounset" => Ok(self.options.nounset),
            "x" | "xtrace" => Ok(self.options.xtrace),
            "pipefail" => Ok(self.options.pipefail),
            _ => Err(anyhow!("Unknown option: {}", option)),
        }
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
    pub fn history(&mut self) -> &History {
        if self.history.is_none() {
            self.history = Some(History::default());
        }
        self.history.as_ref().unwrap()
    }

    pub fn history_mut(&mut self) -> &mut History {
        if self.history.is_none() {
            self.history = Some(History::default());
        }
        self.history.as_mut().unwrap()
    }

    pub fn load_history(&mut self) -> Result<(), String> {
        if self.history.is_none() {
            self.history = Some(History::default());
        }
        self.history.as_mut().unwrap().load().map_err(|e| e.to_string())
    }

    pub fn add_to_history(&mut self, command: String) -> Result<(), String> {
        self.history_mut().add(command).map_err(|e| e.to_string())
    }

    // Undo manager access
    pub fn undo_manager(&self) -> Option<&UndoManager> {
        self.undo_manager.as_ref()
    }

    pub fn undo_manager_mut(&mut self) -> &mut UndoManager {
        if self.undo_manager.is_none() {
            // Initialize on first use
            let manager = UndoManager::new().unwrap_or_else(|e| {
                eprintln!("Warning: Failed to initialize undo manager: {}", e);
                panic!("Cannot create undo manager");
            });
            self.undo_manager = Some(manager);
        }
        self.undo_manager.as_mut().unwrap()
    }

    // Job manager access
    pub fn job_manager(&self) -> &JobManager {
        &self.job_manager
    }

    pub fn job_manager_mut(&mut self) -> &mut JobManager {
        &mut self.job_manager
    }

    /// Expand a variable with operators like ${VAR:-default}
    pub fn expand_variable(&mut self, expansion: &VarExpansion) -> Result<String> {
        let var_value = self.get_variable(&expansion.name);
        
        match &expansion.operator {
            VarExpansionOp::Simple => {
                Ok(var_value.unwrap_or_default())
            }
            VarExpansionOp::UseDefault(default) => {
                Ok(var_value.unwrap_or_else(|| default.clone()))
            }
            VarExpansionOp::AssignDefault(default) => {
                if let Some(value) = var_value {
                    Ok(value)
                } else {
                    self.set_variable(expansion.name.clone(), default.clone());
                    Ok(default.clone())
                }
            }
            VarExpansionOp::ErrorIfUnset(error_msg) => {
                var_value.ok_or_else(|| {
                    anyhow!("{}: {}", expansion.name, error_msg)
                })
            }
            VarExpansionOp::RemoveShortestPrefix(pattern) => {
                let value = var_value.unwrap_or_default();
                Ok(Self::remove_prefix(&value, pattern, false))
            }
            VarExpansionOp::RemoveLongestPrefix(pattern) => {
                let value = var_value.unwrap_or_default();
                Ok(Self::remove_prefix(&value, pattern, true))
            }
            VarExpansionOp::RemoveShortestSuffix(pattern) => {
                let value = var_value.unwrap_or_default();
                Ok(Self::remove_suffix(&value, pattern, false))
            }
            VarExpansionOp::RemoveLongestSuffix(pattern) => {
                let value = var_value.unwrap_or_default();
                Ok(Self::remove_suffix(&value, pattern, true))
            }
        }
    }

    /// Remove prefix pattern from value
    fn remove_prefix(value: &str, pattern: &str, longest: bool) -> String {
        // Simple glob pattern matching
        if pattern.contains('*') {
            // Extract the literal part after the *
            let literal_part = if pattern.ends_with('*') {
                pattern.trim_end_matches('*')
            } else {
                // Pattern like "*foo/" - the part after * is the literal to match
                pattern.split('*').next_back().unwrap_or("")
            };

            if literal_part.is_empty() {
                return value.to_string();
            }

            if longest {
                // ## - Remove the longest match (find last occurrence)
                if let Some(pos) = value.rfind(literal_part) {
                    return value[pos + literal_part.len()..].to_string();
                }
            } else {
                // # - Remove the shortest match (find first occurrence)
                if let Some(pos) = value.find(literal_part) {
                    return value[pos + literal_part.len()..].to_string();
                }
            }
        } else {
            // Literal prefix match
            if let Some(stripped) = value.strip_prefix(pattern) {
                return stripped.to_string();
            }
        }
        value.to_string()
    }

    /// Remove suffix pattern from value
    fn remove_suffix(value: &str, pattern: &str, longest: bool) -> String {
        // Simple glob pattern matching
        if pattern.contains('*') {
            // Extract the literal part before the *
            let literal_part = if pattern.starts_with('*') {
                pattern.trim_start_matches('*')
            } else {
                // Pattern like ".tar*" - the part before * is the literal to match
                pattern.split('*').next().unwrap_or("")
            };

            if literal_part.is_empty() {
                return value.to_string();
            }

            if longest {
                // %% - Remove the longest match (find first occurrence)
                if let Some(pos) = value.find(literal_part) {
                    return value[..pos].to_string();
                }
            } else {
                // % - Remove the shortest match (find last occurrence)
                if let Some(pos) = value.rfind(literal_part) {
                    return value[..pos].to_string();
                }
            }
        } else {
            // Literal suffix match
            if let Some(stripped) = value.strip_suffix(pattern) {
                return stripped.to_string();
            }
        }
        value.to_string()
    }
}
