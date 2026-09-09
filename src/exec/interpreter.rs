use std::{
    collections::HashMap,
    io::{Error, ErrorKind},
};

use crate::{
    build::parser::{AccessOp, BinaryOp, Expr, Item, Program, Stmt, UnaryOp},
    exec::crust_stdlib::eval_std_call,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    String(String),
    Char(char),
    Boolean(bool),
    Array(Vec<Value>),
    Callee(String, Box<Value>, AccessOp),
    Name(String),
    None,
}

pub type Env = HashMap<String, (Value, bool)>;

pub enum Flow {
    Normal,
    Return(Value),
    Break,
    Continue,
}

pub struct Interpreter {
    program: Program,
}

impl Interpreter {
    pub fn new(program: Program) -> Self {
        Interpreter { program }
    }

    pub fn run(&mut self, args: Vec<String>) {
        let entry = self.program.items.iter().find_map(|item| match item {
            Item::Entry(body) => Some(body.clone()),
            _ => None,
        });

        if let Some(body) = entry {
            let mut env: Env = HashMap::new();
            let executed = self.exec_block(&body, &mut env);
            if let Ok(flow) = executed {
                match flow {
                    Flow::Return(value) => {
                        println!("program returned: {:?}", value);
                    }
                    Flow::Normal => {}
                    _ => {
                        eprintln!("unexpected flow control encountered.");
                        std::process::exit(1);
                    }
                }
            } else if let Err(e) = executed {
                eprintln!("error during execution:\n{}", e);
                std::process::exit(1);
            }
        } else {
            eprintln!("no entry point found in the program.");
            std::process::exit(1);
        }
    }

    fn exec_block(&mut self, stmts: &[Stmt], env: &mut Env) -> Result<Flow, Error> {
        for stmt in stmts {
            let flow = self.exec_stmt(stmt, env)?;
            match flow {
                Flow::Normal => continue,
                _ => return Ok(flow),
            }
        }
        Ok(Flow::Normal)
    }

    fn exec_stmt(&mut self, stmt: &Stmt, env: &mut Env) -> Result<Flow, Error> {
        match stmt {
            Stmt::Expr(expr) => {
                self.eval_expr(expr, env)?;
                Ok(Flow::Normal)
            }
            Stmt::Return(expr) => {
                let value = if let Some(expr) = expr {
                    self.eval_expr(expr, env)?
                } else {
                    Value::None
                };
                Ok(Flow::Return(value))
            }
            _ => Err(self.runtime_error(format!(
                "statement execution is not implemented for {stmt:?} yet"
            ))),
        }
    }

    fn eval_expr(&mut self, expr: &Expr, env: &mut Env) -> Result<Value, Error> {
        match expr {
            Expr::Integer(value) => Ok(Value::Int(*value)),
            Expr::Float(value) => Ok(Value::Float(*value)),
            Expr::String(value) => Ok(Value::String(value.clone())),
            Expr::Char(value) => Ok(Value::Char(*value)),
            Expr::Boolean(value) => Ok(Value::Boolean(*value)),
            Expr::Name(name) => env
                .get(name)
                .map(|(value, _)| value.clone())
                .ok_or_else(|| self.runtime_error(format!("unknown variable '{name}'"))),
            Expr::Unary { operator, value } => {
                let value = self.eval_expr(value, env)?;
                match (operator, value) {
                    (UnaryOp::Not, Value::Boolean(value)) => Ok(Value::Boolean(!value)),
                    (UnaryOp::Negate, Value::Int(value)) => Ok(Value::Int(-value)),
                    (UnaryOp::Negate, Value::Float(value)) => Ok(Value::Float(-value)),
                    (UnaryOp::Not, value) => {
                        Err(self.runtime_error(format!("cannot apply ! to {value:?}")))
                    }
                    (UnaryOp::Negate, value) => {
                        Err(self.runtime_error(format!("cannot negate {value:?}")))
                    }
                }
            }
            Expr::Binary {
                left,
                operator,
                right,
            } => {
                let left = self.eval_expr(left, env)?;
                if *operator == BinaryOp::And && left == Value::Boolean(false) {
                    return Ok(Value::Boolean(false));
                }
                if *operator == BinaryOp::Or && left == Value::Boolean(true) {
                    return Ok(Value::Boolean(true));
                }
                let right = self.eval_expr(right, env)?;
                self.eval_binary(left, *operator, right)
            }
            Expr::Call { callee, arguments } => {
                let caller = match callee.as_ref() {
                    Expr::Name(name) => Ok(Value::Name(name.clone())),
                    _ => self.eval_expr(callee, env),
                };
                if let Ok(caller) = caller {
                    let mut args = vec![];
                    for arg in arguments {
                        let evaled_arg = self.eval_expr(arg, env)?;
                        args.push(evaled_arg);
                    }

                    match caller {
                        Value::Callee(function_name, calling_on, access_op) => {
                            match access_op {
                                AccessOp::Namespace => {
                                    match calling_on.as_ref() {
                                        Value::String(namespace_name) => {
                                            match namespace_name.as_str() {
                                                // check for builtin namespaces
                                                "std" => {
                                                    return eval_std_call(function_name, args, env);
                                                }
                                                "env" => unimplemented!(),
                                                _ => unimplemented!(),
                                            }
                                        }
                                        _ => {
                                            return Err(self
                                                .runtime_error("shouldn't happen!!!".to_string()));
                                        }
                                    }
                                }
                                AccessOp::Member => {
                                    return Err(self.runtime_error(
                                        "member calls are not implemented yet".to_string(),
                                    ));
                                }
                            }
                        }
                        _ => {
                            return Err(
                                self.runtime_error(format!("invalid callee value: {caller:?}"))
                            );
                        }
                    }
                } else {
                    return Err(
                        self.runtime_error("cannot call a function with a null callee".to_string())
                    );
                }
            }
            Expr::Access {
                operator: AccessOp::Namespace,
                value,
                member,
            } => {
                let namespace = match value.as_ref() {
                    Expr::Name(name) => Value::String(name.clone()),
                    _ => self.eval_expr(value, env)?,
                };
                Ok(Value::Callee(
                    member.clone(),
                    Box::new(namespace),
                    AccessOp::Namespace,
                ))
            }
            Expr::Access {
                operator: AccessOp::Member,
                value,
                member,
            } => {
                let receiver = match value.as_ref() {
                    Expr::Name(name) => Value::Name(name.clone()),
                    _ => self.eval_expr(value, env)?,
                };
                Ok(Value::Callee(
                    member.clone(),
                    Box::new(receiver),
                    AccessOp::Member,
                ))
            }
            Expr::Group(value) => self.eval_expr(value, env),
        }
    }

    fn eval_binary(&self, left: Value, operator: BinaryOp, right: Value) -> Result<Value, Error> {
        use BinaryOp::*;

        match (left, operator, right) {
            (Value::Int(left), Add, Value::Int(right)) => Ok(Value::Int(left + right)),
            (Value::Int(left), Subtract, Value::Int(right)) => Ok(Value::Int(left - right)),
            (Value::Int(left), Multiply, Value::Int(right)) => Ok(Value::Int(left * right)),
            (Value::Int(left), Divide, Value::Int(right)) if right != 0 => {
                Ok(Value::Int(left / right))
            }
            (Value::Int(left), Remainder, Value::Int(right)) if right != 0 => {
                Ok(Value::Int(left % right))
            }
            (Value::Int(left), Exponent, Value::Int(right)) if right >= 0 => {
                Ok(Value::Int(left.pow(right as u32)))
            }
            (Value::Float(left), Add, Value::Float(right)) => Ok(Value::Float(left + right)),
            (Value::Float(left), Subtract, Value::Float(right)) => Ok(Value::Float(left - right)),
            (Value::Float(left), Multiply, Value::Float(right)) => Ok(Value::Float(left * right)),
            (Value::Float(left), Divide, Value::Float(right)) if right != 0.0 => {
                Ok(Value::Float(left / right))
            }
            (Value::Float(left), Remainder, Value::Float(right)) if right != 0.0 => {
                Ok(Value::Float(left % right))
            }
            (Value::Float(left), Exponent, Value::Float(right)) => {
                Ok(Value::Float(left.powf(right)))
            }
            (left, Equal, right) => Ok(Value::Boolean(left == right)),
            (left, NotEqual, right) => Ok(Value::Boolean(left != right)),
            (Value::Int(left), Less, Value::Int(right)) => Ok(Value::Boolean(left < right)),
            (Value::Int(left), LessOrEqual, Value::Int(right)) => Ok(Value::Boolean(left <= right)),
            (Value::Int(left), Greater, Value::Int(right)) => Ok(Value::Boolean(left > right)),
            (Value::Int(left), GreaterOrEqual, Value::Int(right)) => {
                Ok(Value::Boolean(left >= right))
            }
            (Value::Float(left), Less, Value::Float(right)) => Ok(Value::Boolean(left < right)),
            (Value::Float(left), LessOrEqual, Value::Float(right)) => {
                Ok(Value::Boolean(left <= right))
            }
            (Value::Float(left), Greater, Value::Float(right)) => Ok(Value::Boolean(left > right)),
            (Value::Float(left), GreaterOrEqual, Value::Float(right)) => {
                Ok(Value::Boolean(left >= right))
            }
            (Value::Boolean(left), And, Value::Boolean(right)) => Ok(Value::Boolean(left && right)),
            (Value::Boolean(left), Or, Value::Boolean(right)) => Ok(Value::Boolean(left || right)),
            (left, operator, right) => Err(self.runtime_error(format!(
                "cannot apply {operator:?} to {left:?} and {right:?}"
            ))),
        }
    }

    fn runtime_error(&self, message: String) -> Error {
        Error::new(ErrorKind::InvalidData, message)
    }
}
