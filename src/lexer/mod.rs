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

    // Operators and punctuation
    #[token("=")]
    Equals,

    #[token("==")]
    DoubleEquals,

    #[token("!=")]
    NotEquals,

    #[token(">")]
    GreaterThan,

    #[token("<")]
    LessThan,

    #[token(">=")]
    GreaterThanOrEqual,

    #[token("<=")]
    LessThanOrEqual,

    #[token("|||")]
    ParallelPipe,

    #[token("|")]
    Pipe,

    #[token("&&")]
    And,

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

    // Identifiers and commands
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_-]*", |lex| lex.slice().to_string())]
    Identifier(String),

    // Command substitution
    #[regex(r"\$\([^)]+\)", |lex| lex.slice().to_string())]
    CommandSubstitution(String),

    #[regex(r"\$[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Variable(String),

    // File paths and arguments
    #[regex(r"[./][^\s|;&(){}]+", |lex| lex.slice().to_string())]
    Path(String),

    // Flags
    #[regex(r"-[a-zA-Z0-9]+", |lex| lex.slice().to_string())]
    ShortFlag(String),

    #[regex(r"--[a-zA-Z0-9][a-zA-Z0-9-]*", |lex| lex.slice().to_string())]
    LongFlag(String),

    // Redirects
    #[token(">>")]
    AppendRedirect,

    #[token("2>")]
    StderrRedirect,

    #[token("&>")]
    AllRedirect,

    // Newline and EOF
    #[regex(r"\n")]
    Newline,

    #[token("\r\n")]
    CrLf,

    // Comments
    #[regex(r"#[^\n]*", logos::skip)]
    Comment,
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
}
