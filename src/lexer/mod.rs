use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t]+")]
pub enum Token {
    // Keywords
    #[token("let")]
    Let,

    #[token("if")]
    If,

    #[token("else")]
    Else,

    #[token("fn")]
    Fn,

    #[token("match")]
    Match,

    #[token("for")]
    For,

    #[token("in")]
    In,

    #[token("while")]
    While,

    #[token("do")]
    Do,

    #[token("done")]
    Done,

    #[token("until")]
    Until,

    #[token("case")]
    Case,

    #[token("esac")]
    Esac,

    // Operators and punctuation
    #[token("=")]
    Equals,

    #[token("==")]
    DoubleEquals,

    #[token("!=")]
    NotEquals,

    #[token(">=")]
    GreaterThanOrEqual,

    #[token("<=")]
    LessThanOrEqual,

    #[token(">")]
    GreaterThan,

    #[token("|||")]
    ParallelPipe,

    #[token("|")]
    Pipe,

    #[token("&&")]
    And,

    #[token("&")]
    Ampersand,

    #[token("||")]
    Or,

    #[token("!")]
    Bang,

    #[token("(")]
    LeftParen,

    #[token(")")]
    RightParen,

    #[token("{")]
    LeftBrace,

    #[token("}")]
    RightBrace,

    #[token("[")]
    LeftBracket,

    #[token("]")]
    RightBracket,

    #[token(";")]
    Semicolon,

    #[token(":")]
    Colon,

    #[token(",")]
    Comma,

    #[token(".")]
    Dot,

    #[token("->")]
    Arrow,

    #[token("=>")]
    FatArrow,

    // String literals
    #[regex(r#""([^"\\]|\\.)*""#, |lex| lex.slice().to_string())]
    String(String),

    #[regex(r"'([^'\\]|\\.)*'", |lex| lex.slice().to_string())]
    SingleQuotedString(String),

    // Numbers
    #[regex(r"-?[0-9]+", |lex| lex.slice().parse().ok())]
    Integer(i64),

    #[regex(r"-?[0-9]+\.[0-9]+", |lex| lex.slice().parse().ok())]
    Float(f64),

    // Glob patterns (*, ?, [...] wildcards in filename context)
    // Patterns with * or ? (e.g., *.rs, file?.txt, src/**/*.rs)
    #[regex(r"[a-zA-Z0-9_.\-/]*[*?][a-zA-Z0-9_.*?\-/\[\]]*", |lex| lex.slice().to_string())]
    // Bracket glob patterns (e.g., [abc].txt, file[0-9].txt)
    // Must have content after ] to distinguish from test builtin [ ]
    #[regex(r"[a-zA-Z0-9_.\-/]*\[[^\]]+\][a-zA-Z0-9_.*?\-/]+", |lex| lex.slice().to_string())]
    GlobPattern(String),

    // Identifiers and commands (dots allowed so filenames like README.md tokenize as one word)
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_.\-]*", |lex| lex.slice().to_string())]
    Identifier(String),

    // Command substitution - needs custom parsing for nested cases
    #[regex(r"\$\(", parse_command_substitution)]
    CommandSubstitution(String),

    // Backtick command substitution
    #[regex(r"`", parse_backtick_substitution)]
    BacktickSubstitution(String),

    // Braced variables - must come before Special and Regular variables
    #[regex(r"\$\{[^}]+\}", |lex| lex.slice().to_string())]
    BracedVariable(String),

    // Special variables ($?, $!, $$, $#, $@, $*, $0-9, $-, $_)
    // Includes both single and special multi-char patterns
    #[regex(r"\$[?!$#@*\-_0-9]", |lex| lex.slice().to_string())]
    SpecialVariable(String),

    // Regular variables (at least 2 chars after $, or single letter)
    // This ensures $_ is matched as SpecialVariable, not Variable
    #[regex(r"\$[a-zA-Z][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Variable(String),

    // File paths and arguments
    #[regex(r"[.~/][^\s|;&(){}]+", |lex| lex.slice().to_string())]
    Path(String),

    // Flags
    // Bare dash (used in cd - for previous directory)
    #[token("-")]
    Dash,

    #[regex(r"-[a-zA-Z0-9]+", |lex| lex.slice().to_string())]
    ShortFlag(String),

    #[regex(r"--[a-zA-Z0-9][a-zA-Z0-9-]*", |lex| lex.slice().to_string())]
    LongFlag(String),

    // Plus flags (for unsetting shell options like +e, +u, +x)
    #[regex(r"\+[a-zA-Z0-9]+", |lex| lex.slice().to_string())]
    PlusFlag(String),

    // Redirects
    #[token(">>")]
    StdoutAppend,

    #[token("2>&1")]
    StderrToStdout,

    #[token("2>")]
    StderrRedirect,

    #[token("&>")]
    BothRedirect,

    #[token("<")]
    StdinRedirect,

    // Newline and EOF
    #[regex(r"\n")]
    Newline,

    #[token("\r\n")]
    CrLf,

    // Comments
    #[regex(r"#[^\n]*", logos::skip)]
    Comment,
}

// Custom parser for $(...) that handles nesting
fn parse_command_substitution(lex: &mut logos::Lexer<Token>) -> Option<String> {
    let start = lex.span().start;
    let input = lex.source();
    let mut depth = 1;
    let mut pos = lex.span().end;

    while pos < input.len() && depth > 0 {
        let ch = input.as_bytes()[pos] as char;
        if ch == '(' && pos > 0 && input.as_bytes()[pos - 1] as char == '$' {
            depth += 1;
        } else if ch == ')' {
            depth -= 1;
        }
        pos += 1;
    }

    if depth == 0 {
        // Extract the command including the $() delimiters
        let result = input[start..pos].to_string();
        // Update the lexer position
        lex.bump(pos - lex.span().end);
        Some(result)
    } else {
        None
    }
}

// Custom parser for backtick command substitution
fn parse_backtick_substitution(lex: &mut logos::Lexer<Token>) -> Option<String> {
    let start = lex.span().start;
    let input = lex.source();
    let mut pos = lex.span().end;

    // Find matching backtick
    while pos < input.len() {
        let ch = input.as_bytes()[pos] as char;
        if ch == '`' {
            pos += 1;
            let result = input[start..pos].to_string();
            lex.bump(pos - lex.span().end);
            return Some(result);
        } else if ch == '\\' && pos + 1 < input.len() {
            // Skip escaped character
            pos += 2;
        } else {
            pos += 1;
        }
    }

    None // Unclosed backtick
}

pub struct Lexer<'a> {
    inner: logos::Lexer<'a, Token>,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            inner: Token::lexer(input),
        }
    }

    pub fn tokenize(input: &str) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();
        let mut lexer = Token::lexer(input);

        while let Some(token_result) = lexer.next() {
            match token_result {
                Ok(token) => tokens.push(token),
                Err(_) => {
                    return Err(LexerError::InvalidToken {
                        position: lexer.span().start,
                        text: lexer.slice().to_string(),
                    });
                }
            }
        }

        Ok(tokens)
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token, LexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|result| {
            result.map_err(|_| LexerError::InvalidToken {
                position: self.inner.span().start,
                text: self.inner.slice().to_string(),
            })
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LexerError {
    #[error("Invalid token at position {position}: '{text}'")]
    InvalidToken { position: usize, text: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_command() {
        let tokens = Lexer::tokenize("ls -la /home").unwrap();
        assert_eq!(tokens.len(), 3);
        assert!(matches!(tokens[0], Token::Identifier(_)));
        assert!(matches!(tokens[1], Token::ShortFlag(_)));
        assert!(matches!(tokens[2], Token::Path(_)));
    }

    #[test]
    fn test_pipeline() {
        let tokens = Lexer::tokenize("ls | grep foo").unwrap();
        assert!(tokens.contains(&Token::Pipe));
    }

    #[test]
    fn test_variable() {
        let tokens = Lexer::tokenize("echo $HOME").unwrap();
        assert!(matches!(tokens[1], Token::Variable(_)));
    }

    #[test]
    fn test_string_interpolation() {
        let tokens = Lexer::tokenize(r#"echo "hello world""#).unwrap();
        assert!(matches!(tokens[1], Token::String(_)));
    }

    #[test]
    fn test_let_statement() {
        let tokens = Lexer::tokenize("let x = 42").unwrap();
        assert_eq!(tokens[0], Token::Let);
        assert!(matches!(tokens[1], Token::Identifier(_)));
        assert_eq!(tokens[2], Token::Equals);
        assert!(matches!(tokens[3], Token::Integer(42)));
    }

    #[test]
    fn test_function_definition() {
        let tokens = Lexer::tokenize("fn deploy(env: String) {}").unwrap();
        assert_eq!(tokens[0], Token::Fn);
        assert!(matches!(tokens[1], Token::Identifier(_)));
    }

    #[test]
    fn test_command_substitution_simple() {
        let tokens = Lexer::tokenize("echo $(pwd)").unwrap();
        assert_eq!(tokens.len(), 2);
        if let Token::CommandSubstitution(cmd) = &tokens[1] {
            assert_eq!(cmd, "$(pwd)");
        } else {
            panic!("Expected CommandSubstitution token");
        }
    }

    #[test]
    fn test_command_substitution_nested() {
        let tokens = Lexer::tokenize("echo $(echo $(pwd))").unwrap();
        assert_eq!(tokens.len(), 2);
        if let Token::CommandSubstitution(cmd) = &tokens[1] {
            assert_eq!(cmd, "$(echo $(pwd))");
        } else {
            panic!("Expected CommandSubstitution token");
        }
    }

    #[test]
    fn test_backtick_substitution() {
        let tokens = Lexer::tokenize("echo `pwd`").unwrap();
        assert_eq!(tokens.len(), 2);
        if let Token::BacktickSubstitution(cmd) = &tokens[1] {
            assert_eq!(cmd, "`pwd`");
        } else {
            panic!("Expected BacktickSubstitution token");
        }
    }

    #[test]
    fn test_braced_variable_simple() {
        let tokens = Lexer::tokenize("echo ${VAR}").unwrap();
        assert_eq!(tokens.len(), 2);
        if let Token::BracedVariable(var) = &tokens[1] {
            assert_eq!(var, "${VAR}");
        } else {
            panic!("Expected BracedVariable token, got {:?}", tokens[1]);
        }
    }

    #[test]
    fn test_braced_variable_use_default() {
        let tokens = Lexer::tokenize("echo ${VAR:-default}").unwrap();
        assert_eq!(tokens.len(), 2);
        if let Token::BracedVariable(var) = &tokens[1] {
            assert_eq!(var, "${VAR:-default}");
        } else {
            panic!("Expected BracedVariable token, got {:?}", tokens[1]);
        }
    }

    #[test]
    fn test_braced_variable_assign_default() {
        let tokens = Lexer::tokenize("echo ${VAR:=default}").unwrap();
        assert_eq!(tokens.len(), 2);
        if let Token::BracedVariable(var) = &tokens[1] {
            assert_eq!(var, "${VAR:=default}");
        } else {
            panic!("Expected BracedVariable token, got {:?}", tokens[1]);
        }
    }

    #[test]
    fn test_braced_variable_error_if_unset() {
        let tokens = Lexer::tokenize("echo ${VAR:?error}").unwrap();
        assert_eq!(tokens.len(), 2);
        if let Token::BracedVariable(var) = &tokens[1] {
            assert_eq!(var, "${VAR:?error}");
        } else {
            panic!("Expected BracedVariable token, got {:?}", tokens[1]);
        }
    }

    #[test]
    fn test_braced_variable_prefix_removal() {
        let tokens = Lexer::tokenize("echo ${VAR#prefix}").unwrap();
        assert_eq!(tokens.len(), 2);
        if let Token::BracedVariable(var) = &tokens[1] {
            assert_eq!(var, "${VAR#prefix}");
        } else {
            panic!("Expected BracedVariable token, got {:?}", tokens[1]);
        }
    }

    #[test]
    fn test_braced_variable_suffix_removal() {
        let tokens = Lexer::tokenize("echo ${VAR%suffix}").unwrap();
        assert_eq!(tokens.len(), 2);
        if let Token::BracedVariable(var) = &tokens[1] {
            assert_eq!(var, "${VAR%suffix}");
        } else {
            panic!("Expected BracedVariable token, got {:?}", tokens[1]);
        }
    }

    #[test]
    fn test_special_variable_shell_pid() {
        let tokens = Lexer::tokenize("$$").unwrap();
        assert_eq!(tokens.len(), 1, "Should have 1 token for $$");
        if let Token::SpecialVariable(var) = &tokens[0] {
            assert_eq!(var, "$$", "Should be $$ token");
        } else {
            panic!("Expected SpecialVariable token for $$, got {:?}", tokens[0]);
        }
    }

    #[test]
    fn test_special_variable_last_bg_pid() {
        let tokens = Lexer::tokenize("$!").unwrap();
        assert_eq!(tokens.len(), 1);
        if let Token::SpecialVariable(var) = &tokens[0] {
            assert_eq!(var, "$!");
        } else {
            panic!("Expected SpecialVariable token for $!, got {:?}", tokens[0]);
        }
    }

    #[test]
    fn test_special_variable_option_flags() {
        let tokens = Lexer::tokenize("$-").unwrap();
        assert_eq!(tokens.len(), 1);
        if let Token::SpecialVariable(var) = &tokens[0] {
            assert_eq!(var, "$-");
        } else {
            panic!("Expected SpecialVariable token for $-, got {:?}", tokens[0]);
        }
    }

    #[test]
    fn test_special_variable_last_arg() {
        let tokens = Lexer::tokenize("$_").unwrap();
        assert_eq!(tokens.len(), 1);
        if let Token::SpecialVariable(var) = &tokens[0] {
            assert_eq!(var, "$_");
        } else {
            panic!("Expected SpecialVariable token for $_, got {:?}", tokens[0]);
        }
    }

    #[test]
    fn test_while_keyword() {
        let tokens = Lexer::tokenize("while").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::While);
    }

    #[test]
    fn test_do_keyword() {
        let tokens = Lexer::tokenize("do").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::Do);
    }

    #[test]
    fn test_done_keyword() {
        let tokens = Lexer::tokenize("done").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::Done);
    }

    #[test]
    fn test_filename_with_dot() {
        let tokens = Lexer::tokenize("cat README.md").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0], Token::Identifier(_)));
        assert!(matches!(tokens[1], Token::Identifier(ref s) if s == "README.md"));
    }

    #[test]
    fn test_filename_multiple_dots() {
        let tokens = Lexer::tokenize("echo file.tar.gz").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[1], Token::Identifier(ref s) if s == "file.tar.gz"));
    }

    #[test]
    fn test_dot_alone_is_path() {
        let tokens = Lexer::tokenize("echo .").unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[1], Token::Dot);
    }

    #[test]
    fn test_dotdot_is_path() {
        let tokens = Lexer::tokenize("echo ..").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[1], Token::Path(ref s) if s == ".."));
    }

    #[test]
    fn test_dot_slash_path() {
        let tokens = Lexer::tokenize("./script.sh").unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0], Token::Path(ref s) if s == "./script.sh"));
    }
}
