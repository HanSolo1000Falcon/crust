use std::{
    cell::RefCell,
    collections::HashMap,
    io::{Error, ErrorKind},
    rc::Rc,
};

use crate::{
    build::parser::{AccessOp, BinaryOp, Expr, Function, Item, Object, Program, Stmt, UnaryOp},
    exec::types::{array, string},
    exec::{crust_envlib::eval_env_call, crust_stdlib::eval_std_call},
};

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded_libraries.rs"));
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    String(String),
    Char(char),
    Boolean(bool),
    Array(Vec<Value>),
    Object {
        name: String,
        fields: HashMap<String, Value>,
    },
    Reference(Rc<RefCell<Value>>),
    Callee(String, Box<Value>, AccessOp),
    Name(String),
    None,
}

pub type Env = HashMap<String, (Rc<RefCell<Value>>, bool)>;

pub enum Flow {
    Normal,
    Return(Value),
    Break,
    Continue,
}

struct ObjectCallContext {
    object_name: String,
    constructor: bool,
}

pub struct Interpreter {
    program: Program,
    program_args: Option<Vec<String>>,
    object_call_stack: Vec<ObjectCallContext>,
}

impl Interpreter {
    pub fn new(program: Program) -> Self {
        let mut program = program;
        let stdlib = Program::from_bytes(embedded::STDLIB_BYTES)
            .expect("embedded stdlib has an invalid program format");
        let envlib = Program::from_bytes(embedded::ENVLIB_BYTES)
            .expect("embedded envlib has an invalid program format");
        program.items.extend(stdlib.items);
        program.items.extend(envlib.items);

        Interpreter {
            program,
            program_args: None,
            object_call_stack: Vec::new(),
        }
    }

    pub fn run(&mut self, args: Vec<String>) {
        let entry = self.program.items.iter().find_map(|item| match item {
            Item::Entry(body) => Some(body.clone()),
            _ => None,
        });

        self.program_args = Some(args);

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
            Stmt::Let {
                mutable,
                name,
                value,
            } => {
                let value = self.eval_expr(value, env)?;
                env.insert(name.clone(), (Rc::new(RefCell::new(value)), *mutable));
                Ok(Flow::Normal)
            }
            Stmt::Assign { target, value } => {
                let value = self.eval_expr(value, env)?;
                self.assign_value(target, value, env)?;
                Ok(Flow::Normal)
            }
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
            Stmt::Block(block) => {
                let mut block_env = env.clone();
                self.exec_block(block, &mut block_env)
            }
            Stmt::If {
                condition,
                body,
                else_body,
            } => {
                let condition = self.eval_expr(condition, env)?;
                let Value::Boolean(condition) = condition else {
                    return Err(self.runtime_error("if condition must be a boolean".to_string()));
                };

                let selected_body = if condition {
                    Some(body)
                } else {
                    else_body.as_ref()
                };
                if let Some(selected_body) = selected_body {
                    let mut branch_env = env.clone();
                    self.exec_block(selected_body, &mut branch_env)
                } else {
                    Ok(Flow::Normal)
                }
            }
            Stmt::While { condition, body } => {
                loop {
                    let condition = self.eval_expr(condition, env)?;
                    let Value::Boolean(condition) = condition else {
                        return Err(
                            self.runtime_error("while condition must be a boolean".to_string())
                        );
                    };
                    if !condition {
                        break;
                    }

                    let mut iteration_env = env.clone();
                    match self.exec_block(body, &mut iteration_env)? {
                        Flow::Normal | Flow::Continue => continue,
                        Flow::Break => break,
                        Flow::Return(value) => return Ok(Flow::Return(value)),
                    }
                }
                Ok(Flow::Normal)
            }
            Stmt::Break => Ok(Flow::Break),
            Stmt::Continue => Ok(Flow::Continue),
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
                .map(|(value, _)| value.borrow().clone())
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
            Expr::Call { callee, arguments } => match callee.as_ref() {
                Expr::Name(function_name) => self.eval_function_call(function_name, arguments, env),
                _ => match self.eval_expr(callee, env)? {
                    Value::Callee(function_name, calling_on, access_op) => match access_op {
                        AccessOp::Namespace => match calling_on.as_ref() {
                            Value::String(namespace_name) => match namespace_name.as_str() {
                                "std" if self.has_namespace_function("std", &function_name) => self
                                    .eval_namespace_function_call(
                                        namespace_name,
                                        &function_name,
                                        arguments,
                                        env,
                                    ),
                                "env" if self.has_namespace_function("env", &function_name) => self
                                    .eval_namespace_function_call(
                                        namespace_name,
                                        &function_name,
                                        arguments,
                                        env,
                                    ),
                                "std" => eval_std_call(
                                    function_name,
                                    self.eval_values(arguments, env)?,
                                    env,
                                ),
                                "env" => eval_env_call(
                                    function_name,
                                    self.eval_values(arguments, env)?,
                                    env,
                                    self.program_args.clone().unwrap(),
                                ),
                                _ => self.eval_namespace_function_call(
                                    namespace_name,
                                    &function_name,
                                    arguments,
                                    env,
                                ),
                            },
                            _ => Err(self.runtime_error("shouldn't happen!!!".to_string())),
                        },
                        AccessOp::Member => {
                            self.eval_member_call(*calling_on, &function_name, arguments, env)
                        }
                    },
                    caller => Err(self.runtime_error(format!("invalid callee value: {caller:?}"))),
                },
            },
            Expr::Access {
                operator: AccessOp::Namespace,
                value,
                member,
            } => {
                let namespace = Value::String(self.namespace_path(value, env)?);
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
                    Expr::Name(name) => {
                        let Some((value, _)) = env.get(name) else {
                            return Err(self.runtime_error(format!("unknown variable '{name}'")));
                        };
                        Value::Reference(value.clone())
                    }
                    _ => self.eval_expr(value, env)?,
                };
                let receiver_value = match &receiver {
                    Value::Reference(value) => value.borrow().clone(),
                    value => value.clone(),
                };
                if let Value::Object { fields, .. } = &receiver_value {
                    if let Some(field) = fields.get(member) {
                        let object_name = match &receiver_value {
                            Value::Object { name, .. } => name,
                            _ => unreachable!(),
                        };
                        let (_, private) =
                            self.object_field(object_name, member).ok_or_else(|| {
                                self.runtime_error(format!(
                                    "unknown field '{object_name}:{member}'"
                                ))
                            })?;
                        if private && !self.can_access_private(object_name) {
                            return Err(self.runtime_error(format!(
                                "field '{object_name}:{member}' is private"
                            )));
                        }
                        return Ok(field.clone());
                    }
                }
                Ok(Value::Callee(
                    member.clone(),
                    Box::new(receiver),
                    AccessOp::Member,
                ))
            }
            Expr::Group(value) => self.eval_expr(value, env),
        }
    }

    fn eval_function_call(
        &mut self,
        function_name: &str,
        arguments: &[Expr],
        env: &mut Env,
    ) -> Result<Value, Error> {
        let function = self
            .program
            .items
            .iter()
            .find_map(|item| match item {
                Item::Function(function) if function.name == function_name => {
                    Some(function.clone())
                }
                _ => None,
            })
            .ok_or_else(|| self.runtime_error(format!("unknown function '{function_name}'")))?;

        let args = self.eval_arguments(arguments, &function.parameters, env)?;
        self.execute_function(&function, args)
    }

    fn eval_namespace_function_call(
        &mut self,
        namespace_name: &str,
        function_name: &str,
        arguments: &[Expr],
        env: &mut Env,
    ) -> Result<Value, Error> {
        if namespace_name == "array" && function_name == "construct" {
            return Ok(Value::Array(self.eval_values(arguments, env)?));
        }

        let namespace_parts: Vec<&str> = namespace_name.split("::").collect();
        let item = Self::find_namespace_item(&self.program.items, &namespace_parts, function_name)
            .ok_or_else(|| {
                self.runtime_error(format!(
                    "unknown function '{namespace_name}::{function_name}'"
                ))
            })?;

        match item {
            Item::Function(function) => {
                let args = self.eval_arguments(arguments, &function.parameters, env)?;
                self.execute_function(&function, args)
            }
            Item::Object(object) if function_name == "construct" => {
                let parameters = object
                    .constructor
                    .as_ref()
                    .map(|constructor| constructor.parameters.as_slice())
                    .unwrap_or(&[]);
                let args = self.eval_arguments(arguments, parameters, env)?;
                self.construct_object(&object, args)
            }
            _ => Err(self.runtime_error(format!(
                "'{namespace_name}::{function_name}' is not callable"
            ))),
        }
    }

    fn eval_member_call(
        &mut self,
        receiver: Value,
        function_name: &str,
        arguments: &[Expr],
        env: &mut Env,
    ) -> Result<Value, Error> {
        let receiver_value = match &receiver {
            Value::Reference(value) => value.borrow().clone(),
            value => value.clone(),
        };
        if matches!(receiver_value, Value::Array(_) | Value::String(_)) {
            let args = self.eval_values(arguments, env)?;
            return match receiver {
                Value::Reference(value) => {
                    let mut value = value.borrow_mut();
                    if matches!(&*value, Value::Array(_)) {
                        array::eval_method(&mut value, function_name, args)
                    } else {
                        string::eval_method(&value, function_name, args)
                    }
                }
                value => {
                    if matches!(value, Value::Array(_)) {
                        array::eval_method(&mut value.clone(), function_name, args)
                    } else {
                        string::eval_method(&value, function_name, args)
                    }
                }
            };
        }
        let Value::Object { name, .. } = &receiver_value else {
            return Err(self.runtime_error(format!(
                "cannot call member '{function_name}' on {receiver_value:?}"
            )));
        };
        let function = self
            .program
            .items
            .iter()
            .find_map(|item| match item {
                Item::Object(object) if object.name == *name => object
                    .methods
                    .iter()
                    .find(|method| method.name == function_name)
                    .cloned(),
                _ => None,
            })
            .ok_or_else(|| {
                self.runtime_error(format!("unknown member '{name}:{function_name}'"))
            })?;
        if function.private && !self.can_access_private(name) {
            return Err(self.runtime_error(format!("method '{name}:{function_name}' is private")));
        }

        let args = self.eval_arguments(arguments, &function.parameters, env)?;
        self.execute_method(&function, receiver_value, args)
    }

    fn eval_values(&mut self, arguments: &[Expr], env: &mut Env) -> Result<Vec<Value>, Error> {
        arguments
            .iter()
            .map(|argument| self.eval_expr(argument, env))
            .collect()
    }

    fn namespace_path(&mut self, expr: &Expr, env: &mut Env) -> Result<String, Error> {
        match expr {
            Expr::Name(name) => Ok(name.clone()),
            Expr::Access {
                value,
                operator: AccessOp::Namespace,
                member,
            } => Ok(format!("{}::{member}", self.namespace_path(value, env)?)),
            _ => Err(self.runtime_error("namespace access requires identifiers".to_string())),
        }
    }

    fn find_namespace_item(
        items: &[Item],
        namespace_parts: &[&str],
        function_name: &str,
    ) -> Option<Item> {
        let (first, rest) = namespace_parts.split_first()?;
        let namespace = items.iter().find_map(|item| match item {
            Item::Namespace(namespace) if namespace.name == *first => Some(namespace),
            _ => None,
        });
        if let Some(namespace) = namespace {
            if rest.is_empty() {
                return namespace.items.iter().find_map(|item| match item {
                    Item::Function(function) if function.name == function_name => {
                        Some(Item::Function(function.clone()))
                    }
                    _ => None,
                });
            }
            return Self::find_namespace_item(&namespace.items, rest, function_name);
        }

        if namespace_parts.len() == 1 && function_name == "construct" {
            return items.iter().find_map(|item| match item {
                Item::Object(object) if object.name == *first => Some(Item::Object(object.clone())),
                _ => None,
            });
        }
        None
    }

    fn has_namespace_function(&self, namespace_name: &str, function_name: &str) -> bool {
        let namespace_parts: Vec<&str> = namespace_name.split("::").collect();
        Self::find_namespace_item(&self.program.items, &namespace_parts, function_name).is_some()
    }

    fn eval_arguments(
        &mut self,
        arguments: &[Expr],
        parameters: &[crate::build::parser::Parameter],
        env: &mut Env,
    ) -> Result<Vec<Value>, Error> {
        if parameters.len() != arguments.len() {
            return Err(self.runtime_error(format!(
                "expected {} arguments but got {}",
                parameters.len(),
                arguments.len()
            )));
        }

        parameters
            .iter()
            .zip(arguments)
            .map(|(parameter, argument)| {
                if parameter.reference {
                    let Expr::Name(name) = argument else {
                        return Err(self.runtime_error(format!(
                            "reference parameter '{}' requires a variable",
                            parameter.name
                        )));
                    };
                    let Some((value, _)) = env.get(name) else {
                        return Err(self.runtime_error(format!("unknown variable '{name}'")));
                    };
                    Ok(Value::Reference(value.clone()))
                } else {
                    self.eval_expr(argument, env)
                }
            })
            .collect()
    }

    fn construct_object(&mut self, object: &Object, args: Vec<Value>) -> Result<Value, Error> {
        let fields = object
            .fields
            .iter()
            .map(|field| (field.name.clone(), Value::None))
            .collect();
        let instance = Value::Object {
            name: object.name.clone(),
            fields,
        };
        let Some(constructor) = &object.constructor else {
            if args.is_empty() {
                return Ok(instance);
            }
            return Err(self.runtime_error(format!(
                "object '{}' has no constructor but got {} arguments",
                object.name,
                args.len()
            )));
        };

        self.execute_method(constructor, instance, args)
    }

    fn execute_method(
        &mut self,
        function: &Function,
        receiver: Value,
        args: Vec<Value>,
    ) -> Result<Value, Error> {
        if function.parameters.len() != args.len() {
            return Err(self.runtime_error(format!(
                "function '{}' expected {} arguments but got {}",
                function.name,
                function.parameters.len(),
                args.len()
            )));
        }

        let object_name = match &receiver {
            Value::Object { name, .. } => name.clone(),
            _ => {
                return Err(
                    self.runtime_error("methods can only be called on object values".to_string())
                );
            }
        };
        let mut env = HashMap::new();
        env.insert("this".to_string(), (Rc::new(RefCell::new(receiver)), false));
        for (parameter, value) in function.parameters.iter().zip(args) {
            let value = match value {
                Value::Reference(value) if parameter.reference => value,
                value => Rc::new(RefCell::new(value)),
            };
            env.insert(parameter.name.clone(), (value, parameter.mutable));
        }

        self.object_call_stack.push(ObjectCallContext {
            object_name,
            constructor: function.name == "constructor",
        });
        let flow = self.exec_block(&function.body, &mut env);
        self.object_call_stack.pop();

        match flow? {
            Flow::Return(value) => Ok(value),
            Flow::Normal if function.name == "constructor" => {
                Ok(env.remove("this").unwrap().0.borrow().clone())
            }
            Flow::Normal => Ok(Value::None),
            Flow::Break | Flow::Continue => {
                Err(self.runtime_error("break or continue cannot escape a function".to_string()))
            }
        }
    }

    fn assign_value(&self, target: &Expr, value: Value, env: &mut Env) -> Result<(), Error> {
        match target {
            Expr::Name(name) => {
                let Some((target, mutable)) = env.get(name) else {
                    return Err(self.runtime_error(format!("unknown variable '{name}'")));
                };
                if !*mutable {
                    return Err(self.runtime_error(format!("cannot assign to immutable '{name}'")));
                }
                *target.borrow_mut() = value;
                Ok(())
            }
            Expr::Access {
                operator: AccessOp::Member,
                value: receiver,
                member,
            } => {
                let Expr::Name(name) = receiver.as_ref() else {
                    return Err(
                        self.runtime_error("member assignment requires a variable".to_string())
                    );
                };
                let Some((target, _)) = env.get(name) else {
                    return Err(self.runtime_error(format!("'{name}' is not an object")));
                };
                let mut target = target.borrow_mut();
                let Value::Object {
                    name: object_name,
                    fields,
                } = &mut *target
                else {
                    return Err(self.runtime_error(format!("'{name}' is not an object")));
                };
                let object_name = object_name.clone();
                let Some((field_mutable, field_private)) = self.object_field(&object_name, member)
                else {
                    return Err(
                        self.runtime_error(format!("unknown field '{object_name}:{member}'"))
                    );
                };
                if field_private && !self.can_access_private(&object_name) {
                    return Err(
                        self.runtime_error(format!("field '{object_name}:{member}' is private"))
                    );
                }
                let initializing = self.object_call_stack.last().is_some_and(|context| {
                    context.constructor && context.object_name == object_name
                });
                if !field_mutable && !initializing {
                    return Err(self.runtime_error(format!(
                        "cannot assign to immutable field '{object_name}:{member}'"
                    )));
                }
                fields.insert(member.clone(), value);
                Ok(())
            }
            _ => Err(self.runtime_error("invalid assignment target".to_string())),
        }
    }

    fn execute_function(&mut self, function: &Function, args: Vec<Value>) -> Result<Value, Error> {
        if function.parameters.len() != args.len() {
            return Err(self.runtime_error(format!(
                "function '{}' expected {} arguments but got {}",
                function.name,
                function.parameters.len(),
                args.len()
            )));
        }

        let mut env = HashMap::new();
        for (parameter, value) in function.parameters.iter().zip(args) {
            let value = match value {
                Value::Reference(value) if parameter.reference => value,
                value => Rc::new(RefCell::new(value)),
            };
            env.insert(parameter.name.clone(), (value, parameter.mutable));
        }

        match self.exec_block(&function.body, &mut env)? {
            Flow::Return(value) => Ok(value),
            Flow::Normal => Ok(Value::None),
            Flow::Break | Flow::Continue => {
                Err(self.runtime_error("break or continue cannot escape a function".to_string()))
            }
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
            (Value::Int(left), Add, Value::Float(right)) => Ok(Value::Float(left as f64 + right)),
            (Value::Float(left), Add, Value::Int(right)) => Ok(Value::Float(left + right as f64)),
            (Value::Int(left), Subtract, Value::Float(right)) => {
                Ok(Value::Float(left as f64 - right))
            }
            (Value::Float(left), Subtract, Value::Int(right)) => {
                Ok(Value::Float(left - right as f64))
            }
            (Value::Int(left), Multiply, Value::Float(right)) => {
                Ok(Value::Float(left as f64 * right))
            }
            (Value::Float(left), Multiply, Value::Int(right)) => {
                Ok(Value::Float(left * right as f64))
            }
            (Value::Int(left), Divide, Value::Float(right)) if right != 0.0 => {
                Ok(Value::Float(left as f64 / right))
            }
            (Value::Float(left), Divide, Value::Int(right)) if right != 0 => {
                Ok(Value::Float(left / right as f64))
            }
            (Value::Int(left), Remainder, Value::Float(right)) if right != 0.0 => {
                Ok(Value::Float(left as f64 % right))
            }
            (Value::Float(left), Remainder, Value::Int(right)) if right != 0 => {
                Ok(Value::Float(left % right as f64))
            }
            (Value::Int(left), Exponent, Value::Float(right)) => {
                Ok(Value::Float((left as f64).powf(right)))
            }
            (Value::Float(left), Exponent, Value::Int(right)) => {
                Ok(Value::Float(left.powf(right as f64)))
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
            (Value::Int(left), Equal, Value::Float(right)) => {
                Ok(Value::Boolean(left as f64 == right))
            }
            (Value::Float(left), Equal, Value::Int(right)) => {
                Ok(Value::Boolean(left == right as f64))
            }
            (Value::Int(left), NotEqual, Value::Float(right)) => {
                Ok(Value::Boolean(left as f64 != right))
            }
            (Value::Float(left), NotEqual, Value::Int(right)) => {
                Ok(Value::Boolean(left != right as f64))
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
            (Value::Int(left), Less, Value::Float(right)) => {
                Ok(Value::Boolean((left as f64) < right))
            }
            (Value::Float(left), Less, Value::Int(right)) => {
                Ok(Value::Boolean(left < right as f64))
            }
            (Value::Int(left), LessOrEqual, Value::Float(right)) => {
                Ok(Value::Boolean((left as f64) <= right))
            }
            (Value::Float(left), LessOrEqual, Value::Int(right)) => {
                Ok(Value::Boolean(left <= right as f64))
            }
            (Value::Int(left), Greater, Value::Float(right)) => {
                Ok(Value::Boolean((left as f64) > right))
            }
            (Value::Float(left), Greater, Value::Int(right)) => {
                Ok(Value::Boolean(left > right as f64))
            }
            (Value::Int(left), GreaterOrEqual, Value::Float(right)) => {
                Ok(Value::Boolean((left as f64) >= right))
            }
            (Value::Float(left), GreaterOrEqual, Value::Int(right)) => {
                Ok(Value::Boolean(left >= right as f64))
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

    fn can_access_private(&self, object_name: &str) -> bool {
        self.object_call_stack
            .last()
            .is_some_and(|context| context.object_name == object_name)
    }

    fn object_field(&self, object_name: &str, field_name: &str) -> Option<(bool, bool)> {
        self.program.items.iter().find_map(|item| match item {
            Item::Object(object) if object.name == object_name => object
                .fields
                .iter()
                .find(|field| field.name == field_name)
                .map(|field| (field.mutable, field.private)),
            _ => None,
        })
    }
}
