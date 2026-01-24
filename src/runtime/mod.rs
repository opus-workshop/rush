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
    positional_params: Vec<String>,  // Track $1, $2, etc. for shift builtin
    positional_stack: Vec<Vec<String>>,  // Stack for function scopes
    function_depth: usize,  // Track function call depth for return builtin
    // Permanent file descriptor redirections (set by exec builtin)
    permanent_stdout: Option<i32>,
    permanent_stderr: Option<i32>,
    permanent_stdin: Option<i32>,
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
            positional_params: Vec::new(),
            positional_stack: Vec::new(),
            function_depth: 0,
            permanent_stdout: None,
            permanent_stderr: None,
            permanent_stdin: None,
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

    /// Remove a variable from the current scope or global scope
    /// Returns true if the variable was found and removed
    pub fn remove_variable(&mut self, name: &str) -> bool {
        // If we're in a function scope, try to remove from the current scope first
        if let Some(scope) = self.scopes.last_mut() {
            if scope.remove(name).is_some() {
                return true;
            }
        }
        // Otherwise remove from global scope
        self.variables.remove(name).is_some()
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

    /// Remove a function definition
    /// Returns true if the function was found and removed
    pub fn remove_function(&mut self, name: &str) -> bool {
        self.functions.remove(name).is_some()
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
        // Note: local_variables field doesn't exist yet - commented out for now
        // if let Some(scope) = self.scopes.pop() {
        //     // Clear local variables that were in this scope
        //     for key in scope.keys() {
        //         self.local_variables.remove(key);
        //     }
        // }
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

    // Function context tracking for return builtin
    pub fn enter_function_context(&mut self) {
        self.function_depth += 1;
    }

    pub fn exit_function_context(&mut self) {
        if self.function_depth > 0 {
            self.function_depth -= 1;
        }
    }

    pub fn in_function_context(&self) -> bool {
        self.function_depth > 0
    }
    
    /// Alias for in_function_context (for backward compatibility with local builtin)
    pub fn in_function(&self) -> bool {
        self.in_function_context()
    }
    
    /// Set a local variable in the current function scope
    /// Returns an error if not in a function scope
    pub fn set_local_variable(&mut self, name: String, value: String) -> Result<()> {
        if !self.in_function_context() {
            return Err(anyhow!("Cannot set local variable outside of function"));
        }
        // Set in the current scope (which should be a function scope)
        self.set_variable(name, value);
        Ok(())
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

    // Positional parameter management
    
    /// Set all positional parameters ($1, $2, etc.)
    pub fn set_positional_params(&mut self, params: Vec<String>) {
        self.positional_params = params;
        self.update_positional_variables();
    }
    
    /// Get a specific positional parameter by index (1-based)
    pub fn get_positional_param(&self, index: usize) -> Option<String> {
        if index == 0 {
            // $0 is handled separately
            None
        } else {
            self.positional_params.get(index - 1).cloned()
        }
    }
    
    /// Get all positional parameters
    pub fn get_positional_params(&self) -> &[String] {
        &self.positional_params
    }
    
    /// Shift positional parameters by n positions
    pub fn shift_params(&mut self, n: usize) -> Result<()> {
        if n > self.positional_params.len() {
            return Err(anyhow!(
                "shift: shift count ({}) exceeds number of positional parameters ({})",
                n,
                self.positional_params.len()
            ));
        }
        
        // Remove first n parameters
        self.positional_params.drain(0..n);
        
        // Update $1, $2, $#, $@, $* variables
        self.update_positional_variables();
        
        Ok(())
    }
    
    /// Push positional parameters onto stack (for function calls)
    pub fn push_positional_scope(&mut self, params: Vec<String>) {
        self.positional_stack.push(self.positional_params.clone());
        self.positional_params = params;
        self.update_positional_variables();
    }
    
    /// Pop positional parameters from stack (after function returns)
    pub fn pop_positional_scope(&mut self) {
        if let Some(params) = self.positional_stack.pop() {
            self.positional_params = params;
            self.update_positional_variables();
        }
    }
    
    /// Get the count of positional parameters (for $#)
    pub fn param_count(&self) -> usize {
        self.positional_params.len()
    }
    
    /// Update $1, $2, $#, $@, $* variables based on current positional params
    fn update_positional_variables(&mut self) {
        // Get old count BEFORE updating $#
        let old_count = self.variables.get("#")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);

        // Update $# (parameter count)
        self.variables.insert("#".to_string(), self.positional_params.len().to_string());

        // Update $@ and $* (all parameters as space-separated string)
        let all_params = self.positional_params.join(" ");
        self.variables.insert("@".to_string(), all_params.clone());
        self.variables.insert("*".to_string(), all_params);

        // Clear old numbered parameters that are no longer in use
        for i in 1..=old_count.max(self.positional_params.len()) {
            self.variables.remove(&i.to_string());
        }

        // Set new numbered parameters
        for (i, param) in self.positional_params.iter().enumerate() {
            self.variables.insert((i + 1).to_string(), param.clone());
        }
    }

    // Permanent file descriptor redirection management (for exec builtin)
    
    /// Set permanent stdout redirection file descriptor
    pub fn set_permanent_stdout(&mut self, fd: Option<i32>) {
        self.permanent_stdout = fd;
    }
    
    /// Get permanent stdout redirection file descriptor
    pub fn get_permanent_stdout(&self) -> Option<i32> {
        self.permanent_stdout
    }
    
    /// Set permanent stderr redirection file descriptor
    pub fn set_permanent_stderr(&mut self, fd: Option<i32>) {
        self.permanent_stderr = fd;
    }
    
    /// Get permanent stderr redirection file descriptor
    pub fn get_permanent_stderr(&self) -> Option<i32> {
        self.permanent_stderr
    }
    
    /// Set permanent stdin redirection file descriptor
    pub fn set_permanent_stdin(&mut self, fd: Option<i32>) {
        self.permanent_stdin = fd;
    }
    
    /// Get permanent stdin redirection file descriptor
    pub fn get_permanent_stdin(&self) -> Option<i32> {
        self.permanent_stdin
    }

    // Trap handler management
    
    // TODO: Re-enable when trap module exists
    // /// Set a trap handler for a signal
    // pub fn set_trap(&mut self, signal: TrapSignal, command: String) {
    //     self.trap_handlers.set(signal, command);
    // }

    // /// Remove a trap handler for a signal
    // pub fn remove_trap(&mut self, signal: TrapSignal) {
    //     self.trap_handlers.remove(signal);
    // }

    // /// Get the trap handler for a signal
    // pub fn get_trap(&self, signal: TrapSignal) -> Option<&String> {
    //     self.trap_handlers.get(signal)
    // }

    // /// Get all trap handlers
    // pub fn get_all_traps(&self) -> &HashMap<TrapSignal, String> {
    //     self.trap_handlers.all()
    // }
    
    // /// Check if a signal has a trap handler
    // pub fn has_trap(&self, signal: TrapSignal) -> bool {
    //     self.trap_handlers.has_handler(signal)
    // }
}
