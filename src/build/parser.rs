use std::fmt;

use super::lexer::{Token, Type};

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub items: Vec<Item>,
}

impl Program {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut writer = ByteWriter::new();
        writer.bytes(b"CRST");
        writer.u8(1);
        writer.items(&self.items);
        writer.finish()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let mut reader = ByteReader::new(bytes);
        if reader.take(4)? != b"CRST" {
            return Err("invalid CRST header".to_string());
        }
        if reader.u8()? != 1 {
            return Err("unsupported CRST format version".to_string());
        }
        let program = Program {
            items: reader.items()?,
        };
        if !reader.is_empty() {
            return Err("trailing bytes after CRST program".to_string());
        }
        Ok(program)
    }
}

struct ByteWriter {
    bytes: Vec<u8>,
}

impl ByteWriter {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }

    fn bytes(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u32(&mut self, value: usize) {
        self.bytes.extend_from_slice(&(value as u32).to_le_bytes());
    }

    fn i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn f64(&mut self, value: f64) {
        self.bytes.extend_from_slice(&value.to_bits().to_le_bytes());
    }

    fn string(&mut self, value: &str) {
        self.u32(value.len());
        self.bytes(value.as_bytes());
    }

    fn items(&mut self, items: &[Item]) {
        self.u32(items.len());
        for item in items {
            self.item(item);
        }
    }

    fn item(&mut self, item: &Item) {
        match item {
            Item::Statement(statement) => {
                self.u8(0);
                self.statement(statement);
            }
            Item::Function(function) => {
                self.u8(1);
                self.function(function);
            }
            Item::Object(object) => {
                self.u8(2);
                self.string(&object.name);
                self.u32(object.fields.len());
                for field in &object.fields {
                    self.string(&field.name);
                    self.u8(field.mutable as u8);
                    self.u8(field.private as u8);
                }
                match &object.constructor {
                    Some(constructor) => {
                        self.u8(1);
                        self.function(constructor);
                    }
                    None => self.u8(0),
                }
                self.u32(object.methods.len());
                for method in &object.methods {
                    self.function(method);
                }
            }
            Item::Namespace(namespace) => {
                self.u8(3);
                self.string(&namespace.name);
                self.items(&namespace.items);
            }
            Item::Entry(statements) => {
                self.u8(4);
                self.statements(statements);
            }
        }
    }

    fn token(&mut self, token: &Token) {
        self.type_tag(&token.token_type);
        self.u32(token.line);
        self.u32(token.column);
        self.string(&token.file);
    }

    fn type_tag(&mut self, token_type: &Type) {
        match token_type {
            Type::EOL => self.u8(0),
            Type::EOF => self.u8(1),
            Type::Mut => self.u8(2),
            Type::Immut => self.u8(3),
            Type::Ref => self.u8(4),
            Type::Func => self.u8(5),
            Type::Private => self.u8(6),
            Type::Colon => self.u8(7),
            Type::DoubleColon => self.u8(8),
            Type::ColonArrow => self.u8(9),
            Type::If => self.u8(10),
            Type::Else => self.u8(11),
            Type::While => self.u8(12),
            Type::Break => self.u8(13),
            Type::Continue => self.u8(14),
            Type::Ret => self.u8(15),
            Type::Object => self.u8(16),
            Type::Namespace => self.u8(17),
            Type::Entry => self.u8(18),
            Type::Constructor => self.u8(19),
            Type::Identifier(value) => {
                self.u8(20);
                self.string(value);
            }
            Type::Integer(value) => {
                self.u8(21);
                self.i64(*value);
            }
            Type::Float(value) => {
                self.u8(22);
                self.f64(*value);
            }
            Type::String(value) => {
                self.u8(23);
                self.string(value);
            }
            Type::Char(value) => {
                self.u8(24);
                self.u32(*value as usize);
            }
            Type::Boolean(value) => {
                self.u8(25);
                self.u8(*value as u8);
            }
            Type::Get(tokens) => {
                self.u8(26);
                self.u32(tokens.len());
                for token in tokens {
                    self.token(token);
                }
            }
            Type::LParen => self.u8(27),
            Type::RParen => self.u8(28),
            Type::LBrace => self.u8(29),
            Type::RBrace => self.u8(30),
            Type::Comma => self.u8(31),
            Type::Equals => self.u8(32),
            Type::DoubleEquals => self.u8(33),
            Type::BangEquals => self.u8(34),
            Type::LessThan => self.u8(35),
            Type::GreaterThan => self.u8(36),
            Type::LessThanOrEqual => self.u8(37),
            Type::GreaterThanOrEqual => self.u8(38),
            Type::Plus => self.u8(39),
            Type::Minus => self.u8(40),
            Type::Star => self.u8(41),
            Type::Slash => self.u8(42),
            Type::Percent => self.u8(43),
            Type::Exponent => self.u8(44),
            Type::Bang => self.u8(45),
            Type::DoubleAmpersand => self.u8(46),
            Type::DoublePipe => self.u8(47),
        }
    }

    fn function(&mut self, function: &Function) {
        self.string(&function.name);
        self.u8(function.private as u8);
        self.u32(function.parameters.len());
        for parameter in &function.parameters {
            self.string(&parameter.name);
            self.u8(parameter.mutable as u8);
            self.u8(parameter.reference as u8);
        }
        self.statements(&function.body);
    }

    fn statements(&mut self, statements: &[Stmt]) {
        self.u32(statements.len());
        for statement in statements {
            self.statement(statement);
        }
    }

    fn statement(&mut self, statement: &Stmt) {
        match statement {
            Stmt::Let {
                mutable,
                name,
                value,
            } => {
                self.u8(0);
                self.u8(*mutable as u8);
                self.string(name);
                self.expression(value);
            }
            Stmt::Assign { target, value } => {
                self.u8(1);
                self.expression(target);
                self.expression(value);
            }
            Stmt::Expr(expression) => {
                self.u8(2);
                self.expression(expression);
            }
            Stmt::If {
                condition,
                body,
                else_body,
            } => {
                self.u8(3);
                self.expression(condition);
                self.statements(body);
                match else_body {
                    Some(statements) => {
                        self.u8(1);
                        self.statements(statements);
                    }
                    None => self.u8(0),
                }
            }
            Stmt::While { condition, body } => {
                self.u8(4);
                self.expression(condition);
                self.statements(body);
            }
            Stmt::Break => self.u8(5),
            Stmt::Continue => self.u8(6),
            Stmt::Return(expression) => {
                self.u8(7);
                match expression {
                    Some(expression) => {
                        self.u8(1);
                        self.expression(expression);
                    }
                    None => self.u8(0),
                }
            }
            Stmt::Block(statements) => {
                self.u8(8);
                self.statements(statements);
            }
        }
    }

    fn expression(&mut self, expression: &Expr) {
        match expression {
            Expr::Integer(value) => {
                self.u8(0);
                self.i64(*value);
            }
            Expr::Float(value) => {
                self.u8(1);
                self.f64(*value);
            }
            Expr::String(value) => {
                self.u8(2);
                self.string(value);
            }
            Expr::Char(value) => {
                self.u8(3);
                self.u32(*value as usize);
            }
            Expr::Boolean(value) => {
                self.u8(4);
                self.u8(*value as u8);
            }
            Expr::Name(value) => {
                self.u8(5);
                self.string(value);
            }
            Expr::Unary { operator, value } => {
                self.u8(6);
                self.u8(match operator {
                    UnaryOp::Not => 0,
                    UnaryOp::Negate => 1,
                });
                self.expression(value);
            }
            Expr::Binary {
                left,
                operator,
                right,
            } => {
                self.u8(7);
                self.u8(*operator as u8);
                self.expression(left);
                self.expression(right);
            }
            Expr::Call { callee, arguments } => {
                self.u8(8);
                self.expression(callee);
                self.u32(arguments.len());
                for argument in arguments {
                    self.expression(argument);
                }
            }
            Expr::Access {
                value,
                operator,
                member,
            } => {
                self.u8(9);
                self.u8(match operator {
                    AccessOp::Namespace => 0,
                    AccessOp::Member => 1,
                });
                self.expression(value);
                self.string(member);
            }
            Expr::Group(value) => {
                self.u8(10);
                self.expression(value);
            }
        }
    }
}

struct ByteReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> ByteReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], String> {
        let end = self
            .position
            .checked_add(length)
            .ok_or("invalid byte length")?;
        if end > self.bytes.len() {
            return Err("unexpected end of CRST data".to_string());
        }
        let value = &self.bytes[self.position..end];
        self.position = end;
        Ok(value)
    }

    fn is_empty(&self) -> bool {
        self.position == self.bytes.len()
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> Result<usize, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()) as usize)
    }
    fn i64(&mut self) -> Result<i64, String> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn f64(&mut self) -> Result<f64, String> {
        Ok(f64::from_bits(u64::from_le_bytes(
            self.take(8)?.try_into().unwrap(),
        )))
    }
    fn string(&mut self) -> Result<String, String> {
        let length = self.u32()?;
        String::from_utf8(self.take(length)?.to_vec())
            .map_err(|_| "invalid UTF-8 string in CRST data".to_string())
    }
    fn flag(&mut self) -> Result<bool, String> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err("invalid boolean flag in CRST data".to_string()),
        }
    }

    fn items(&mut self) -> Result<Vec<Item>, String> {
        let count = self.u32()?;
        (0..count).map(|_| self.item()).collect()
    }
    fn item(&mut self) -> Result<Item, String> {
        match self.u8()? {
            0 => Ok(Item::Statement(self.statement()?)),
            1 => Ok(Item::Function(self.function()?)),
            2 => {
                let name = self.string()?;
                let field_count = self.u32()?;
                let mut fields = Vec::with_capacity(field_count);
                for _ in 0..field_count {
                    fields.push(Field {
                        name: self.string()?,
                        mutable: self.flag()?,
                        private: self.flag()?,
                    });
                }
                let constructor = if self.flag()? {
                    Some(self.function()?)
                } else {
                    None
                };
                let method_count = self.u32()?;
                let mut methods = Vec::with_capacity(method_count);
                for _ in 0..method_count {
                    methods.push(self.function()?);
                }
                Ok(Item::Object(Object {
                    name,
                    fields,
                    constructor,
                    methods,
                }))
            }
            3 => Ok(Item::Namespace(Namespace {
                name: self.string()?,
                items: self.items()?,
            })),
            4 => Ok(Item::Entry(self.statements()?)),
            tag => Err(format!("unknown item tag {tag}")),
        }
    }

    fn token(&mut self) -> Result<Token, String> {
        Ok(Token {
            token_type: self.token_type()?,
            line: self.u32()?,
            column: self.u32()?,
            file: self.string()?,
        })
    }
    fn token_type(&mut self) -> Result<Type, String> {
        Ok(match self.u8()? {
            0 => Type::EOL,
            1 => Type::EOF,
            2 => Type::Mut,
            3 => Type::Immut,
            4 => Type::Ref,
            5 => Type::Func,
            6 => Type::Private,
            7 => Type::Colon,
            8 => Type::DoubleColon,
            9 => Type::ColonArrow,
            10 => Type::If,
            11 => Type::Else,
            12 => Type::While,
            13 => Type::Break,
            14 => Type::Continue,
            15 => Type::Ret,
            16 => Type::Object,
            17 => Type::Namespace,
            18 => Type::Entry,
            19 => Type::Constructor,
            20 => Type::Identifier(self.string()?),
            21 => Type::Integer(self.i64()?),
            22 => Type::Float(self.f64()?),
            23 => Type::String(self.string()?),
            24 => Type::Char(char::from_u32(self.u32()? as u32).ok_or("invalid character")?),
            25 => Type::Boolean(self.flag()?),
            26 => {
                let count = self.u32()?;
                let mut tokens = Vec::with_capacity(count);
                for _ in 0..count {
                    tokens.push(self.token()?);
                }
                Type::Get(tokens)
            }
            27 => Type::LParen,
            28 => Type::RParen,
            29 => Type::LBrace,
            30 => Type::RBrace,
            31 => Type::Comma,
            32 => Type::Equals,
            33 => Type::DoubleEquals,
            34 => Type::BangEquals,
            35 => Type::LessThan,
            36 => Type::GreaterThan,
            37 => Type::LessThanOrEqual,
            38 => Type::GreaterThanOrEqual,
            39 => Type::Plus,
            40 => Type::Minus,
            41 => Type::Star,
            42 => Type::Slash,
            43 => Type::Percent,
            44 => Type::Exponent,
            45 => Type::Bang,
            46 => Type::DoubleAmpersand,
            47 => Type::DoublePipe,
            tag => return Err(format!("unknown token tag {tag}")),
        })
    }

    fn function(&mut self) -> Result<Function, String> {
        let name = self.string()?;
        let private = self.flag()?;
        let count = self.u32()?;
        let mut parameters = Vec::with_capacity(count);
        for _ in 0..count {
            parameters.push(Parameter {
                name: self.string()?,
                mutable: self.flag()?,
                reference: self.flag()?,
            });
        }
        Ok(Function {
            name,
            private,
            parameters,
            body: self.statements()?,
        })
    }
    fn statements(&mut self) -> Result<Vec<Stmt>, String> {
        let count = self.u32()?;
        (0..count).map(|_| self.statement()).collect()
    }
    fn statement(&mut self) -> Result<Stmt, String> {
        Ok(match self.u8()? {
            0 => Stmt::Let {
                mutable: self.flag()?,
                name: self.string()?,
                value: self.expression()?,
            },
            1 => Stmt::Assign {
                target: self.expression()?,
                value: self.expression()?,
            },
            2 => Stmt::Expr(self.expression()?),
            3 => Stmt::If {
                condition: self.expression()?,
                body: self.statements()?,
                else_body: if self.flag()? {
                    Some(self.statements()?)
                } else {
                    None
                },
            },
            4 => Stmt::While {
                condition: self.expression()?,
                body: self.statements()?,
            },
            5 => Stmt::Break,
            6 => Stmt::Continue,
            7 => Stmt::Return(if self.flag()? {
                Some(self.expression()?)
            } else {
                None
            }),
            8 => Stmt::Block(self.statements()?),
            tag => return Err(format!("unknown statement tag {tag}")),
        })
    }
    fn expression(&mut self) -> Result<Expr, String> {
        Ok(match self.u8()? {
            0 => Expr::Integer(self.i64()?),
            1 => Expr::Float(self.f64()?),
            2 => Expr::String(self.string()?),
            3 => Expr::Char(char::from_u32(self.u32()? as u32).ok_or("invalid character")?),
            4 => Expr::Boolean(self.flag()?),
            5 => Expr::Name(self.string()?),
            6 => Expr::Unary {
                operator: match self.u8()? {
                    0 => UnaryOp::Not,
                    1 => UnaryOp::Negate,
                    tag => return Err(format!("unknown unary operator tag {tag}")),
                },
                value: Box::new(self.expression()?),
            },
            7 => {
                let operator = binary_operator(self.u8()?)?;
                let left = Box::new(self.expression()?);
                let right = Box::new(self.expression()?);
                Expr::Binary {
                    left,
                    operator,
                    right,
                }
            }
            8 => {
                let callee = Box::new(self.expression()?);
                let count = self.u32()?;
                let mut arguments = Vec::with_capacity(count);
                for _ in 0..count {
                    arguments.push(self.expression()?);
                }
                Expr::Call { callee, arguments }
            }
            9 => {
                let operator = match self.u8()? {
                    0 => AccessOp::Namespace,
                    1 => AccessOp::Member,
                    tag => return Err(format!("unknown access operator tag {tag}")),
                };
                let value = Box::new(self.expression()?);
                let member = self.string()?;
                Expr::Access {
                    value,
                    operator,
                    member,
                }
            }
            10 => Expr::Group(Box::new(self.expression()?)),
            tag => return Err(format!("unknown expression tag {tag}")),
        })
    }
}

fn binary_operator(tag: u8) -> Result<BinaryOp, String> {
    match tag {
        0 => Ok(BinaryOp::Add),
        1 => Ok(BinaryOp::Subtract),
        2 => Ok(BinaryOp::Multiply),
        3 => Ok(BinaryOp::Divide),
        4 => Ok(BinaryOp::Remainder),
        5 => Ok(BinaryOp::Exponent),
        6 => Ok(BinaryOp::Equal),
        7 => Ok(BinaryOp::NotEqual),
        8 => Ok(BinaryOp::Less),
        9 => Ok(BinaryOp::LessOrEqual),
        10 => Ok(BinaryOp::Greater),
        11 => Ok(BinaryOp::GreaterOrEqual),
        12 => Ok(BinaryOp::And),
        13 => Ok(BinaryOp::Or),
        _ => Err(format!("unknown binary operator tag {tag}")),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Statement(Stmt),
    Function(Function),
    Object(Object),
    Namespace(Namespace),
    Entry(Vec<Stmt>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub private: bool,
    pub parameters: Vec<Parameter>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub mutable: bool,
    pub reference: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub name: String,
    pub fields: Vec<Field>,
    pub constructor: Option<Function>,
    pub methods: Vec<Function>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: String,
    pub mutable: bool,
    pub private: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Namespace {
    pub name: String,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let {
        mutable: bool,
        name: String,
        value: Expr,
    },
    Assign {
        target: Expr,
        value: Expr,
    },
    Expr(Expr),
    If {
        condition: Expr,
        body: Vec<Stmt>,
        else_body: Option<Vec<Stmt>>,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    Break,
    Continue,
    Return(Option<Expr>),
    Block(Vec<Stmt>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Integer(i64),
    Float(f64),
    String(String),
    Char(char),
    Boolean(bool),
    Name(String),
    Unary {
        operator: UnaryOp,
        value: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        operator: BinaryOp,
        right: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        arguments: Vec<Expr>,
    },
    Access {
        value: Box<Expr>,
        operator: AccessOp,
        member: String,
    },
    Group(Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Not,
    Negate,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Exponent,
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AccessOp {
    Namespace,
    Member,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub file: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} at {} {}:{}",
            self.message, self.file, self.line, self.column
        )
    }
}
impl std::error::Error for ParseError {}

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

    pub fn parse(mut self) -> Result<Program, ParseError> {
        let mut items = Vec::new();
        self.skip_eol();
        while !self.at_end() {
            items.push(self.parse_item()?);
            self.skip_eol();
        }
        Ok(Program { items })
    }

    fn parse_item(&mut self) -> Result<Item, ParseError> {
        match self.peek_type() {
            Some(Type::Get(tokens)) => {
                let tokens = tokens.clone();
                self.advance();
                self.tokens.splice(self.position..self.position, tokens);
                self.skip_eol();
                self.parse_item()
            }
            Some(Type::Entry) => {
                self.advance();
                Ok(Item::Entry(self.parse_block()?))
            }
            Some(Type::Object) => self.parse_object().map(Item::Object),
            Some(Type::Namespace) => self.parse_namespace().map(Item::Namespace),
            Some(Type::Private) | Some(Type::Func) => self.parse_function().map(Item::Function),
            _ => self.parse_statement().map(Item::Statement),
        }
    }

    fn parse_object(&mut self) -> Result<Object, ParseError> {
        self.expect_simple(Type::Object)?;
        let name = self.expect_identifier()?;
        self.expect_simple(Type::LBrace)?;
        let mut fields = Vec::new();
        let mut constructor = None;
        let mut methods = Vec::new();
        self.skip_eol();
        while !self.check(&Type::RBrace) {
            if self.consume_simple(Type::Constructor) {
                constructor =
                    Some(self.parse_function_after_name("constructor".to_string(), false)?);
                self.skip_eol();
                continue;
            }
            let mutable = self.consume_simple(Type::Mut);
            let (member_name, private) = if self.consume_simple(Type::Func) {
                let private = if self.consume_simple(Type::Colon) {
                    self.consume_simple(Type::Private)
                } else {
                    false
                };
                (self.expect_identifier()?, private)
            } else {
                let private = if self.consume_simple(Type::Colon) {
                    self.consume_simple(Type::Private)
                } else {
                    false
                };
                (self.expect_identifier()?, private)
            };
            if self.check(&Type::LParen) {
                methods.push(self.parse_function_after_name(member_name, private)?);
            } else {
                fields.push(Field {
                    name: member_name,
                    mutable,
                    private,
                });
            }
            self.consume_eol();
            self.skip_eol();
        }
        self.expect_simple(Type::RBrace)?;
        Ok(Object {
            name,
            fields,
            constructor,
            methods,
        })
    }

    fn parse_namespace(&mut self) -> Result<Namespace, ParseError> {
        self.expect_simple(Type::Namespace)?;
        let name = self.expect_identifier()?;
        self.expect_simple(Type::LBrace)?;
        let mut items = Vec::new();
        self.skip_eol();
        while !self.check(&Type::RBrace) {
            items.push(self.parse_item()?);
            self.skip_eol();
        }
        self.expect_simple(Type::RBrace)?;
        Ok(Namespace { name, items })
    }

    fn parse_function(&mut self) -> Result<Function, ParseError> {
        let private = self.consume_simple(Type::Private);
        self.expect_simple(Type::Func)?;
        let name = self.expect_identifier()?;
        self.parse_function_after_name(name, private)
    }

    fn parse_function_after_name(
        &mut self,
        name: String,
        private: bool,
    ) -> Result<Function, ParseError> {
        self.expect_simple(Type::LParen)?;
        let mut parameters = Vec::new();
        while !self.check(&Type::RParen) {
            let mutable = self.consume_simple(Type::Mut) || self.consume_simple(Type::Immut);
            let reference = if self.consume_simple(Type::Colon) {
                self.consume_simple(Type::Ref)
            } else {
                false
            };
            parameters.push(Parameter {
                name: self.expect_identifier()?,
                mutable,
                reference,
            });
            if !self.consume_simple(Type::Comma) {
                break;
            }
        }
        self.expect_simple(Type::RParen)?;
        Ok(Function {
            name,
            private,
            parameters,
            body: self.parse_block()?,
        })
    }

    fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        match self.peek_type() {
            Some(Type::Mut) | Some(Type::Immut) => {
                let mutable = self.consume_simple(Type::Mut);
                if !mutable {
                    self.expect_simple(Type::Immut)?;
                }
                let name = self.expect_identifier()?;
                self.expect_simple(Type::Equals)?;
                Ok(Stmt::Let {
                    mutable,
                    name,
                    value: self.parse_expression()?,
                })
            }
            Some(Type::If) => self.parse_if(),
            Some(Type::While) => {
                self.advance();
                let condition = self.parse_expression()?;
                Ok(Stmt::While {
                    condition,
                    body: self.parse_block()?,
                })
            }
            Some(Type::Break) => {
                self.advance();
                Ok(Stmt::Break)
            }
            Some(Type::Continue) => {
                self.advance();
                Ok(Stmt::Continue)
            }
            Some(Type::Ret) => {
                self.advance();
                if self.is_statement_end() {
                    Ok(Stmt::Return(None))
                } else {
                    Ok(Stmt::Return(Some(self.parse_expression()?)))
                }
            }
            Some(Type::LBrace) => Ok(Stmt::Block(self.parse_block()?)),
            _ => {
                let target = self.parse_expression()?;
                if self.consume_simple(Type::Equals) {
                    Ok(Stmt::Assign {
                        target,
                        value: self.parse_expression()?,
                    })
                } else {
                    Ok(Stmt::Expr(target))
                }
            }
        }
    }

    fn parse_if(&mut self) -> Result<Stmt, ParseError> {
        self.expect_simple(Type::If)?;
        let condition = self.parse_expression()?;
        let body = self.parse_block()?;
        self.skip_eol();
        let else_body = if self.consume_simple(Type::Else) {
            if self.check(&Type::If) {
                let else_if_stmt = self.parse_if()?;
                Some(vec![else_if_stmt])
            } else {
                Some(self.parse_block()?)
            }
        } else {
            None
        };
        Ok(Stmt::If {
            condition,
            body,
            else_body,
        })
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, ParseError> {
        self.expect_simple(Type::LBrace)?;
        let mut statements = Vec::new();
        self.skip_eol();
        while !self.check(&Type::RBrace) {
            statements.push(self.parse_statement()?);
            self.consume_eol();
            self.skip_eol();
        }
        self.expect_simple(Type::RBrace)?;
        Ok(statements)
    }

    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_binary(0)
    }

    fn parse_binary(&mut self, minimum_precedence: u8) -> Result<Expr, ParseError> {
        let mut left = self.parse_unary()?;
        loop {
            let Some((operator, precedence)) = self.binary_operator() else {
                break;
            };
            if precedence < minimum_precedence {
                break;
            }
            self.advance();
            let next_minimum = precedence
                + if matches!(operator, BinaryOp::Exponent) {
                    0
                } else {
                    1
                };
            let right = self.parse_binary(next_minimum)?;
            left = Expr::Binary {
                left: Box::new(left),
                operator,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        match self.peek_type() {
            Some(Type::Bang) => {
                self.advance();
                Ok(Expr::Unary {
                    operator: UnaryOp::Not,
                    value: Box::new(self.parse_unary()?),
                })
            }
            Some(Type::Minus) => {
                self.advance();
                Ok(Expr::Unary {
                    operator: UnaryOp::Negate,
                    value: Box::new(self.parse_unary()?),
                })
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let mut expression = match self.advance_type() {
            Some(Type::Integer(value)) => Expr::Integer(value),
            Some(Type::Float(value)) => Expr::Float(value),
            Some(Type::String(value)) => Expr::String(value),
            Some(Type::Char(value)) => Expr::Char(value),
            Some(Type::Boolean(value)) => Expr::Boolean(value),
            Some(Type::Identifier(value)) => Expr::Name(value),
            Some(Type::LParen) => {
                let value = self.parse_expression()?;
                self.expect_simple(Type::RParen)?;
                Expr::Group(Box::new(value))
            }
            _ => return Err(self.error("expected an expression")),
        };
        loop {
            if self.consume_simple(Type::LParen) {
                let mut arguments = Vec::new();
                while !self.check(&Type::RParen) {
                    arguments.push(self.parse_expression()?);
                    if !self.consume_simple(Type::Comma) {
                        break;
                    }
                }
                self.expect_simple(Type::RParen)?;
                expression = Expr::Call {
                    callee: Box::new(expression),
                    arguments,
                };
            } else if self.consume_simple(Type::DoubleColon)
                || self.consume_simple(Type::ColonArrow)
            {
                let operator = if self.tokens[self.position - 1].token_type == Type::DoubleColon {
                    AccessOp::Namespace
                } else {
                    AccessOp::Member
                };
                expression = Expr::Access {
                    value: Box::new(expression),
                    operator,
                    member: self.expect_identifier()?,
                };
            } else {
                break;
            }
        }
        Ok(expression)
    }

    fn binary_operator(&self) -> Option<(BinaryOp, u8)> {
        match self.peek_type() {
            Some(Type::DoublePipe) => Some((BinaryOp::Or, 1)),
            Some(Type::DoubleAmpersand) => Some((BinaryOp::And, 2)),
            Some(Type::DoubleEquals) => Some((BinaryOp::Equal, 3)),
            Some(Type::BangEquals) => Some((BinaryOp::NotEqual, 3)),
            Some(Type::LessThan) => Some((BinaryOp::Less, 4)),
            Some(Type::LessThanOrEqual) => Some((BinaryOp::LessOrEqual, 4)),
            Some(Type::GreaterThan) => Some((BinaryOp::Greater, 4)),
            Some(Type::GreaterThanOrEqual) => Some((BinaryOp::GreaterOrEqual, 4)),
            Some(Type::Plus) => Some((BinaryOp::Add, 5)),
            Some(Type::Minus) => Some((BinaryOp::Subtract, 5)),
            Some(Type::Star) => Some((BinaryOp::Multiply, 6)),
            Some(Type::Slash) => Some((BinaryOp::Divide, 6)),
            Some(Type::Percent) => Some((BinaryOp::Remainder, 6)),
            Some(Type::Exponent) => Some((BinaryOp::Exponent, 7)),
            _ => None,
        }
    }

    fn expect_identifier(&mut self) -> Result<String, ParseError> {
        match self.advance_type() {
            Some(Type::Identifier(name)) => Ok(name),
            _ => Err(self.error("expected an identifier")),
        }
    }

    fn expect_simple(&mut self, expected: Type) -> Result<(), ParseError> {
        if self.consume_simple(expected.clone()) {
            Ok(())
        } else {
            Err(self.error(&format!("expected {:?}", expected)))
        }
    }

    fn consume_simple(&mut self, expected: Type) -> bool {
        if self.check(&expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check(&self, expected: &Type) -> bool {
        self.peek_type().map_or(false, |actual| {
            std::mem::discriminant(actual) == std::mem::discriminant(expected)
        })
    }

    fn advance_type(&mut self) -> Option<Type> {
        let token = self.tokens.get(self.position)?.token_type.clone();
        self.position += 1;
        Some(token)
    }

    fn advance(&mut self) {
        self.position += 1;
    }

    fn peek_type(&self) -> Option<&Type> {
        self.tokens
            .get(self.position)
            .map(|token| &token.token_type)
    }

    fn at_end(&self) -> bool {
        self.position >= self.tokens.len()
    }

    fn skip_eol(&mut self) {
        while self.consume_simple(Type::EOL) {}
    }

    fn consume_eol(&mut self) -> bool {
        self.consume_simple(Type::EOL)
    }

    fn is_statement_end(&self) -> bool {
        self.at_end() || self.check(&Type::EOL) || self.check(&Type::RBrace)
    }

    fn error(&self, message: &str) -> ParseError {
        let token = self
            .tokens
            .get(self.position)
            .or_else(|| self.tokens.last());
        ParseError {
            message: message.to_string(),
            line: token.map_or(0, |token| token.line),
            column: token.map_or(0, |token| token.column),
            file: token.map_or_else(String::new, |token| token.file.clone()),
        }
    }
}
