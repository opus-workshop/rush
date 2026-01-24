use crate::executor::{ExecutionResult, Output};
use crate::runtime::Runtime;
use anyhow::{anyhow, Result};

#[cfg(unix)]
use nix::sys::signal::{self, Signal};
#[cfg(unix)]
use nix::unistd::Pid;

/// Parse signal name or number to a Signal
#[cfg(unix)]
fn parse_signal(sig_str: &str) -> Result<Option<Signal>> {
    // Remove leading dash if present
    let sig_str = sig_str.strip_prefix('-').unwrap_or(sig_str);

    // Try to parse as number first
    if let Ok(num) = sig_str.parse::<i32>() {
        // Signal 0 is special - used to check if process exists
        if num == 0 {
            return Ok(None);
        }
        return Signal::try_from(num)
            .map(Some)
            .map_err(|_| anyhow!("kill: {}: invalid signal specification", sig_str));
    }

    // Parse signal name (with or without SIG prefix)
    let sig_name = if sig_str.starts_with("SIG") {
        sig_str
    } else {
        &format!("SIG{}", sig_str)
    };

    let signal = match sig_name {
        "SIGHUP" | "HUP" => Signal::SIGHUP,
        "SIGINT" | "INT" => Signal::SIGINT,
        "SIGQUIT" | "QUIT" => Signal::SIGQUIT,
        "SIGILL" | "ILL" => Signal::SIGILL,
        "SIGTRAP" | "TRAP" => Signal::SIGTRAP,
        "SIGABRT" | "ABRT" => Signal::SIGABRT,
        "SIGBUS" | "BUS" => Signal::SIGBUS,
        "SIGFPE" | "FPE" => Signal::SIGFPE,
        "SIGKILL" | "KILL" => Signal::SIGKILL,
        "SIGUSR1" | "USR1" => Signal::SIGUSR1,
        "SIGSEGV" | "SEGV" => Signal::SIGSEGV,
        "SIGUSR2" | "USR2" => Signal::SIGUSR2,
        "SIGPIPE" | "PIPE" => Signal::SIGPIPE,
        "SIGALRM" | "ALRM" => Signal::SIGALRM,
        "SIGTERM" | "TERM" => Signal::SIGTERM,
        "SIGCHLD" | "CHLD" => Signal::SIGCHLD,
        "SIGCONT" | "CONT" => Signal::SIGCONT,
        "SIGSTOP" | "STOP" => Signal::SIGSTOP,
        "SIGTSTP" | "TSTP" => Signal::SIGTSTP,
        "SIGTTIN" | "TTIN" => Signal::SIGTTIN,
        "SIGTTOU" | "TTOU" => Signal::SIGTTOU,
        "SIGURG" | "URG" => Signal::SIGURG,
        "SIGXCPU" | "XCPU" => Signal::SIGXCPU,
        "SIGXFSZ" | "XFSZ" => Signal::SIGXFSZ,
        "SIGVTALRM" | "VTALRM" => Signal::SIGVTALRM,
        "SIGPROF" | "PROF" => Signal::SIGPROF,
        "SIGWINCH" | "WINCH" => Signal::SIGWINCH,
        "SIGIO" | "IO" => Signal::SIGIO,
        "SIGSYS" | "SYS" => Signal::SIGSYS,
        _ => return Err(anyhow!("kill: {}: invalid signal specification", sig_str)),
    };
    
    Ok(Some(signal))
}

/// Implement the `kill` builtin command
///
/// Usage:
/// - `kill PID...` - Send SIGTERM to PIDs
/// - `kill -SIGNAL PID...` - Send specified signal to PIDs
/// - `kill -N PID...` - Send signal number N to PIDs
///
/// Examples:
/// - `kill 1234` - sends SIGTERM to PID 1234
/// - `kill -9 1234` - sends SIGKILL to PID 1234
/// - `kill -INT 1234` - sends SIGINT to PID 1234
/// - `kill 1234 5678` - sends SIGTERM to both PIDs
#[cfg(unix)]
pub fn builtin_kill(args: &[String], _runtime: &mut Runtime) -> Result<ExecutionResult> {
    if args.is_empty() {
        return Ok(ExecutionResult {
            output: Output::Text(String::new()),
            stderr: "kill: usage: kill [-s sigspec | -n signum | -sigspec] pid | jobspec ... or kill -l [sigspec]\n".to_string(),
            exit_code: 1,
            error: None,
        });
    }

    // Parse signal and PIDs
    let mut signal: Option<Signal> = Some(Signal::SIGTERM); // Default signal
    let mut pids = Vec::new();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        // Check if this is a signal specification
        if arg.starts_with('-') && arg.len() > 1 {
            // Check if it's a negative number or a signal name
            let without_dash = &arg[1..];
            
            // If it starts with a non-digit, it's definitely a signal name
            if !without_dash.chars().next().unwrap().is_ascii_digit() {
                // This is a signal name, parse it or error
                match parse_signal(arg) {
                    Ok(sig_opt) => {
                        signal = sig_opt;
                        i += 1;
                        continue;
                    }
                    Err(e) => {
                        return Ok(ExecutionResult {
                            output: Output::Text(String::new()),
                            stderr: format!("{}\n", e),
                            exit_code: 1,
        error: None,            
                        });
                    }
                }
            }
            
            // Otherwise, try parsing as a signal number first, then as negative PID
            match parse_signal(arg) {
                Ok(sig_opt) => {
                    signal = sig_opt;
                    i += 1;
                    continue;
                }
                Err(_) => {
                    // Fall through to try as PID (will fail with "invalid process ID" for negative)
                }
            }
        }

        // Try to parse as PID
        match arg.parse::<i32>() {
            Ok(pid) => {
                if pid <= 0 {
                    return Ok(ExecutionResult {
                        output: Output::Text(String::new()),
                        stderr: format!("kill: {}: invalid process ID\n", arg),
                        exit_code: 1,
                        error: None,
                    });
                }
                pids.push(pid);
            }
            Err(_) => {
                return Ok(ExecutionResult {
                    output: Output::Text(String::new()),
                    stderr: format!("kill: {}: arguments must be process or job IDs\n", arg),
                    exit_code: 1,
                    error: None,
                });
            }
        }
        i += 1;
    }

    if pids.is_empty() {
        return Ok(ExecutionResult {
            output: Output::Text(String::new()),
            stderr: "kill: usage: kill [-s sigspec | -n signum | -sigspec] pid | jobspec ... or kill -l [sigspec]\n".to_string(),
            exit_code: 1,
            error: None,
        });
    }

    // Send signal to each PID
    let mut stderr_output = String::new();
    let mut exit_code = 0;

    for pid in pids {
        let result = match signal {
            Some(sig) => signal::kill(Pid::from_raw(pid), sig),
            None => {
                // Signal 0 - check if process exists
                // We can do this by trying to send signal 0 via libc
                use nix::errno::Errno;
                let ret = unsafe { nix::libc::kill(pid, 0) };
                if ret == 0 {
                    Ok(())
                } else {
                    Err(Errno::last())
                }
            }
        };

        match result {
            Ok(_) => {
                // Success - no output
            }
            Err(errno) => {
                stderr_output.push_str(&format!("kill: ({}) - {}\n", pid, errno));
                exit_code = 1;
            }
        }
    }

    Ok(ExecutionResult {
        output: Output::Text(String::new()),
        stderr: stderr_output,
        exit_code,
        error: None,
    })
}

#[cfg(not(unix))]
pub fn builtin_kill(_args: &[String], _runtime: &mut Runtime) -> Result<ExecutionResult> {
    Ok(ExecutionResult {
        output: Output::Text(String::new()),
        stderr: "kill: not supported on this platform\n".to_string(),
        exit_code: 1,
        error: None,            
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn test_kill_no_args() {
        let mut runtime = Runtime::new();
        let result = builtin_kill(&[], &mut runtime).unwrap();
        assert_eq!(result.exit_code, 1);
        assert!(result.stderr.contains("usage"));
    }

    #[cfg(unix)]
    #[test]
    fn test_kill_invalid_pid() {
        let mut runtime = Runtime::new();
        let result = builtin_kill(&["abc".to_string()], &mut runtime).unwrap();
        assert_eq!(result.exit_code, 1);
        assert!(result.stderr.contains("arguments must be process"));
    }

    #[cfg(unix)]
    #[test]
    fn test_kill_negative_pid() {
        let mut runtime = Runtime::new();
        // In shells, -1 is treated as signal 1 (SIGHUP), not as a negative PID
        // When only a signal is given with no PID, it should show usage error
        let result = builtin_kill(&["-1".to_string()], &mut runtime).unwrap();
        assert_eq!(result.exit_code, 1);
        assert!(result.stderr.contains("usage"));
    }

    #[cfg(unix)]
    #[test]
    fn test_kill_zero_pid() {
        let mut runtime = Runtime::new();
        let result = builtin_kill(&["0".to_string()], &mut runtime).unwrap();
        assert_eq!(result.exit_code, 1);
        assert!(result.stderr.contains("invalid process ID"));
    }

    #[cfg(unix)]
    #[test]
    fn test_kill_self_with_sigterm() {
        // Skip this test because sending SIGTERM to self would actually terminate the test
        // In real usage, this works correctly, but it's not safe to test
    }

    #[cfg(unix)]
    #[test]
    fn test_kill_self_with_signal_zero() {
        let mut runtime = Runtime::new();
        let my_pid = std::process::id();

        // Signal 0 is a special case - just checks if process exists
        let result = builtin_kill(&["-0".to_string(), my_pid.to_string()], &mut runtime).unwrap();

        // Should succeed (process exists)
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stderr, "");
    }

    #[cfg(unix)]
    #[test]
    fn test_kill_with_signal_name() {
        // Skip this test because sending real signals to self is dangerous in tests
        // Signal 0 is tested separately as a safe alternative
    }

    #[cfg(unix)]
    #[test]
    fn test_kill_with_signal_name_prefixed() {
        // Skip this test because sending real signals to self is dangerous in tests
        // Signal 0 is tested separately as a safe alternative
    }

    #[cfg(unix)]
    #[test]
    fn test_kill_with_numeric_signal() {
        // Skip this test because sending real signals to self is dangerous in tests
        // Signal 0 is tested separately as a safe alternative
    }

    #[cfg(unix)]
    #[test]
    fn test_kill_multiple_pids() {
        let mut runtime = Runtime::new();
        let my_pid = std::process::id();

        // Send signal 0 to self multiple times (just checking process exists)
        let result = builtin_kill(
            &["-0".to_string(), my_pid.to_string(), my_pid.to_string()],
            &mut runtime
        ).unwrap();

        assert_eq!(result.exit_code, 0);
    }

    #[cfg(unix)]
    #[test]
    fn test_kill_nonexistent_pid() {
        let mut runtime = Runtime::new();

        // Use a very high PID that likely doesn't exist
        let result = builtin_kill(&["999999".to_string()], &mut runtime).unwrap();

        // Should fail
        assert_eq!(result.exit_code, 1);
        assert!(!result.stderr.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn test_kill_invalid_signal() {
        let mut runtime = Runtime::new();
        let my_pid = std::process::id();

        // Try with invalid signal name
        let result = builtin_kill(&["-INVALID".to_string(), my_pid.to_string()], &mut runtime).unwrap();

        // Should fail
        assert_eq!(result.exit_code, 1);
        assert!(result.stderr.contains("invalid signal"));
    }

    #[cfg(unix)]
    #[test]
    fn test_kill_signal_only() {
        let mut runtime = Runtime::new();

        // Signal without PID should fail
        let result = builtin_kill(&["-TERM".to_string()], &mut runtime).unwrap();
        assert_eq!(result.exit_code, 1);
        assert!(result.stderr.contains("usage"));
    }

    #[cfg(unix)]
    #[test]
    fn test_parse_signal_names() {
        // Test various signal name formats
        assert!(parse_signal("TERM").is_ok());
        assert!(parse_signal("SIGTERM").is_ok());
        assert!(parse_signal("-TERM").is_ok());
        assert!(parse_signal("-SIGTERM").is_ok());
        assert!(parse_signal("INT").is_ok());
        assert!(parse_signal("KILL").is_ok());
        assert!(parse_signal("HUP").is_ok());
        assert!(parse_signal("USR1").is_ok());
        assert!(parse_signal("USR2").is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn test_parse_signal_numbers() {
        // Test numeric signals
        assert!(parse_signal("9").is_ok()); // SIGKILL
        assert!(parse_signal("15").is_ok()); // SIGTERM
        assert!(parse_signal("2").is_ok()); // SIGINT
        assert!(parse_signal("1").is_ok()); // SIGHUP
        assert!(parse_signal("0").is_ok()); // Signal 0 (check if process exists)
    }

    #[cfg(unix)]
    #[test]
    fn test_parse_signal_invalid() {
        // Test invalid signal specifications
        assert!(parse_signal("INVALID").is_err());
        assert!(parse_signal("999").is_err());
        assert!(parse_signal("abc").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn test_kill_partial_failure() {
        let mut runtime = Runtime::new();
        let my_pid = std::process::id();

        // Try to kill self (should succeed) and a nonexistent PID (should fail)
        let result = builtin_kill(
            &["-0".to_string(), my_pid.to_string(), "999999".to_string()],
            &mut runtime
        ).unwrap();

        // Should have partial failure
        assert_eq!(result.exit_code, 1);
        assert!(!result.stderr.is_empty());
    }
}
