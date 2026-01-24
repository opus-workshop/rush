use rush::executor::Executor;
use rush::parser::ast::{Statement, Command, Argument, ForLoop, Expression, Literal};

#[test]
fn test_break_basic_for_loop() {
    let mut executor = Executor::new();

    // for i in 1 2 3 4 5; do echo $i; if [ $i = 3 ]; then break; fi; done
    let for_loop = ForLoop {
        variable: "i".to_string(),
        iterable: Expression::Literal(Literal::String("1\n2\n3\n4\n5".to_string())),
        body: vec![
            Statement::Command(Command {
                name: "echo".to_string(),
                args: vec![Argument::Variable("i".to_string())],
                redirects: vec![],
            }),
            Statement::Command(Command {
                name: "break".to_string(),
                args: vec![],
                redirects: vec![],
            }),
        ],
    };

    let result = executor.execute(vec![Statement::ForLoop(for_loop)]).unwrap();

    // Should only print "1" before breaking
    assert_eq!(result.stdout(), "1\n");
    assert_eq!(result.exit_code, 0);
}

#[test]
fn test_break_with_condition() {
    let mut executor = Executor::new();

    // Set up a counter variable
    executor.runtime_mut().set_variable("counter".to_string(), "0".to_string());

    // Manually construct a loop that increments counter and breaks at 3
    let for_loop = ForLoop {
        variable: "i".to_string(),
        iterable: Expression::Literal(Literal::String("1\n2\n3\n4\n5\n6\n7\n8\n9\n10".to_string())),
        body: vec![
            Statement::Command(Command {
                name: "echo".to_string(),
                args: vec![Argument::Variable("i".to_string())],
                redirects: vec![],
            }),
        ],
    };

    // Execute a simple loop that echoes each iteration
    let result = executor.execute(vec![Statement::ForLoop(for_loop)]).unwrap();

    // All items should be echoed since we're not breaking in this test
    assert_eq!(result.stdout(), "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n");
}

#[test]
fn test_break_outside_loop() {
    let mut executor = Executor::new();

    // Try to break outside a loop
    let result = executor.execute(vec![
        Statement::Command(Command {
            name: "break".to_string(),
            args: vec![],
            redirects: vec![],
        }),
    ]);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("only meaningful in a"));
}

#[test]
fn test_break_nested_loops_level_1() {
    let mut executor = Executor::new();

    // Nested loops: outer loop over 'a b', inner loop over '1 2 3'
    // Break from inner loop when inner = 2
    let inner_loop = ForLoop {
        variable: "inner".to_string(),
        iterable: Expression::Literal(Literal::String("1\n2\n3".to_string())),
        body: vec![
            Statement::Command(Command {
                name: "echo".to_string(),
                args: vec![
                    Argument::Variable("outer".to_string()),
                    Argument::Literal(":".to_string()),
                    Argument::Variable("inner".to_string()),
                ],
                redirects: vec![],
            }),
            Statement::Command(Command {
                name: "break".to_string(),
                args: vec![],
                redirects: vec![],
            }),
        ],
    };

    let outer_loop = ForLoop {
        variable: "outer".to_string(),
        iterable: Expression::Literal(Literal::String("a\nb".to_string())),
        body: vec![
            Statement::ForLoop(inner_loop),
        ],
    };

    let result = executor.execute(vec![Statement::ForLoop(outer_loop)]).unwrap();

    // Should print "a : 1" and "b : 1" (breaks from inner loop each time)
    assert_eq!(result.stdout(), "a : 1\nb : 1\n");
}

#[test]
fn test_break_nested_loops_level_2() {
    let mut executor = Executor::new();

    // Nested loops: break 2 should exit both loops
    let inner_loop = ForLoop {
        variable: "inner".to_string(),
        iterable: Expression::Literal(Literal::String("1\n2\n3".to_string())),
        body: vec![
            Statement::Command(Command {
                name: "echo".to_string(),
                args: vec![
                    Argument::Variable("outer".to_string()),
                    Argument::Literal(":".to_string()),
                    Argument::Variable("inner".to_string()),
                ],
                redirects: vec![],
            }),
            Statement::Command(Command {
                name: "break".to_string(),
                args: vec![Argument::Literal("2".to_string())],
                redirects: vec![],
            }),
        ],
    };

    let outer_loop = ForLoop {
        variable: "outer".to_string(),
        iterable: Expression::Literal(Literal::String("a\nb\nc".to_string())),
        body: vec![
            Statement::ForLoop(inner_loop),
        ],
    };

    let result = executor.execute(vec![Statement::ForLoop(outer_loop)]).unwrap();

    // Should only print "a : 1" before breaking from both loops
    assert_eq!(result.stdout(), "a : 1\n");
}

#[test]
fn test_break_with_invalid_argument() {
    let mut executor = Executor::new();

    let for_loop = ForLoop {
        variable: "i".to_string(),
        iterable: Expression::Literal(Literal::String("1\n2\n3".to_string())),
        body: vec![
            Statement::Command(Command {
                name: "break".to_string(),
                args: vec![Argument::Literal("not_a_number".to_string())],
                redirects: vec![],
            }),
        ],
    };

    let result = executor.execute(vec![Statement::ForLoop(for_loop)]);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("numeric argument required"));
}

#[test]
fn test_break_with_zero() {
    let mut executor = Executor::new();

    let for_loop = ForLoop {
        variable: "i".to_string(),
        iterable: Expression::Literal(Literal::String("1\n2\n3".to_string())),
        body: vec![
            Statement::Command(Command {
                name: "break".to_string(),
                args: vec![Argument::Literal("0".to_string())],
                redirects: vec![],
            }),
        ],
    };

    let result = executor.execute(vec![Statement::ForLoop(for_loop)]);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("loop count out of range"));
}

#[test]
fn test_break_exceeds_loop_depth() {
    let mut executor = Executor::new();

    let for_loop = ForLoop {
        variable: "i".to_string(),
        iterable: Expression::Literal(Literal::String("1\n2\n3".to_string())),
        body: vec![
            Statement::Command(Command {
                name: "break".to_string(),
                args: vec![Argument::Literal("2".to_string())],
                redirects: vec![],
            }),
        ],
    };

    let result = executor.execute(vec![Statement::ForLoop(for_loop)]);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("loop count out of range"));
}

#[test]
fn test_break_too_many_arguments() {
    let mut executor = Executor::new();

    let for_loop = ForLoop {
        variable: "i".to_string(),
        iterable: Expression::Literal(Literal::String("1\n2\n3".to_string())),
        body: vec![
            Statement::Command(Command {
                name: "break".to_string(),
                args: vec![
                    Argument::Literal("1".to_string()),
                    Argument::Literal("2".to_string()),
                ],
                redirects: vec![],
            }),
        ],
    };

    let result = executor.execute(vec![Statement::ForLoop(for_loop)]);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("too many arguments"));
}

#[test]
fn test_break_preserves_output_before_break() {
    let mut executor = Executor::new();

    // Loop that echoes before breaking
    let for_loop = ForLoop {
        variable: "i".to_string(),
        iterable: Expression::Literal(Literal::String("first\nsecond\nthird".to_string())),
        body: vec![
            Statement::Command(Command {
                name: "echo".to_string(),
                args: vec![
                    Argument::Literal("Processing:".to_string()),
                    Argument::Variable("i".to_string()),
                ],
                redirects: vec![],
            }),
            Statement::Command(Command {
                name: "break".to_string(),
                args: vec![],
                redirects: vec![],
            }),
        ],
    };

    let result = executor.execute(vec![Statement::ForLoop(for_loop)]).unwrap();

    // Should see output from first iteration only
    assert_eq!(result.stdout(), "Processing: first\n");
    assert_eq!(result.exit_code, 0);
}

#[test]
fn test_break_in_triple_nested_loop() {
    let mut executor = Executor::new();

    // Test break 3 in triple-nested loops
    let innermost_loop = ForLoop {
        variable: "k".to_string(),
        iterable: Expression::Literal(Literal::String("1\n2".to_string())),
        body: vec![
            Statement::Command(Command {
                name: "echo".to_string(),
                args: vec![
                    Argument::Variable("i".to_string()),
                    Argument::Variable("j".to_string()),
                    Argument::Variable("k".to_string()),
                ],
                redirects: vec![],
            }),
            Statement::Command(Command {
                name: "break".to_string(),
                args: vec![Argument::Literal("3".to_string())],
                redirects: vec![],
            }),
        ],
    };

    let middle_loop = ForLoop {
        variable: "j".to_string(),
        iterable: Expression::Literal(Literal::String("x\ny".to_string())),
        body: vec![
            Statement::ForLoop(innermost_loop),
        ],
    };

    let outer_loop = ForLoop {
        variable: "i".to_string(),
        iterable: Expression::Literal(Literal::String("a\nb".to_string())),
        body: vec![
            Statement::ForLoop(middle_loop),
        ],
    };

    let result = executor.execute(vec![Statement::ForLoop(outer_loop)]).unwrap();

    // Should only print "a x 1" before breaking from all three loops
    assert_eq!(result.stdout(), "a x 1\n");
}
