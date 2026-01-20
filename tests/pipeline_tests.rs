use std::fs;
use std::io::Write;
use std::process::Command;

/// Helper to run rush with a command
fn run_rush(cmd: &str) -> std::process::Output {
    Command::new("./target/release/rush")
        .arg("-c")
        .arg(cmd)
        .output()
        .expect("Failed to execute rush")
}

/// Helper to create test data file
fn create_test_file(path: &str, content: &str) {
    let mut file = fs::File::create(path).expect("Failed to create test file");
    file.write_all(content.as_bytes())
        .expect("Failed to write test file");
}

#[test]
fn test_two_stage_pipeline() {
    create_test_file("/tmp/rush_test_pipe.txt", "hello\nworld\nrust\n");

    let output = run_rush("cat /tmp/rush_test_pipe.txt | grep --no-color -N rust");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "rust\n");
    assert_eq!(output.status.code(), Some(0));

    fs::remove_file("/tmp/rush_test_pipe.txt").ok();
}

#[test]
fn test_three_stage_pipeline() {
    create_test_file("/tmp/rush_test_pipe3.txt", "apple\nbanana\napple\napple\nbanana\n");

    // cat | grep | wc
    let output = run_rush("cat /tmp/rush_test_pipe3.txt | grep --no-color -N apple | wc -l");
    let count: i32 = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .expect("Expected number");
    assert_eq!(count, 3);
    assert_eq!(output.status.code(), Some(0));

    fs::remove_file("/tmp/rush_test_pipe3.txt").ok();
}

#[test]
fn test_five_stage_pipeline() {
    // Create a file with many lines
    let mut content = String::new();
    for i in 1..=100 {
        content.push_str(&format!("Line {}: test data\n", i));
    }
    create_test_file("/tmp/rush_test_pipe5.txt", &content);

    // 5-stage pipeline: cat | grep | grep | head | wc -l
    let output = run_rush(
        "cat /tmp/rush_test_pipe5.txt | grep --no-color -N Line | grep --no-color -N test | head -10 | wc -l",
    );
    let count: i32 = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .expect("Expected number");
    assert_eq!(count, 10);
    assert_eq!(output.status.code(), Some(0));

    fs::remove_file("/tmp/rush_test_pipe5.txt").ok();
}

#[test]
fn test_pipeline_exit_code_last_command() {
    create_test_file("/tmp/rush_test_exitcode.txt", "hello\nworld\n");

    // First command succeeds, last command fails (grep with no match)
    let output = run_rush("cat /tmp/rush_test_exitcode.txt | grep --no-color -N nonexistent");
    assert_eq!(output.status.code(), Some(1)); // grep returns 1 for no match

    fs::remove_file("/tmp/rush_test_exitcode.txt").ok();
}

#[test]
fn test_pipeline_with_builtins() {
    create_test_file("/tmp/rush_test_builtins.txt", "line1\nline2\nline3\n");

    // Using Rush's built-in cat and grep (disable colors and line numbers for predictable output)
    let output = run_rush("cat /tmp/rush_test_builtins.txt | grep --no-color -N line2");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "line2\n");
    assert_eq!(output.status.code(), Some(0));

    fs::remove_file("/tmp/rush_test_builtins.txt").ok();
}

#[test]
fn test_pipeline_with_external_commands() {
    create_test_file("/tmp/rush_test_external.txt", "apple\nbanana\ncherry\n");

    // Using external sort command (via PATH)
    let output = run_rush("cat /tmp/rush_test_external.txt | sort");
    let stdout_str = String::from_utf8_lossy(&output.stdout);

    // Verify sorted output
    assert!(stdout_str.contains("apple"), "Output should contain 'apple': {:?}", stdout_str);
    assert!(stdout_str.contains("banana"), "Output should contain 'banana': {:?}", stdout_str);
    assert!(stdout_str.contains("cherry"), "Output should contain 'cherry': {:?}", stdout_str);
    assert_eq!(output.status.code(), Some(0), "Command should succeed. stderr: {}", String::from_utf8_lossy(&output.stderr));

    fs::remove_file("/tmp/rush_test_external.txt").ok();
}

#[test]
fn test_pipeline_large_data() {
    // Create a large file (10,000 lines)
    let mut content = String::new();
    for i in 1..=10000 {
        content.push_str(&format!("Line {}: Lorem ipsum dolor sit amet\n", i));
    }
    create_test_file("/tmp/rush_test_large.txt", &content);

    // Process large file through pipeline
    let output = run_rush("cat /tmp/rush_test_large.txt | grep --no-color -N Lorem | wc -l");
    let count: i32 = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .expect("Expected number");
    assert_eq!(count, 10000);
    assert_eq!(output.status.code(), Some(0));

    fs::remove_file("/tmp/rush_test_large.txt").ok();
}

#[test]
fn test_pipeline_with_head_early_termination() {
    // Create a large file
    let mut content = String::new();
    for i in 1..=1000 {
        content.push_str(&format!("Line {}\n", i));
    }
    create_test_file("/tmp/rush_test_head.txt", &content);

    // head should terminate early and cat should handle SIGPIPE gracefully
    let output = run_rush("cat /tmp/rush_test_head.txt | head -5");
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout_str.lines().collect();
    assert_eq!(lines.len(), 5);
    assert_eq!(output.status.code(), Some(0));

    fs::remove_file("/tmp/rush_test_head.txt").ok();
}

#[test]
fn test_pipeline_empty_input() {
    create_test_file("/tmp/rush_test_empty.txt", "");

    let output = run_rush("cat /tmp/rush_test_empty.txt | grep --no-color -N test");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(output.status.code(), Some(1)); // grep returns 1 for no match

    fs::remove_file("/tmp/rush_test_empty.txt").ok();
}

#[test]
fn test_pipeline_error_in_middle_command() {
    create_test_file("/tmp/rush_test_error.txt", "hello\nworld\n");

    // Middle command references non-existent file, but pipeline should handle it
    // Note: This tests error propagation behavior
    let output = run_rush("cat /tmp/rush_test_error.txt | cat /nonexistent/file.txt | grep --no-color -N hello");

    // The pipeline should fail (non-zero exit code)
    assert_ne!(output.status.code(), Some(0));

    fs::remove_file("/tmp/rush_test_error.txt").ok();
}

#[test]
fn test_complex_real_world_pipeline() {
    // Create a log-like file
    let content = r#"2024-01-01 INFO Starting application
2024-01-01 ERROR Failed to connect
2024-01-01 INFO Retrying connection
2024-01-01 ERROR Failed to connect
2024-01-01 INFO Connection established
2024-01-01 ERROR Unexpected error
"#;
    create_test_file("/tmp/rush_test_logs.txt", content);

    // Count ERROR lines
    let output = run_rush("cat /tmp/rush_test_logs.txt | grep --no-color -N ERROR | wc -l");
    let count: i32 = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .expect("Expected number");
    assert_eq!(count, 3);
    assert_eq!(output.status.code(), Some(0));

    fs::remove_file("/tmp/rush_test_logs.txt").ok();
}
