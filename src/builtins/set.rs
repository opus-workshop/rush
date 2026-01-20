use crate::executor::ExecutionResult;
use crate::runtime::Runtime;
use anyhow::{anyhow, Result};

pub fn builtin_set(args: &[String], runtime: &mut Runtime) -> Result<ExecutionResult> {
    // No args: show current options
    if args.is_empty() {
        return show_options(runtime);
    }

    // Parse options
    for arg in args {
        if let Some(option_chars) = arg.strip_prefix('-') {
            // Setting options: -e, -u, -x, etc.

            // Handle -o followed by option name
            if option_chars == "o" {
                // Next arg should be the option name
                let idx = args.iter().position(|a| a == arg).unwrap();
                if idx + 1 >= args.len() {
                    return Err(anyhow!("set: -o requires an argument"));
                }
                let option_name = &args[idx + 1];
                runtime.set_option(option_name, true)?;
                continue;
            }

            // Handle combined short options like -ex
            for ch in option_chars.chars() {
                let opt = ch.to_string();
                runtime.set_option(&opt, true)?;
            }
        } else if let Some(option_chars) = arg.strip_prefix('+') {
            // Unsetting options: +e, +u, +x, etc.

            // Handle +o followed by option name
            if option_chars == "o" {
                // Next arg should be the option name
                let idx = args.iter().position(|a| a == arg).unwrap();
                if idx + 1 >= args.len() {
                    return Err(anyhow!("set: +o requires an argument"));
                }
                let option_name = &args[idx + 1];
                runtime.set_option(option_name, false)?;
                continue;
            }

            // Handle combined short options
            for ch in option_chars.chars() {
                let opt = ch.to_string();
                runtime.set_option(&opt, false)?;
            }
        } else if arg != "pipefail" && arg != "nounset" && arg != "errexit" && arg != "xtrace" {
            // Skip long option names that are arguments to -o/+o
            return Err(anyhow!("set: invalid argument: {}", arg));
        }
    }

    Ok(ExecutionResult::success(String::new()))
}

fn show_options(runtime: &Runtime) -> Result<ExecutionResult> {
    let mut output = String::new();

    // Show all options in the format "set [-/+]option"
    if runtime.options.errexit {
        output.push_str("set -e\n");
    } else {
        output.push_str("set +e\n");
    }

    if runtime.options.nounset {
        output.push_str("set -u\n");
    } else {
        output.push_str("set +u\n");
    }

    if runtime.options.xtrace {
        output.push_str("set -x\n");
    } else {
        output.push_str("set +x\n");
    }

    if runtime.options.pipefail {
        output.push_str("set -o pipefail\n");
    } else {
        output.push_str("set +o pipefail\n");
    }

    Ok(ExecutionResult::success(output))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Runtime;

    #[test]
    fn test_set_single_option() {
        let mut runtime = Runtime::new();

        // Test setting errexit
        let result = builtin_set(&["-e".to_string()], &mut runtime);
        assert!(result.is_ok());
        assert!(runtime.options.errexit);

        // Test unsetting errexit
        let result = builtin_set(&["+e".to_string()], &mut runtime);
        assert!(result.is_ok());
        assert!(!runtime.options.errexit);
    }

    #[test]
    fn test_set_multiple_options() {
        let mut runtime = Runtime::new();

        // Test setting multiple options at once
        let result = builtin_set(&["-eux".to_string()], &mut runtime);
        assert!(result.is_ok());
        assert!(runtime.options.errexit);
        assert!(runtime.options.nounset);
        assert!(runtime.options.xtrace);
    }

    #[test]
    fn test_set_o_option() {
        let mut runtime = Runtime::new();

        // Test setting pipefail
        let result = builtin_set(&["-o".to_string(), "pipefail".to_string()], &mut runtime);
        assert!(result.is_ok());
        assert!(runtime.options.pipefail);

        // Test unsetting pipefail
        let result = builtin_set(&["+o".to_string(), "pipefail".to_string()], &mut runtime);
        assert!(result.is_ok());
        assert!(!runtime.options.pipefail);
    }

    #[test]
    fn test_set_show_options() {
        let mut runtime = Runtime::new();
        runtime.options.errexit = true;
        runtime.options.xtrace = true;

        let result = builtin_set(&[], &mut runtime).unwrap();
        assert!(result.stdout.contains("set -e"));
        assert!(result.stdout.contains("set -x"));
        assert!(result.stdout.contains("set +u"));
        assert!(result.stdout.contains("set +o pipefail"));
    }

    #[test]
    fn test_set_invalid_option() {
        let mut runtime = Runtime::new();

        let result = builtin_set(&["-z".to_string()], &mut runtime);
        assert!(result.is_err());
    }
}
