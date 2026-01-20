pub mod ast;

use crate::lexer::Token;
use ast::*;
use anyhow::{anyhow, Result};

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    pub fn parse(&mut self) -> Result<Vec<Statement>> {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            // Skip newlines between statements
            while self.match_token(&Token::Newline) || self.match_token(&Token::CrLf) {
                self.advance();
            }

            if self.is_at_end() {
                break;
            }

            statements.push(self.parse_statement()?);
        }

        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<Statement> {
        // Check for keywords first
        match self.peek() {
            Some(Token::Let) => self.parse_assignment(),
            Some(Token::Fn) => self.parse_function_def(),
            Some(Token::If) => self.parse_if_statement(),
            Some(Token::For) => self.parse_for_loop(),
            Some(Token::Match) => self.parse_match_expression(),
            _ => self.parse_command_or_pipeline(),
        }
    }

    fn parse_command_or_pipeline(&mut self) -> Result<Statement> {
        let first_command = self.parse_command()?;

        // Check if this is a pipeline
        if self.match_token(&Token::Pipe) {
            self.advance();
            let mut commands = vec![first_command];

            loop {
                commands.push(self.parse_command()?);

                if !self.match_token(&Token::Pipe) {
                    break;
                }
                self.advance();
            }

            Ok(Statement::Pipeline(Pipeline { commands }))
        } else {
            Ok(Statement::Command(first_command))
        }
    }

    fn parse_command(&mut self) -> Result<Command> {
        let name = match self.advance() {
            Some(Token::Identifier(s)) => s.clone(),
            _ => return Err(anyhow!("Expected command name")),
        };

        let mut args = Vec::new();
        let mut redirects = Vec::new();

        while !self.is_at_end()
            && !self.match_token(&Token::Pipe)
            && !self.match_token(&Token::Newline)
            && !self.match_token(&Token::Semicolon)
        {
            match self.peek() {
                Some(Token::AppendRedirect) => {
                    self.advance();
                    let target = self.parse_redirect_target()?;
                    redirects.push(Redirect {
                        kind: RedirectKind::Append,
                        target,
                    });
                }
                Some(Token::StderrRedirect) => {
                    self.advance();
                    let target = self.parse_redirect_target()?;
                    redirects.push(Redirect {
                        kind: RedirectKind::Stderr,
                        target,
                    });
                }
                Some(Token::AllRedirect) => {
                    self.advance();
                    let target = self.parse_redirect_target()?;
                    redirects.push(Redirect {
                        kind: RedirectKind::All,
                        target,
                    });
                }
                _ => {
                    args.push(self.parse_argument()?);
                }
            }
        }

        Ok(Command {
            name,
            args,
            redirects,
        })
    }

    fn parse_argument(&mut self) -> Result<Argument> {
        match self.advance() {
            Some(Token::String(s)) | Some(Token::SingleQuotedString(s)) => {
                // Remove quotes
                let unquoted = s.trim_matches('"').trim_matches('\'');
                Ok(Argument::Literal(unquoted.to_string()))
            }
            Some(Token::Identifier(s)) => Ok(Argument::Literal(s.clone())),
            Some(Token::Variable(s)) => Ok(Argument::Variable(s.clone())),
            Some(Token::ShortFlag(s)) | Some(Token::LongFlag(s)) => {
                Ok(Argument::Flag(s.clone()))
            }
            Some(Token::Path(s)) => Ok(Argument::Path(s.clone())),
            Some(Token::Integer(n)) => Ok(Argument::Literal(n.to_string())),
            _ => Err(anyhow!("Expected argument")),
        }
    }

    fn parse_redirect_target(&mut self) -> Result<String> {
        match self.advance() {
            Some(Token::Path(s)) | Some(Token::Identifier(s)) => Ok(s.clone()),
            Some(Token::String(s)) => Ok(s.trim_matches('"').to_string()),
            _ => Err(anyhow!("Expected redirect target")),
        }
    }

    fn parse_assignment(&mut self) -> Result<Statement> {
        self.expect_token(&Token::Let)?;

        let name = match self.advance() {
            Some(Token::Identifier(s)) => s.clone(),
            _ => return Err(anyhow!("Expected variable name")),
        };

        self.expect_token(&Token::Equals)?;

        let value = self.parse_expression()?;

        Ok(Statement::Assignment(Assignment { name, value }))
    }

    fn parse_expression(&mut self) -> Result<Expression> {
        // For now, simple expression parsing
        match self.peek() {
            Some(Token::String(s)) => {
                let s = s.clone();
                self.advance();
                Ok(Expression::Literal(Literal::String(
                    s.trim_matches('"').to_string(),
                )))
            }
            Some(Token::Integer(n)) => {
                let n = *n;
                self.advance();
                Ok(Expression::Literal(Literal::Integer(n)))
            }
            Some(Token::Float(f)) => {
                let f = *f;
                self.advance();
                Ok(Expression::Literal(Literal::Float(f)))
            }
            Some(Token::Variable(v)) => {
                let v = v.clone();
                self.advance();
                Ok(Expression::Variable(v))
            }
            Some(Token::CommandSubstitution(cmd)) => {
                let cmd = cmd.clone();
                self.advance();
                Ok(Expression::CommandSubstitution(cmd))
            }
            Some(Token::Identifier(s)) => {
                let s = s.clone();
                self.advance();
                Ok(Expression::Literal(Literal::String(s)))
            }
            _ => Err(anyhow!("Expected expression")),
        }
    }

    fn parse_function_def(&mut self) -> Result<Statement> {
        self.expect_token(&Token::Fn)?;

        let name = match self.advance() {
            Some(Token::Identifier(s)) => s.clone(),
            _ => return Err(anyhow!("Expected function name")),
        };

        self.expect_token(&Token::LeftParen)?;

        let params = self.parse_parameters()?;

        self.expect_token(&Token::RightParen)?;
        self.expect_token(&Token::LeftBrace)?;

        let body = self.parse_block()?;

        self.expect_token(&Token::RightBrace)?;

        Ok(Statement::FunctionDef(FunctionDef { name, params, body }))
    }

    fn parse_parameters(&mut self) -> Result<Vec<Parameter>> {
        let mut params = Vec::new();

        while !self.match_token(&Token::RightParen) {
            let name = match self.advance() {
                Some(Token::Identifier(s)) => s.clone(),
                _ => return Err(anyhow!("Expected parameter name")),
            };

            let type_hint = if self.match_token(&Token::Colon) {
                self.advance();
                match self.advance() {
                    Some(Token::Identifier(s)) => Some(s.clone()),
                    _ => None,
                }
            } else {
                None
            };

            params.push(Parameter { name, type_hint });

            if self.match_token(&Token::Comma) {
                self.advance();
            }
        }

        Ok(params)
    }

    fn parse_block(&mut self) -> Result<Vec<Statement>> {
        let mut statements = Vec::new();

        while !self.match_token(&Token::RightBrace) && !self.is_at_end() {
            // Skip newlines
            while self.match_token(&Token::Newline) {
                self.advance();
            }

            if self.match_token(&Token::RightBrace) {
                break;
            }

            statements.push(self.parse_statement()?);
        }

        Ok(statements)
    }

    fn parse_if_statement(&mut self) -> Result<Statement> {
        self.expect_token(&Token::If)?;

        let condition = self.parse_expression()?;

        self.expect_token(&Token::LeftBrace)?;
        let then_block = self.parse_block()?;
        self.expect_token(&Token::RightBrace)?;

        let else_block = if self.match_token(&Token::Else) {
            self.advance();
            self.expect_token(&Token::LeftBrace)?;
            let block = self.parse_block()?;
            self.expect_token(&Token::RightBrace)?;
            Some(block)
        } else {
            None
        };

        Ok(Statement::IfStatement(IfStatement {
            condition,
            then_block,
            else_block,
        }))
    }

    fn parse_for_loop(&mut self) -> Result<Statement> {
        self.expect_token(&Token::For)?;

        let variable = match self.advance() {
            Some(Token::Identifier(s)) => s.clone(),
            _ => return Err(anyhow!("Expected variable name")),
        };

        self.expect_token(&Token::In)?;

        let iterable = self.parse_expression()?;

        self.expect_token(&Token::LeftBrace)?;
        let body = self.parse_block()?;
        self.expect_token(&Token::RightBrace)?;

        Ok(Statement::ForLoop(ForLoop {
            variable,
            iterable,
            body,
        }))
    }

    fn parse_match_expression(&mut self) -> Result<Statement> {
        self.expect_token(&Token::Match)?;

        let value = self.parse_expression()?;

        self.expect_token(&Token::LeftBrace)?;

        let mut arms = Vec::new();
        while !self.match_token(&Token::RightBrace) && !self.is_at_end() {
            let pattern = self.parse_pattern()?;
            self.expect_token(&Token::FatArrow)?;
            self.expect_token(&Token::LeftBrace)?;
            let body = self.parse_block()?;
            self.expect_token(&Token::RightBrace)?;

            arms.push(MatchArm { pattern, body });

            if self.match_token(&Token::Comma) {
                self.advance();
            }
        }

        self.expect_token(&Token::RightBrace)?;

        Ok(Statement::MatchExpression(MatchExpression { value, arms }))
    }

    fn parse_pattern(&mut self) -> Result<Pattern> {
        match self.advance() {
            Some(Token::Identifier(s)) => Ok(Pattern::Identifier(s.clone())),
            Some(Token::String(s)) => Ok(Pattern::Literal(Literal::String(s.clone()))),
            Some(Token::Integer(n)) => Ok(Pattern::Literal(Literal::Integer(*n))),
            _ => Ok(Pattern::Wildcard),
        }
    }

    // Helper methods
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn advance(&mut self) -> Option<&Token> {
        if !self.is_at_end() {
            let token = &self.tokens[self.position];
            self.position += 1;
            Some(token)
        } else {
            None
        }
    }

    fn match_token(&self, expected: &Token) -> bool {
        if let Some(token) = self.peek() {
            std::mem::discriminant(token) == std::mem::discriminant(expected)
        } else {
            false
        }
    }

    fn expect_token(&mut self, expected: &Token) -> Result<()> {
        if self.match_token(expected) {
            self.advance();
            Ok(())
        } else {
            Err(anyhow!(
                "Expected {:?}, found {:?}",
                expected,
                self.peek()
            ))
        }
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.tokens.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    #[test]
    fn test_parse_simple_command() {
        let tokens = Lexer::tokenize("ls -la").unwrap();
        let mut parser = Parser::new(tokens);
        let statements = parser.parse().unwrap();

        assert_eq!(statements.len(), 1);
        match &statements[0] {
            Statement::Command(cmd) => {
                assert_eq!(cmd.name, "ls");
                assert_eq!(cmd.args.len(), 1);
            }
            _ => panic!("Expected command"),
        }
    }

    #[test]
    fn test_parse_pipeline() {
        let tokens = Lexer::tokenize("ls | grep foo").unwrap();
        let mut parser = Parser::new(tokens);
        let statements = parser.parse().unwrap();

        assert_eq!(statements.len(), 1);
        match &statements[0] {
            Statement::Pipeline(pipeline) => {
                assert_eq!(pipeline.commands.len(), 2);
            }
            _ => panic!("Expected pipeline"),
        }
    }

    #[test]
    fn test_parse_assignment() {
        let tokens = Lexer::tokenize("let x = 42").unwrap();
        let mut parser = Parser::new(tokens);
        let statements = parser.parse().unwrap();

        assert_eq!(statements.len(), 1);
        match &statements[0] {
            Statement::Assignment(assignment) => {
                assert_eq!(assignment.name, "x");
            }
            _ => panic!("Expected assignment"),
        }
    }
}
