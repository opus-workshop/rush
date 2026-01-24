use rush::executor::Executor;
use rush::parser::ast::{Statement, Command, Argument};

/// Test exec with no arguments is a no-op
#[test]
fn test_exec_no_arguments() {
    let mut executor = Executor::new();

    let cmd = Statement::Command(Command {
        name: "exec".to_string(),
        args: vec![],
        redirects: vec![],
    });

    let result = executor.execute(vec![cmd]).unwrap();
    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout(), "");
}

/// Test exec with builtin command fails
#[test]
fn test_exec_builtin_error() {
    let mut executor = Executor::new();

    // exec cd /tmp should fail because cd is a builtin
    let cmd = Statement::Command(Command {
        name: "exec".to_string(),
        args: vec![
            Argument::Literal("cd".to_string()),
            Argument::Literal("/tmp".to_string()),
        ],
        redirects: vec![],
    });

    let result = executor.execute(vec![cmd]);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("cannot execute builtin"),
            "Expected 'cannot execute builtin' error, got: {}", err_msg);
}

/// Test exec with nonexistent command fails
#[test]
fn test_exec_nonexistent_command() {
    let mut executor = Executor::new();

    let cmd = Statement::Command(Command {
        name: "exec".to_string(),
        args: vec![
            Argument::Literal("nonexistent_command_xyz12345".to_string()),
        ],
        redirects: vec![],
    });

    let result = executor.execute(vec![cmd]);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("command not found"),
            "Expected 'command not found' error, got: {}", err_msg);
}

/// Test exec with absolute path to nonexistent file
#[test]
fn test_exec_absolute_path_not_found() {
    let mut executor = Executor::new();

    let cmd = Statement::Command(Command {
        name: "exec".to_string(),
        args: vec![
            Argument::Literal("/nonexistent/path/to/command".to_string()),
        ],
        redirects: vec![],
    });

    let result = executor.execute(vec![cmd]);
    assert!(result.is_err());
}

/// Test exec errors when trying to execute a builtin (echo)
#[test]
fn test_exec_echo_builtin_error() {
    let mut executor = Executor::new();

    let cmd = Statement::Command(Command {
        name: "exec".to_string(),
        args: vec![
            Argument::Literal("echo".to_string()),
            Argument::Literal("hello".to_string()),
        ],
        redirects: vec![],
    });

    let result = executor.execute(vec![cmd]);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot execute builtin"));
}

/// Test exec errors when trying to execute true builtin
#[test]
fn test_exec_true_builtin_error() {
    let mut executor = Executor::new();

    let cmd = Statement::Command(Command {
        name: "exec".to_string(),
        args: vec![
            Argument::Literal("true".to_string()),
        ],
        redirects: vec![],
    });

    let result = executor.execute(vec![cmd]);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot execute builtin"));
}

/// Test exec errors when trying to execute false builtin
#[test]
fn test_exec_false_builtin_error() {
    let mut executor = Executor::new();

    let cmd = Statement::Command(Command {
        name: "exec".to_string(),
        args: vec![
            Argument::Literal("false".to_string()),
        ],
        redirects: vec![],
    });

    let result = executor.execute(vec![cmd]);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot execute builtin"));
}

// Note: We cannot test actual process replacement in unit tests
// because exec replaces the test process itself and would cause
// the test runner to exit. Actual exec functionality with process
// replacement should be tested manually or via end-to-end tests
// that spawn a separate shell process.
//
// The following behaviors cannot be tested in integration tests:
// - exec echo hello (would replace test process with echo)
// - exec ls (would replace test process with ls)
// - exec with redirections (would affect test process's FDs)
// - exec env FOO=bar printenv FOO (would replace test process)
//
// These behaviors are tested in the unit tests in exec.rs where
// we can test the individual functions without actually calling exec.
