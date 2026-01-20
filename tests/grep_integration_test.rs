use rush::builtins::Builtins;
use rush::runtime::Runtime;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_grep_basic_search() {
    let tmp = TempDir::new().unwrap();
    let file_path = tmp.path().join("test.txt");
    fs::write(&file_path, "hello world\nfoo bar\nhello rust\n").unwrap();

    let mut runtime = Runtime::new();
    runtime.set_cwd(tmp.path().to_path_buf());

    let builtins = Builtins::new();
    let args = vec![
        "hello".to_string(),
        file_path.to_string_lossy().to_string(),
    ];

    let result = builtins.execute("grep", args, &mut runtime).unwrap();
    assert_eq!(result.exit_code, 0);
    // Output includes line numbers (1:, 3:) and ANSI color codes
    assert!(result.stdout.contains("1:") && result.stdout.contains("hello") && result.stdout.contains("world"));
    assert!(result.stdout.contains("3:") && result.stdout.contains("hello") && result.stdout.contains("rust"));
    // "foo bar" should not appear in results
    assert!(!result.stdout.contains("foo") || !result.stdout.contains("bar"));
}

#[test]
fn test_grep_case_insensitive() {
    let tmp = TempDir::new().unwrap();
    let file_path = tmp.path().join("test.txt");
    fs::write(&file_path, "HELLO world\nfoo BAR\nhello rust\n").unwrap();

    let mut runtime = Runtime::new();
    runtime.set_cwd(tmp.path().to_path_buf());

    let builtins = Builtins::new();
    let args = vec![
        "-i".to_string(),
        "hello".to_string(),
        file_path.to_string_lossy().to_string(),
    ];

    let result = builtins.execute("grep", args, &mut runtime).unwrap();
    assert_eq!(result.exit_code, 0);
    assert!(result.stdout.contains("HELLO") && result.stdout.contains("world"));
    assert!(result.stdout.contains("hello") && result.stdout.contains("rust"));
}

#[test]
fn test_grep_recursive() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("subdir")).unwrap();

    let file1 = tmp.path().join("file1.txt");
    let file2 = tmp.path().join("subdir").join("file2.txt");

    fs::write(&file1, "hello world\n").unwrap();
    fs::write(&file2, "hello rust\n").unwrap();

    let mut runtime = Runtime::new();
    runtime.set_cwd(tmp.path().to_path_buf());

    let builtins = Builtins::new();
    let args = vec![
        "-r".to_string(),
        "hello".to_string(),
        tmp.path().to_string_lossy().to_string(),
    ];

    let result = builtins.execute("grep", args, &mut runtime).unwrap();
    assert_eq!(result.exit_code, 0);
    assert!(result.stdout.contains("hello") && result.stdout.contains("world"));
    assert!(result.stdout.contains("hello") && result.stdout.contains("rust"));
}

#[test]
fn test_grep_invert_match() {
    let tmp = TempDir::new().unwrap();
    let file_path = tmp.path().join("test.txt");
    fs::write(&file_path, "hello world\nfoo bar\nhello rust\n").unwrap();

    let mut runtime = Runtime::new();
    runtime.set_cwd(tmp.path().to_path_buf());

    let builtins = Builtins::new();
    let args = vec![
        "-v".to_string(),
        "hello".to_string(),
        file_path.to_string_lossy().to_string(),
    ];

    let result = builtins.execute("grep", args, &mut runtime).unwrap();
    assert_eq!(result.exit_code, 0);
    // Invert match: should only show lines WITHOUT "hello"
    assert!(!result.stdout.contains("hello"));
    assert!(result.stdout.contains("foo") && result.stdout.contains("bar"));
}

#[test]
fn test_grep_no_match() {
    let tmp = TempDir::new().unwrap();
    let file_path = tmp.path().join("test.txt");
    fs::write(&file_path, "hello world\nfoo bar\n").unwrap();

    let mut runtime = Runtime::new();
    runtime.set_cwd(tmp.path().to_path_buf());

    let builtins = Builtins::new();
    let args = vec![
        "nonexistent".to_string(),
        file_path.to_string_lossy().to_string(),
    ];

    let result = builtins.execute("grep", args, &mut runtime).unwrap();
    assert_eq!(result.exit_code, 1);
    assert!(result.stdout.is_empty());
}
