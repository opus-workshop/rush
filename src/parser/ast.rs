use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    Command(Command),
    Pipeline(Pipeline),
    ParallelExecution(ParallelExecution),
    Assignment(Assignment),
    FunctionDef(FunctionDef),
    IfStatement(IfStatement),
    ForLoop(ForLoop),
    MatchExpression(MatchExpression),
    ConditionalAnd(ConditionalAnd),
    ConditionalOr(ConditionalOr),
    Subshell(Vec<Statement>),
    BackgroundCommand(Box<Statement>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Command {
    pub name: String,
    pub args: Vec<Argument>,
    pub redirects: Vec<Redirect>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pipeline {
    pub commands: Vec<Command>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParallelExecution {
    pub commands: Vec<Command>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Assignment {
    pub name: String,
    pub value: Expression,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionDef {
    pub name: String,
    pub params: Vec<Parameter>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub type_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IfStatement {
    pub condition: Expression,
    pub then_block: Vec<Statement>,
    pub else_block: Option<Vec<Statement>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForLoop {
    pub variable: String,
    pub iterable: Expression,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchExpression {
    pub value: Expression,
    pub arms: Vec<MatchArm>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConditionalAnd {
    pub left: Box<Statement>,
    pub right: Box<Statement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConditionalOr {
    pub left: Box<Statement>,
    pub right: Box<Statement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Pattern {
    Identifier(String),
    Literal(Literal),
    Wildcard,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    Literal(Literal),
    Variable(String),
    BinaryOp(BinaryOp),
    UnaryOp(UnaryOp),
    FunctionCall(FunctionCall),
    CommandSubstitution(String),
    MemberAccess(MemberAccess),
    VariableExpansion(VarExpansion),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VarExpansion {
    pub name: String,
    pub operator: VarExpansionOp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VarExpansionOp {
    /// ${VAR} - Simple expansion
    Simple,
    /// ${VAR:-default} - Use default if unset
    UseDefault(String),
    /// ${VAR:=default} - Assign default if unset
    AssignDefault(String),
    /// ${VAR:?error} - Error if unset
    ErrorIfUnset(String),
    /// ${VAR#pattern} - Remove shortest prefix match
    RemoveShortestPrefix(String),
    /// ${VAR##pattern} - Remove longest prefix match
    RemoveLongestPrefix(String),
    /// ${VAR%pattern} - Remove shortest suffix match
    RemoveShortestSuffix(String),
    /// ${VAR%%pattern} - Remove longest suffix match
    RemoveLongestSuffix(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BinaryOp {
    pub left: Box<Expression>,
    pub op: BinaryOperator,
    pub right: Box<Expression>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnaryOp {
    pub op: UnaryOperator,
    pub operand: Box<Expression>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnaryOperator {
    Not,
    Negate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub args: Vec<Expression>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemberAccess {
    pub object: Box<Expression>,
    pub member: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Argument {
    Literal(String),
    Variable(String),
    BracedVariable(String),
    CommandSubstitution(String),
    Flag(String),
    Path(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Redirect {
    pub kind: RedirectKind,
    pub target: Option<String>, // None for special cases like 2>&1
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RedirectKind {
    Stdout,          // >
    StdoutAppend,    // >>
    Stdin,           // <
    Stderr,          // 2>
    StderrToStdout,  // 2>&1
    Both,            // &>
}
