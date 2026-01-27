use crate::executor::{ExecutionResult, Output};
use crate::runtime::Runtime;
use anyhow::{anyhow, Result};
use std::time::Instant;
use libc;

/// The time builtin command
///
/// Measures and reports execution time of a command.
/// Shows real (wall-clock), user (CPU in user mode), and sys (CPU in kernel mode) time.
///
/// Usage:
///   time command [args...]
///   time pipeline
///
/// Output format (POSIX-like):
///   real    0m0.123s
///   user    0m0.100s
///   sys     0m0.020s
///
/// Examples:
///   time echo hello
///   time sleep 0.1
///   time ls | wc -l
pub fn builtin_time(args: &[String], runtime: &mut Runtime) -> Result<ExecutionResult> {
    // Need at least a command to time
    if args.is_empty() {
        return Err(anyhow!("time: usage: time command [args...]"));
    }

    // Join all arguments back into a command string
    let command_string = args.join(" ");

    // Parse and execute the command string
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::executor::Executor;

    // Tokenize the command string
    let tokens = Lexer::tokenize(&command_string)
        .map_err(|e| anyhow!("time: tokenize error: {}", e))?;

    // Parse the tokens into statements
    let mut parser = Parser::new(tokens);
    let statements = parser.parse()
        .map_err(|e| anyhow!("time: parse error: {}", e))?;

    // Create a temporary executor with the current runtime state
    let mut executor = Executor::new_embedded();

    // Copy the current runtime state into the executor
    *executor.runtime_mut() = runtime.clone();

    // Record the start time (for wall-clock time)
    let start_time = Instant::now();

    // Get CPU time before execution (if available on this platform)
    let cpu_time_before = get_cpu_time();

    // Execute the parsed statements
    let result = executor.execute(statements)
        .map_err(|e| anyhow!("time: execution error: {}", e))?;

    // Record the end time
    let elapsed_real = start_time.elapsed();

    // Get CPU time after execution
    let cpu_time_after = get_cpu_time();

    // Copy back the runtime state to preserve variable changes
    *runtime = executor.runtime_mut().clone();

    // Calculate timing information
    let (user_time, sys_time) = if let (Some(before), Some(after)) = (cpu_time_before, cpu_time_after) {
        (after.user - before.user, after.sys - before.sys)
    } else {
        // Fallback: estimate from elapsed time (not accurate but better than nothing)
        (elapsed_real, std::time::Duration::ZERO)
    };

    // Format timing output
    let timing_output = format_timing(elapsed_real, user_time, sys_time);

    // Combine the command output with timing information
    let combined_output = format!(
        "{}{}",
        result.stdout(),
        timing_output
    );

    // Return result with timing appended to output, but preserve stderr and exit code
    Ok(ExecutionResult {
        output: Output::Text(combined_output),
        stderr: result.stderr,
        exit_code: result.exit_code,
        error: result.error,
    })
}

/// CPU timing information
#[derive(Debug, Clone, Copy)]
struct CpuTime {
    user: std::time::Duration,
    sys: std::time::Duration,
}

/// Get current CPU time usage (user + system)
/// Returns None if the platform doesn't support this
#[cfg(unix)]
fn get_cpu_time() -> Option<CpuTime> {
    use std::time::Duration;

    // Try to use getrusage for accurate CPU timing
    #[cfg(not(target_os = "macos"))]
    {
        unsafe {
            let mut usage: libc::rusage = std::mem::zeroed();
            if libc::getrusage(libc::RUSAGE_CHILDREN, &mut usage) == 0 {
                let user = Duration::new(
                    usage.ru_utime.tv_sec as u64,
                    (usage.ru_utime.tv_usec as u32) * 1000,
                );
                let sys = Duration::new(
                    usage.ru_stime.tv_sec as u64,
                    (usage.ru_stime.tv_usec as u32) * 1000,
                );
                return Some(CpuTime { user, sys });
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        unsafe {
            let mut usage: libc::rusage = std::mem::zeroed();
            if libc::getrusage(libc::RUSAGE_CHILDREN, &mut usage) == 0 {
                let user = Duration::new(
                    usage.ru_utime.tv_sec as u64,
                    usage.ru_utime.tv_usec as u32 * 1000,
                );
                let sys = Duration::new(
                    usage.ru_stime.tv_sec as u64,
                    usage.ru_stime.tv_usec as u32 * 1000,
                );
                return Some(CpuTime { user, sys });
            }
        }
    }

    None
}

#[cfg(not(unix))]
fn get_cpu_time() -> Option<CpuTime> {
    None
}

/// Format timing output in POSIX style
/// real    0m0.123s
/// user    0m0.100s
/// sys     0m0.020s
fn format_timing(real: std::time::Duration, user: std::time::Duration, sys: std::time::Duration) -> String {
    fn duration_to_posix(d: std::time::Duration) -> String {
        let total_secs = d.as_secs_f64();
        let minutes = (total_secs / 60.0).floor() as u64;
        let seconds = total_secs - (minutes as f64 * 60.0);
        format!("{}m{:.3}s", minutes, seconds)
    }

    format!(
        "real\t{}\nuser\t{}\nsys\t{}\n",
        duration_to_posix(real),
        duration_to_posix(user),
        duration_to_posix(sys)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_echo() {
        let mut runtime = Runtime::new();
        let args = vec!["echo".to_string(), "hello".to_string()];
        let result = builtin_time(&args, &mut runtime).unwrap();

        // Check that output contains the command output
        assert!(result.stdout().contains("hello"));
        // Check that output contains timing information
        assert!(result.stdout().contains("real"));
        assert!(result.stdout().contains("user"));
        assert!(result.stdout().contains("sys"));
        assert_eq!(result.exit_code, 0);
    }

    #[test]
    fn test_time_no_args() {
        let mut runtime = Runtime::new();
        let result = builtin_time(&[], &mut runtime);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("usage"));
    }

    #[test]
    fn test_time_false_command() {
        let mut runtime = Runtime::new();
        let args = vec!["false".to_string()];
        let result = builtin_time(&args, &mut runtime).unwrap();

        // false returns exit code 1
        assert_eq!(result.exit_code, 1);
        // But still shows timing
        assert!(result.stdout().contains("real"));
    }

    #[test]
    fn test_format_timing() {
        let real = std::time::Duration::from_millis(123);
        let user = std::time::Duration::from_millis(100);
        let sys = std::time::Duration::from_millis(20);

        let output = format_timing(real, user, sys);

        // Should contain the time format markers
        assert!(output.contains("real"));
        assert!(output.contains("user"));
        assert!(output.contains("sys"));
        // Should contain time values with 's' suffix
        assert!(output.contains("s"));
    }

    #[test]
    fn test_format_timing_with_minutes() {
        let real = std::time::Duration::from_secs(65);
        let user = std::time::Duration::from_secs(60);
        let sys = std::time::Duration::from_secs(5);

        let output = format_timing(real, user, sys);

        // Should show minutes
        assert!(output.contains("1m"));
    }
}
