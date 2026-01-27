//! Feature database for bash syntax compatibility analysis
//!
//! Defines 30+ bash syntax features categorized by their compatibility level.

use std::collections::HashMap;

/// Category of a bash feature
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeatureCategory {
    /// POSIX shell standard features
    Posix,
    /// Bash-specific extensions
    BashSpecific,
    /// Zsh-specific extensions
    ZshSpecific,
}

impl std::fmt::Display for FeatureCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeatureCategory::Posix => write!(f, "POSIX"),
            FeatureCategory::BashSpecific => write!(f, "Bash-specific"),
            FeatureCategory::ZshSpecific => write!(f, "Zsh-specific"),
        }
    }
}

/// Bash syntax feature definition
#[derive(Debug, Clone)]
pub struct BashFeature {
    /// Unique identifier for the feature
    pub id: &'static str,
    /// Human-readable name
    pub name: &'static str,
    /// Description of what the feature does
    pub description: &'static str,
    /// Category (POSIX, Bash-specific, Zsh-specific)
    pub category: FeatureCategory,
    /// Example syntax
    pub example: &'static str,
}

/// Get the feature database
pub fn feature_database() -> HashMap<&'static str, BashFeature> {
    let mut db = HashMap::new();

    // POSIX features
    db.insert(
        "simple_command",
        BashFeature {
            id: "simple_command",
            name: "Simple Command",
            description: "Basic command execution (e.g., 'echo hello')",
            category: FeatureCategory::Posix,
            example: "echo hello",
        },
    );

    db.insert(
        "variable_assignment",
        BashFeature {
            id: "variable_assignment",
            name: "Variable Assignment",
            description: "Assign values to variables",
            category: FeatureCategory::Posix,
            example: "var=value",
        },
    );

    db.insert(
        "variable_expansion",
        BashFeature {
            id: "variable_expansion",
            name: "Variable Expansion",
            description: "Expand variable values with $var or ${var}",
            category: FeatureCategory::Posix,
            example: "$var or ${var}",
        },
    );

    db.insert(
        "double_quoted_string",
        BashFeature {
            id: "double_quoted_string",
            name: "Double Quoted String",
            description: "Strings with variable and command substitution",
            category: FeatureCategory::Posix,
            example: "\"$var and $(cmd)\"",
        },
    );

    db.insert(
        "single_quoted_string",
        BashFeature {
            id: "single_quoted_string",
            name: "Single Quoted String",
            description: "Literal strings, no substitution",
            category: FeatureCategory::Posix,
            example: "'literal string'",
        },
    );

    db.insert(
        "command_substitution",
        BashFeature {
            id: "command_substitution",
            name: "Command Substitution",
            description: "Execute command and use its output",
            category: FeatureCategory::Posix,
            example: "$(cmd) or `cmd`",
        },
    );

    db.insert(
        "pipe",
        BashFeature {
            id: "pipe",
            name: "Pipe",
            description: "Connect stdout of one command to stdin of another",
            category: FeatureCategory::Posix,
            example: "cmd1 | cmd2",
        },
    );

    db.insert(
        "redirection_input",
        BashFeature {
            id: "redirection_input",
            name: "Input Redirection",
            description: "Redirect stdin from file",
            category: FeatureCategory::Posix,
            example: "cmd < file",
        },
    );

    db.insert(
        "redirection_output",
        BashFeature {
            id: "redirection_output",
            name: "Output Redirection",
            description: "Redirect stdout to file",
            category: FeatureCategory::Posix,
            example: "cmd > file",
        },
    );

    db.insert(
        "redirection_append",
        BashFeature {
            id: "redirection_append",
            name: "Append Redirection",
            description: "Append stdout to file",
            category: FeatureCategory::Posix,
            example: "cmd >> file",
        },
    );

    db.insert(
        "for_loop",
        BashFeature {
            id: "for_loop",
            name: "For Loop",
            description: "Iterate over values",
            category: FeatureCategory::Posix,
            example: "for x in a b c; do echo $x; done",
        },
    );

    db.insert(
        "while_loop",
        BashFeature {
            id: "while_loop",
            name: "While Loop",
            description: "Loop while condition is true",
            category: FeatureCategory::Posix,
            example: "while true; do echo hi; done",
        },
    );

    db.insert(
        "if_statement",
        BashFeature {
            id: "if_statement",
            name: "If Statement",
            description: "Conditional execution",
            category: FeatureCategory::Posix,
            example: "if cmd; then echo yes; fi",
        },
    );

    db.insert(
        "function_def",
        BashFeature {
            id: "function_def",
            name: "Function Definition",
            description: "Define reusable functions",
            category: FeatureCategory::Posix,
            example: "func() { echo hello; }",
        },
    );

    db.insert(
        "case_statement",
        BashFeature {
            id: "case_statement",
            name: "Case Statement",
            description: "Multi-way branch on pattern matching",
            category: FeatureCategory::Posix,
            example: "case $x in a) echo a;; b) echo b;; esac",
        },
    );

    db.insert(
        "heredoc",
        BashFeature {
            id: "heredoc",
            name: "Heredoc",
            description: "Multi-line string input",
            category: FeatureCategory::Posix,
            example: "cat << EOF\ntext\nEOF",
        },
    );

    // Bash-specific features
    db.insert(
        "parameter_expansion",
        BashFeature {
            id: "parameter_expansion",
            name: "Parameter Expansion",
            description: "Advanced variable expansion (${var:-default})",
            category: FeatureCategory::BashSpecific,
            example: "${var:-default}",
        },
    );

    db.insert(
        "array_variables",
        BashFeature {
            id: "array_variables",
            name: "Array Variables",
            description: "Indexed arrays arr=(a b c)",
            category: FeatureCategory::BashSpecific,
            example: "arr=(a b c); echo ${arr[0]}",
        },
    );

    db.insert(
        "associative_arrays",
        BashFeature {
            id: "associative_arrays",
            name: "Associative Arrays",
            description: "Hash maps declare -A map; map[key]=value",
            category: FeatureCategory::BashSpecific,
            example: "declare -A map; map[key]=value",
        },
    );

    db.insert(
        "process_substitution",
        BashFeature {
            id: "process_substitution",
            name: "Process Substitution",
            description: "Treat command output as file <(cmd)",
            category: FeatureCategory::BashSpecific,
            example: "diff <(cmd1) <(cmd2)",
        },
    );

    db.insert(
        "extended_globbing",
        BashFeature {
            id: "extended_globbing",
            name: "Extended Globbing",
            description: "Advanced pathname expansion patterns",
            category: FeatureCategory::BashSpecific,
            example: "?(pattern) *(pattern) +(pattern)",
        },
    );

    db.insert(
        "test_operator",
        BashFeature {
            id: "test_operator",
            name: "Test Operator",
            description: "Bash [[ ]] test operator with regex",
            category: FeatureCategory::BashSpecific,
            example: "[[ $var =~ regex ]]",
        },
    );

    db.insert(
        "arithmetic_expansion",
        BashFeature {
            id: "arithmetic_expansion",
            name: "Arithmetic Expansion",
            description: "Evaluate arithmetic expressions $((2+2))",
            category: FeatureCategory::BashSpecific,
            example: "$((2+2))",
        },
    );

    db.insert(
        "arithmetic_condition",
        BashFeature {
            id: "arithmetic_condition",
            name: "Arithmetic Condition",
            description: "Conditional arithmetic (( a > b ))",
            category: FeatureCategory::BashSpecific,
            example: "(( a > b ))",
        },
    );

    db.insert(
        "background_execution",
        BashFeature {
            id: "background_execution",
            name: "Background Execution",
            description: "Run command in background with &",
            category: FeatureCategory::BashSpecific,
            example: "cmd &",
        },
    );

    db.insert(
        "until_loop",
        BashFeature {
            id: "until_loop",
            name: "Until Loop",
            description: "Loop until condition is true",
            category: FeatureCategory::BashSpecific,
            example: "until false; do echo hi; done",
        },
    );

    db.insert(
        "conditional_and",
        BashFeature {
            id: "conditional_and",
            name: "Conditional AND",
            description: "Execute right if left succeeds (&&)",
            category: FeatureCategory::BashSpecific,
            example: "cmd1 && cmd2",
        },
    );

    db.insert(
        "conditional_or",
        BashFeature {
            id: "conditional_or",
            name: "Conditional OR",
            description: "Execute right if left fails (||)",
            category: FeatureCategory::BashSpecific,
            example: "cmd1 || cmd2",
        },
    );

    db.insert(
        "subshell",
        BashFeature {
            id: "subshell",
            name: "Subshell",
            description: "Execute commands in subshell (cmd)",
            category: FeatureCategory::BashSpecific,
            example: "(cmd1; cmd2)",
        },
    );

    db.insert(
        "string_concatenation",
        BashFeature {
            id: "string_concatenation",
            name: "String Concatenation",
            description: "Implicit string concatenation",
            category: FeatureCategory::BashSpecific,
            example: "\"hello\"world or $var$other",
        },
    );

    db.insert(
        "word_splitting",
        BashFeature {
            id: "word_splitting",
            name: "Word Splitting",
            description: "Split variables on whitespace",
            category: FeatureCategory::BashSpecific,
            example: "$var (unquoted)",
        },
    );

    db.insert(
        "glob_expansion",
        BashFeature {
            id: "glob_expansion",
            name: "Glob Expansion",
            description: "Pathname expansion with * ? [ ]",
            category: FeatureCategory::BashSpecific,
            example: "*.txt or dir/**/*",
        },
    );

    db.insert(
        "declare_command",
        BashFeature {
            id: "declare_command",
            name: "Declare Command",
            description: "Declare variables with attributes",
            category: FeatureCategory::BashSpecific,
            example: "declare -r VAR=value",
        },
    );

    db.insert(
        "export_command",
        BashFeature {
            id: "export_command",
            name: "Export Command",
            description: "Export variables to environment",
            category: FeatureCategory::BashSpecific,
            example: "export VAR=value",
        },
    );

    db.insert(
        "local_command",
        BashFeature {
            id: "local_command",
            name: "Local Command",
            description: "Create local variables in functions",
            category: FeatureCategory::BashSpecific,
            example: "local var=value",
        },
    );

    db.insert(
        "readonly_command",
        BashFeature {
            id: "readonly_command",
            name: "Readonly Command",
            description: "Make variables read-only",
            category: FeatureCategory::BashSpecific,
            example: "readonly VAR=value",
        },
    );

    db.insert(
        "unset_command",
        BashFeature {
            id: "unset_command",
            name: "Unset Command",
            description: "Unset variables or functions",
            category: FeatureCategory::BashSpecific,
            example: "unset VAR",
        },
    );

    db.insert(
        "error_handling",
        BashFeature {
            id: "error_handling",
            name: "Error Handling",
            description: "Check exit status with $?",
            category: FeatureCategory::BashSpecific,
            example: "echo $?",
        },
    );

    db.insert(
        "positional_parameters",
        BashFeature {
            id: "positional_parameters",
            name: "Positional Parameters",
            description: "Access command arguments $1, $2, $@",
            category: FeatureCategory::BashSpecific,
            example: "$1, $@, $#",
        },
    );

    db.insert(
        "special_parameters",
        BashFeature {
            id: "special_parameters",
            name: "Special Parameters",
            description: "Special variables like $?, $$, $!",
            category: FeatureCategory::BashSpecific,
            example: "$?, $$, $!, $0",
        },
    );

    db.insert(
        "redirect_stderr",
        BashFeature {
            id: "redirect_stderr",
            name: "Redirect Stderr",
            description: "Redirect stderr to file or stdout",
            category: FeatureCategory::BashSpecific,
            example: "cmd 2> file or cmd 2>&1",
        },
    );

    db.insert(
        "redirect_both",
        BashFeature {
            id: "redirect_both",
            name: "Redirect Both Streams",
            description: "Redirect both stdout and stderr",
            category: FeatureCategory::BashSpecific,
            example: "cmd &> file or cmd > file 2>&1",
        },
    );

    // Zsh-specific features
    db.insert(
        "floating_point_math",
        BashFeature {
            id: "floating_point_math",
            name: "Floating Point Math",
            description: "Native floating point arithmetic",
            category: FeatureCategory::ZshSpecific,
            example: "echo $((1.5 + 2.3))",
        },
    );

    db.insert(
        "builtin_functions",
        BashFeature {
            id: "builtin_functions",
            name: "Builtin Functions",
            description: "Zsh-specific builtin functions",
            category: FeatureCategory::ZshSpecific,
            example: "emulate, disable, enable",
        },
    );

    db
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_database() {
        let db = feature_database();
        assert!(db.len() >= 30, "Database should have at least 30 features");
    }

    #[test]
    fn test_posix_features_exist() {
        let db = feature_database();
        assert!(db.contains_key("simple_command"));
        assert!(db.contains_key("for_loop"));
        assert!(db.contains_key("if_statement"));
        assert!(db.contains_key("heredoc"));
    }

    #[test]
    fn test_bash_specific_features_exist() {
        let db = feature_database();
        assert!(db.contains_key("array_variables"));
        assert!(db.contains_key("process_substitution"));
        assert!(db.contains_key("arithmetic_expansion"));
    }

    #[test]
    fn test_feature_categories() {
        let db = feature_database();
        let mut posix_count = 0;
        let mut bash_count = 0;
        let mut zsh_count = 0;

        for feature in db.values() {
            match feature.category {
                FeatureCategory::Posix => posix_count += 1,
                FeatureCategory::BashSpecific => bash_count += 1,
                FeatureCategory::ZshSpecific => zsh_count += 1,
            }
        }

        assert!(posix_count > 0, "Should have POSIX features");
        assert!(bash_count > 0, "Should have Bash-specific features");
        assert!(zsh_count > 0, "Should have Zsh-specific features");
    }
}
