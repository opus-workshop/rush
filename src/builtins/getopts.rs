//! POSIX getopts builtin for option parsing in shell scripts.
//!
//! Usage: getopts optstring name [arg ...]
//!
//! Parses positional parameters (or provided args) for options.
//! Sets OPTIND, OPTARG, and the named variable.

use crate::executor::{ExecutionResult, Output};
use crate::runtime::Runtime;
use anyhow::{anyhow, Result};

/// Execute the getopts builtin.
///
/// getopts optstring name [arg ...]
///
/// Returns 0 while options remain, 1 when done.
pub fn builtin_getopts(args: &[String], runtime: &mut Runtime) -> Result<ExecutionResult> {
    // Handle help
    if args.iter().any(|a| a == "-h" || a == "--help") {
        return Ok(ExecutionResult::success(HELP_TEXT.to_string()));
    }

    // Need at least optstring and name
    if args.len() < 2 {
        return Err(anyhow!("getopts: usage: getopts optstring name [arg ...]"));
    }

    let optstring = &args[0];
    let varname = &args[1];

    // Get the arguments to parse (either provided args or positional params)
    let parse_args: Vec<String> = if args.len() > 2 {
        args[2..].to_vec()
    } else {
        runtime.get_positional_params().to_vec()
    };

    // Get current OPTIND (1-based index into args)
    let optind: usize = runtime
        .get_variable("OPTIND")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);

    // Get position within current bundled options (for -abc style)
    let optpos: usize = runtime
        .get_variable("_OPTPOS")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    // Check if we're done (OPTIND past end of args)
    if optind > parse_args.len() {
        runtime.set_variable(varname.clone(), "?".to_string());
        return Ok(ExecutionResult {
            output: Output::Text(String::new()),
            stderr: String::new(),
            exit_code: 1,
            error: None,
        }); // No more options
    }

    // Get current argument (0-indexed)
    let arg_idx = optind - 1;
    let current_arg = &parse_args[arg_idx];

    // Check if this is an option (starts with -)
    if !current_arg.starts_with('-') || current_arg == "-" {
        // Not an option - we're done
        runtime.set_variable(varname.clone(), "?".to_string());
        return Ok(ExecutionResult {
            output: Output::Text(String::new()),
            stderr: String::new(),
            exit_code: 1,
            error: None,
        });
    }

    // Check for -- (end of options)
    if current_arg == "--" {
        runtime.set_variable("OPTIND".to_string(), (optind + 1).to_string());
        runtime.set_variable(varname.clone(), "?".to_string());
        return Ok(ExecutionResult {
            output: Output::Text(String::new()),
            stderr: String::new(),
            exit_code: 1,
            error: None,
        });
    }

    // Determine which character position we're at in the current arg
    let char_pos = if optpos > 0 { optpos } else { 1 };
    let chars: Vec<char> = current_arg.chars().collect();

    if char_pos >= chars.len() {
        // Move to next argument
        runtime.set_variable("OPTIND".to_string(), (optind + 1).to_string());
        runtime.set_variable("_OPTPOS".to_string(), "0".to_string());
        // Recurse to process next arg
        return builtin_getopts(args, runtime);
    }

    let opt_char = chars[char_pos];

    // Check if this option is in optstring
    let silent_errors = optstring.starts_with(':');
    let search_string = if silent_errors {
        &optstring[1..]
    } else {
        optstring.as_str()
    };

    // Find the option in optstring
    let opt_pos = search_string.find(opt_char);

    match opt_pos {
        None => {
            // Unknown option
            if silent_errors {
                runtime.set_variable(varname.clone(), "?".to_string());
                runtime.set_variable("OPTARG".to_string(), opt_char.to_string());
            } else {
                eprintln!("getopts: illegal option -- {}", opt_char);
                runtime.set_variable(varname.clone(), "?".to_string());
                runtime.remove_variable("OPTARG");
            }

            // Move to next character or next arg
            if char_pos + 1 < chars.len() {
                runtime.set_variable("_OPTPOS".to_string(), (char_pos + 1).to_string());
            } else {
                runtime.set_variable("OPTIND".to_string(), (optind + 1).to_string());
                runtime.set_variable("_OPTPOS".to_string(), "0".to_string());
            }

            Ok(ExecutionResult::success(String::new()))
        }
        Some(pos) => {
            // Check if option requires an argument (followed by :)
            let needs_arg = search_string
                .chars()
                .nth(pos + 1)
                .map(|c| c == ':')
                .unwrap_or(false);

            if needs_arg {
                // Option requires an argument
                let optarg = if char_pos + 1 < chars.len() {
                    // Argument is attached: -fvalue
                    let arg: String = chars[char_pos + 1..].iter().collect();
                    runtime.set_variable("OPTIND".to_string(), (optind + 1).to_string());
                    runtime.set_variable("_OPTPOS".to_string(), "0".to_string());
                    Some(arg)
                } else if optind < parse_args.len() {
                    // Argument is next word: -f value
                    let arg = parse_args[optind].clone(); // optind is already 1-based, next arg
                    runtime.set_variable("OPTIND".to_string(), (optind + 2).to_string());
                    runtime.set_variable("_OPTPOS".to_string(), "0".to_string());
                    Some(arg)
                } else {
                    // Missing required argument
                    None
                };

                match optarg {
                    Some(arg) => {
                        runtime.set_variable(varname.clone(), opt_char.to_string());
                        runtime.set_variable("OPTARG".to_string(), arg);
                        Ok(ExecutionResult::success(String::new()))
                    }
                    None => {
                        // Missing argument
                        if silent_errors {
                            runtime.set_variable(varname.clone(), ":".to_string());
                            runtime.set_variable("OPTARG".to_string(), opt_char.to_string());
                        } else {
                            eprintln!("getopts: option requires an argument -- {}", opt_char);
                            runtime.set_variable(varname.clone(), "?".to_string());
                            runtime.remove_variable("OPTARG");
                        }
                        runtime.set_variable("OPTIND".to_string(), (optind + 1).to_string());
                        runtime.set_variable("_OPTPOS".to_string(), "0".to_string());
                        Ok(ExecutionResult::success(String::new()))
                    }
                }
            } else {
                // Option without argument
                runtime.set_variable(varname.clone(), opt_char.to_string());
                runtime.remove_variable("OPTARG");

                // Move to next character or next arg
                if char_pos + 1 < chars.len() {
                    runtime.set_variable("_OPTPOS".to_string(), (char_pos + 1).to_string());
                } else {
                    runtime.set_variable("OPTIND".to_string(), (optind + 1).to_string());
                    runtime.set_variable("_OPTPOS".to_string(), "0".to_string());
                }

                Ok(ExecutionResult::success(String::new()))
            }
        }
    }
}

const HELP_TEXT: &str = r#"getopts - parse positional parameters for options

USAGE:
    getopts optstring name [arg ...]

DESCRIPTION:
    Parse positional parameters (or provided arguments) for options.
    Used in shell scripts to process command-line options.

    For each invocation, getopts places the next option in the shell
    variable 'name', the index of the next argument in OPTIND, and
    the option argument (if any) in OPTARG.

OPTSTRING:
    A string of recognized option characters. If a character is
    followed by a colon, the option requires an argument.

    If optstring starts with ':', silent error mode is enabled:
    - Unknown options set name to '?' and OPTARG to the character
    - Missing arguments set name to ':' and OPTARG to the character

EXAMPLES:
    # Basic option parsing
    while getopts "ab:c" opt; do
        case $opt in
            a) echo "Option a";;
            b) echo "Option b with arg: $OPTARG";;
            c) echo "Option c";;
            \?) echo "Invalid option: $OPTARG";;
        esac
    done
    shift $((OPTIND - 1))

    # Silent error mode
    while getopts ":ab:" opt; do
        case $opt in
            a) echo "a";;
            b) echo "b=$OPTARG";;
            :) echo "Missing arg for -$OPTARG";;
            \?) echo "Unknown: -$OPTARG";;
        esac
    done

RETURN VALUE:
    0   An option was found
    1   End of options or error

SEE ALSO:
    shift, set
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn make_runtime() -> Runtime {
        let mut rt = Runtime::new();
        rt.set_variable("OPTIND".to_string(), "1".to_string());
        rt
    }

    #[test]
    fn test_simple_option() {
        let mut rt = make_runtime();
        rt.set_positional_params(vec!["-a".to_string()]);

        let result = builtin_getopts(&["a".to_string(), "opt".to_string()], &mut rt).unwrap();
        assert_eq!(result.exit_code, 0);
        assert_eq!(rt.get_variable("opt"), Some("a".to_string()));
    }

    #[test]
    fn test_option_with_argument() {
        let mut rt = make_runtime();
        rt.set_positional_params(vec!["-b".to_string(), "value".to_string()]);

        let result = builtin_getopts(&["b:".to_string(), "opt".to_string()], &mut rt).unwrap();
        assert_eq!(result.exit_code, 0);
        assert_eq!(rt.get_variable("opt"), Some("b".to_string()));
        assert_eq!(rt.get_variable("OPTARG"), Some("value".to_string()));
    }

    #[test]
    fn test_bundled_options() {
        let mut rt = make_runtime();
        rt.set_positional_params(vec!["-abc".to_string()]);

        // First call gets 'a'
        let result = builtin_getopts(&["abc".to_string(), "opt".to_string()], &mut rt).unwrap();
        assert_eq!(result.exit_code, 0);
        assert_eq!(rt.get_variable("opt"), Some("a".to_string()));

        // Second call gets 'b'
        let result = builtin_getopts(&["abc".to_string(), "opt".to_string()], &mut rt).unwrap();
        assert_eq!(result.exit_code, 0);
        assert_eq!(rt.get_variable("opt"), Some("b".to_string()));

        // Third call gets 'c'
        let result = builtin_getopts(&["abc".to_string(), "opt".to_string()], &mut rt).unwrap();
        assert_eq!(result.exit_code, 0);
        assert_eq!(rt.get_variable("opt"), Some("c".to_string()));
    }

    #[test]
    fn test_end_of_options() {
        let mut rt = make_runtime();
        rt.set_positional_params(vec!["arg".to_string()]); // Not an option

        let result = builtin_getopts(&["a".to_string(), "opt".to_string()], &mut rt).unwrap();
        assert_eq!(result.exit_code, 1);
    }

    #[test]
    fn test_unknown_option() {
        let mut rt = make_runtime();
        rt.set_positional_params(vec!["-x".to_string()]);

        let result = builtin_getopts(&["ab".to_string(), "opt".to_string()], &mut rt).unwrap();
        assert_eq!(result.exit_code, 0);
        assert_eq!(rt.get_variable("opt"), Some("?".to_string()));
    }
}
