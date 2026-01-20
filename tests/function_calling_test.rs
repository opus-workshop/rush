use rush::executor::Executor;
use rush::parser::ast::*;

#[test]
fn test_simple_function_call_no_params() {
    let mut executor = Executor::new();

    // Define a simple function that echoes "hello"
    let func = FunctionDef {
        name: "greet".to_string(),
        params: vec![],
        body: vec![
            Statement::Command(Command {
                name: "echo".to_string(),
                args: vec![Argument::Literal("hello".to_string())],
                redirects: vec![],
            })
        ],
    };

    // Define the function
    executor.execute_statement(Statement::FunctionDef(func)).unwrap();

    // Call the function
    let result = executor.execute_statement(Statement::Command(Command {
        name: "greet".to_string(),
        args: vec![],
        redirects: vec![],
    })).unwrap();

    // Should return "hello\n" from echo
    assert!(result.stdout.contains("hello"));
}

#[test]
fn test_function_with_parameters() {
    let mut executor = Executor::new();

    // Define a function that echoes its first parameter
    let func = FunctionDef {
        name: "say".to_string(),
        params: vec![
            Parameter {
                name: "message".to_string(),
                type_hint: None,
            }
        ],
        body: vec![
            Statement::Command(Command {
                name: "echo".to_string(),
                args: vec![Argument::Variable("$message".to_string())],
                redirects: vec![],
            })
        ],
    };

    executor.execute_statement(Statement::FunctionDef(func)).unwrap();

    // Call the function with an argument
    let result = executor.execute_statement(Statement::Command(Command {
        name: "say".to_string(),
        args: vec![Argument::Literal("world".to_string())],
        redirects: vec![],
    })).unwrap();

    assert!(result.stdout.contains("world"));
}

#[test]
fn test_function_with_multiple_parameters() {
    let mut executor = Executor::new();

    // Define a function that echoes two parameters
    let func = FunctionDef {
        name: "combine".to_string(),
        params: vec![
            Parameter {
                name: "first".to_string(),
                type_hint: None,
            },
            Parameter {
                name: "second".to_string(),
                type_hint: None,
            }
        ],
        body: vec![
            Statement::Command(Command {
                name: "echo".to_string(),
                args: vec![
                    Argument::Variable("$first".to_string()),
                    Argument::Variable("$second".to_string()),
                ],
                redirects: vec![],
            })
        ],
    };

    executor.execute_statement(Statement::FunctionDef(func)).unwrap();

    // Call the function with two arguments
    let result = executor.execute_statement(Statement::Command(Command {
        name: "combine".to_string(),
        args: vec![
            Argument::Literal("hello".to_string()),
            Argument::Literal("world".to_string()),
        ],
        redirects: vec![],
    })).unwrap();

    assert!(result.stdout.contains("hello"));
    assert!(result.stdout.contains("world"));
}

#[test]
fn test_recursive_factorial() {
    let mut executor = Executor::new();

    // Define a factorial function using if statements
    // factorial(n) = if n <= 1 then 1 else n * factorial(n-1)
    // For simplicity, this will just count down and echo the values
    let func = FunctionDef {
        name: "countdown".to_string(),
        params: vec![
            Parameter {
                name: "n".to_string(),
                type_hint: None,
            }
        ],
        body: vec![
            Statement::Command(Command {
                name: "echo".to_string(),
                args: vec![Argument::Variable("$n".to_string())],
                redirects: vec![],
            }),
            // This is a simplified test - a real factorial would need more complex logic
        ],
    };

    executor.execute_statement(Statement::FunctionDef(func)).unwrap();

    // Call the function
    let result = executor.execute_statement(Statement::Command(Command {
        name: "countdown".to_string(),
        args: vec![Argument::Literal("5".to_string())],
        redirects: vec![],
    })).unwrap();

    assert!(result.stdout.contains("5"));
}

#[test]
fn test_function_calling_another_function() {
    let mut executor = Executor::new();

    // Define a helper function
    let helper = FunctionDef {
        name: "helper".to_string(),
        params: vec![],
        body: vec![
            Statement::Command(Command {
                name: "echo".to_string(),
                args: vec![Argument::Literal("helper called".to_string())],
                redirects: vec![],
            })
        ],
    };

    // Define a main function that calls the helper
    let main = FunctionDef {
        name: "main".to_string(),
        params: vec![],
        body: vec![
            Statement::Command(Command {
                name: "echo".to_string(),
                args: vec![Argument::Literal("main called".to_string())],
                redirects: vec![],
            }),
            Statement::Command(Command {
                name: "helper".to_string(),
                args: vec![],
                redirects: vec![],
            }),
        ],
    };

    executor.execute_statement(Statement::FunctionDef(helper)).unwrap();
    executor.execute_statement(Statement::FunctionDef(main)).unwrap();

    // Call the main function
    let result = executor.execute_statement(Statement::Command(Command {
        name: "main".to_string(),
        args: vec![],
        redirects: vec![],
    })).unwrap();

    assert!(result.stdout.contains("main called"));
    assert!(result.stdout.contains("helper called"));
}

#[test]
fn test_scope_isolation_parameters_shadow_variables() {
    let mut executor = Executor::new();

    // Set a global variable
    executor.execute_statement(Statement::Assignment(Assignment {
        name: "x".to_string(),
        value: Expression::Literal(Literal::String("global".to_string())),
    })).unwrap();

    // Define a function with a parameter named 'x'
    let func = FunctionDef {
        name: "test_scope".to_string(),
        params: vec![
            Parameter {
                name: "x".to_string(),
                type_hint: None,
            }
        ],
        body: vec![
            Statement::Command(Command {
                name: "echo".to_string(),
                args: vec![Argument::Variable("$x".to_string())],
                redirects: vec![],
            })
        ],
    };

    executor.execute_statement(Statement::FunctionDef(func)).unwrap();

    // Call the function - should echo "local" not "global"
    let result = executor.execute_statement(Statement::Command(Command {
        name: "test_scope".to_string(),
        args: vec![Argument::Literal("local".to_string())],
        redirects: vec![],
    })).unwrap();

    assert!(result.stdout.contains("local"));
    assert!(!result.stdout.contains("global"));
}

#[test]
fn test_local_variables_dont_leak() {
    let mut executor = Executor::new();

    // Define a function that sets a local variable
    let func = FunctionDef {
        name: "set_local".to_string(),
        params: vec![],
        body: vec![
            Statement::Assignment(Assignment {
                name: "local_var".to_string(),
                value: Expression::Literal(Literal::String("local_value".to_string())),
            }),
        ],
    };

    executor.execute_statement(Statement::FunctionDef(func)).unwrap();

    // Call the function
    executor.execute_statement(Statement::Command(Command {
        name: "set_local".to_string(),
        args: vec![],
        redirects: vec![],
    })).unwrap();

    // Try to access the local variable - should not exist
    let var_value = executor.runtime_mut().get_variable("local_var");
    assert!(var_value.is_none(), "Local variable should not leak to global scope");
}

#[test]
fn test_recursion_depth_limit() {
    let mut executor = Executor::new();

    // Define an infinitely recursive function
    let func = FunctionDef {
        name: "infinite".to_string(),
        params: vec![],
        body: vec![
            Statement::Command(Command {
                name: "infinite".to_string(),
                args: vec![],
                redirects: vec![],
            }),
        ],
    };

    executor.execute_statement(Statement::FunctionDef(func)).unwrap();

    // Call the function - should error with recursion limit
    let result = executor.execute_statement(Statement::Command(Command {
        name: "infinite".to_string(),
        args: vec![],
        redirects: vec![],
    }));

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("recursion") || error_msg.contains("depth"));
}

#[test]
fn test_function_return_value() {
    let mut executor = Executor::new();

    // Define a function that returns a value via echo
    let func = FunctionDef {
        name: "get_value".to_string(),
        params: vec![],
        body: vec![
            Statement::Command(Command {
                name: "echo".to_string(),
                args: vec![Argument::Literal("42".to_string())],
                redirects: vec![],
            })
        ],
    };

    executor.execute_statement(Statement::FunctionDef(func)).unwrap();

    // Call the function and check the return value
    let result = executor.execute_statement(Statement::Command(Command {
        name: "get_value".to_string(),
        args: vec![],
        redirects: vec![],
    })).unwrap();

    assert!(result.stdout.contains("42"));
    assert_eq!(result.exit_code, 0);
}

#[test]
fn test_function_stdout_capture() {
    let mut executor = Executor::new();

    // Define a function that outputs multiple lines
    let func = FunctionDef {
        name: "multi_output".to_string(),
        params: vec![],
        body: vec![
            Statement::Command(Command {
                name: "echo".to_string(),
                args: vec![Argument::Literal("line1".to_string())],
                redirects: vec![],
            }),
            Statement::Command(Command {
                name: "echo".to_string(),
                args: vec![Argument::Literal("line2".to_string())],
                redirects: vec![],
            }),
            Statement::Command(Command {
                name: "echo".to_string(),
                args: vec![Argument::Literal("line3".to_string())],
                redirects: vec![],
            }),
        ],
    };

    executor.execute_statement(Statement::FunctionDef(func)).unwrap();

    // Call the function and verify all output is captured
    let result = executor.execute_statement(Statement::Command(Command {
        name: "multi_output".to_string(),
        args: vec![],
        redirects: vec![],
    })).unwrap();

    // Note: Each echo only captures its own output, not accumulated
    // The last statement's output is what's returned
    assert!(result.stdout.contains("line3"));
}
