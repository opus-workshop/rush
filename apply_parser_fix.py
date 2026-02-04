#!/usr/bin/env python3
"""
Applies the fix for piping into while/until/compound commands.
Bean 211.7: PARSER-BUG: Pipes into while/until/compound commands not supported
"""

import re

# Fix 1: Update AST - add BraceGroup to Statement
def fix_ast(content):
    # Add BraceGroup to Statement enum
    content = content.replace(
        '    Subshell(Vec<Statement>),\n    BackgroundCommand(Box<Statement>),\n}',
        '''    Subshell(Vec<Statement>),
    BackgroundCommand(Box<Statement>),
    /// Brace group: { commands; } - executes in current shell context
    BraceGroup(Vec<Statement>),
}'''
    )
    
    # Add CompoundCommand to PipelineElement enum
    content = content.replace(
        '''/// An element in a pipeline - either a regular command or a subshell
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PipelineElement {
    Command(Command),
    Subshell(Vec<Statement>),
}''',
        '''/// An element in a pipeline - either a regular command, subshell, or compound command
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PipelineElement {
    Command(Command),
    Subshell(Vec<Statement>),
    /// Compound commands (while, until, for, if, case, brace groups) as pipeline elements
    CompoundCommand(Box<Statement>),
}'''
    )
    
    return content

# Fix 2: Update parser - modify parse_pipeline_element and add helpers
def fix_parser(content):
    # Replace parse_pipeline_element function
    old_func = '''    fn parse_pipeline_element(&mut self) -> Result<Statement> {
        if self.match_token(&Token::LeftParen) {
            self.parse_subshell()
        } else if self.is_bare_assignment() {
            self.parse_bare_assignment_or_command()
        } else {
            Ok(Statement::Command(self.parse_command()?))
        }
    }

    /// Check if current position has a `NAME=VALUE` pattern (bare assignment).'''
    
    new_func = '''    fn parse_pipeline_element(&mut self) -> Result<Statement> {
        // Check for compound commands first (can appear after pipe)
        match self.peek() {
            Some(Token::While) => return self.parse_while_loop(),
            Some(Token::Until) => return self.parse_until_loop(),
            Some(Token::For) => return self.parse_for_loop(),
            Some(Token::If) => return self.parse_if_statement(),
            Some(Token::Case) => return self.parse_case_statement(),
            Some(Token::LeftBrace) => return self.parse_brace_group(),
            Some(Token::LeftParen) => return self.parse_subshell(),
            _ => {}
        }
        
        if self.is_bare_assignment() {
            self.parse_bare_assignment_or_command()
        } else {
            Ok(Statement::Command(self.parse_command()?))
        }
    }
    
    /// Parse a brace group: { commands; }
    /// Executes in current shell context (unlike subshell which forks)
    fn parse_brace_group(&mut self) -> Result<Statement> {
        self.expect_token(&Token::LeftBrace)?;
        
        let mut statements = Vec::new();
        
        // Skip leading newlines
        while matches!(self.peek(), Some(Token::Newline) | Some(Token::CrLf)) {
            self.advance();
        }
        
        // Parse statements until we hit a closing brace
        while !self.match_token(&Token::RightBrace) && !self.is_at_end() {
            // Skip newlines between statements
            while matches!(self.peek(), Some(Token::Newline) | Some(Token::CrLf)) {
                self.advance();
            }
            
            if self.match_token(&Token::RightBrace) {
                break;
            }
            
            statements.push(self.parse_conditional_statement()?);
            
            // Handle statement separators (semicolon)
            if self.match_token(&Token::Semicolon) {
                self.advance();
            }
        }
        
        self.expect_token(&Token::RightBrace)?;
        
        Ok(Statement::BraceGroup(statements))
    }
    
    /// Convert a parsed statement into a pipeline element
    fn statement_to_pipeline_element(stmt: Statement) -> Result<PipelineElement> {
        match stmt {
            Statement::Command(cmd) => Ok(PipelineElement::Command(cmd)),
            Statement::Subshell(stmts) => Ok(PipelineElement::Subshell(stmts)),
            // Compound commands can be pipeline elements
            Statement::WhileLoop(_)
            | Statement::UntilLoop(_)
            | Statement::ForLoop(_)
            | Statement::IfStatement(_)
            | Statement::CaseStatement(_)
            | Statement::BraceGroup(_) => Ok(PipelineElement::CompoundCommand(Box::new(stmt))),
            _ => Err(anyhow!("This statement type cannot be used in pipelines")),
        }
    }

    /// Check if current position has a `NAME=VALUE` pattern (bare assignment).'''
    
    content = content.replace(old_func, new_func)
    
    # Update pipeline building to use helper
    content = content.replace(
        '''// Build elements list supporting both commands and subshells
            let first_element = match first_statement {
                Statement::Command(cmd) => PipelineElement::Command(cmd),
                Statement::Subshell(stmts) => PipelineElement::Subshell(stmts),
                _ => return Err(anyhow!("Only commands and subshells can be used in pipelines")),
            };''',
        '''// Build elements list supporting commands, subshells, and compound commands
            let first_element = Self::statement_to_pipeline_element(first_statement)?;'''
    )
    
    content = content.replace(
        '''let elem = match stmt {
                    Statement::Command(cmd) => PipelineElement::Command(cmd),
                    Statement::Subshell(stmts) => PipelineElement::Subshell(stmts),
                    _ => return Err(anyhow!("Only commands and subshells can be used in pipelines")),
                };''',
        '''let elem = Self::statement_to_pipeline_element(stmt)?;'''
    )
    
    # Update backward-compatible commands vec
    content = content.replace(
        'PipelineElement::Subshell(_) => None,',
        'PipelineElement::Subshell(_) | PipelineElement::CompoundCommand(_) => None,'
    )
    
    return content

def main():
    # Fix AST
    with open('src/parser/ast.rs', 'r') as f:
        ast_content = f.read()
    
    fixed_ast = fix_ast(ast_content)
    
    with open('src/parser/ast.rs', 'w') as f:
        f.write(fixed_ast)
    
    print("Fixed src/parser/ast.rs")
    
    # Fix parser
    with open('src/parser/mod.rs', 'r') as f:
        parser_content = f.read()
    
    fixed_parser = fix_parser(parser_content)
    
    with open('src/parser/mod.rs', 'w') as f:
        f.write(fixed_parser)
    
    print("Fixed src/parser/mod.rs")
    
    print("\nNow run: cargo build --release")
    print("Then test: ./target/release/rush -c 'echo hello | while read x; do echo got-$x; done'")

if __name__ == '__main__':
    main()
