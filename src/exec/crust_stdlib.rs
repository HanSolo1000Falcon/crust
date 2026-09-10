use std::io::{self, Error, ErrorKind::InvalidData};

use rand::random_range;

use crate::exec::interpreter::{Env, Value};

pub fn eval_std_call(function_name: String, args: Vec<Value>, env: &Env) -> Result<Value, Error> {
    match function_name.as_str() {
        "println" => std_println(args, env),
        "print" => std_print(args, env),
        "getln" => std_getln(args),
        "rand" => std_rand(args),
        "read_from_file" => std_read_from_file(args),
        _ => Err(Error::new(
            InvalidData,
            format!("the function '{function_name}' is not a part of the std lib"),
        )),
    }
}

fn std_println(args: Vec<Value>, env: &Env) -> Result<Value, Error> {
    let args_len = args.len();
    if args_len != 1 {
        return Err(Error::new(
            InvalidData,
            format!("expected 1 argument on std::println() but got {args_len}"),
        ));
    }

    match args.into_iter().next() {
        Some(Value::String(to_print)) => {
            println!("{}", format_string(&to_print, env)?);
            Ok(Value::None)
        }
        _ => Err(Error::new(
            InvalidData,
            "std::println() only accepts a string",
        )),
    }
}

fn std_print(args: Vec<Value>, env: &Env) -> Result<Value, Error> {
    let args_len = args.len();
    if args_len != 1 {
        return Err(Error::new(
            InvalidData,
            format!("expected 1 argument on std::print() but got {args_len}"),
        ));
    }

    match args.into_iter().next() {
        Some(Value::String(to_print)) => {
            print!("{}", format_string(&to_print, env)?);
            Ok(Value::None)
        }
        _ => Err(Error::new(
            InvalidData,
            "std::print() only accepts a string",
        )),
    }
}

fn std_getln(args: Vec<Value>) -> Result<Value, Error> {
    let args_len = args.len();
    if args_len != 0 {
        return Err(Error::new(
            InvalidData,
            format!("expected 0 arguments on std::getln() but got {args_len}"),
        ));
    }

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    Ok(Value::String(input))
}

fn std_rand(args: Vec<Value>) -> Result<Value, Error> {
    let args_len = args.len();
    if args_len != 2 {
        return Err(Error::new(
            InvalidData,
            format!("expected 2 arguments on std::rand() but got {args_len}"),
        ));
    }

    let bounds: Result<Vec<f64>, Error> = args
        .into_iter()
        .enumerate()
        .map(|(index, value)| match value {
            Value::Int(value) => Ok(value as f64),
            Value::Float(value) => Ok(value),
            _ => Err(Error::new(
                InvalidData,
                format!("std::rand() argument {} must be a number", index + 1),
            )),
        })
        .collect();
    let bounds = bounds?;

    let min = bounds[0];
    let max = bounds[1];

    Ok(Value::Float(random_range(min..max)))
}

fn std_read_from_file(args: Vec<Value>) -> Result<Value, Error> {
    let args_len = args.len();
    if args_len != 1 {
        return Err(Error::new(
            InvalidData,
            format!("expected 1 arguments on std::read_from_file() but got {args_len}"),
        ));
    }

    match args[0].clone() {
        Value::String(file_path) => Ok(Value::String(std::fs::read_to_string(file_path)?.to_string())),
        _ => Err(Error::new(InvalidData, "std::read_from_file() expects a string as it's only argument")),
    }
}

fn format_string(input: &str, env: &Env) -> Result<String, Error> {
    let mut output = String::with_capacity(input.len());
    let mut characters = input.chars().peekable();

    while let Some(character) = characters.next() {
        match character {
            '\\' => {
                if let Some(escaped) = characters.next() {
                    output.push(escaped);
                } else {
                    output.push('\\');
                }
            }
            '{' => {
                let mut name = String::new();
                let mut closed = false;
                while let Some(&character) = characters.peek() {
                    characters.next();
                    if character == '}' {
                        closed = true;
                        break;
                    }
                    name.push(character);
                }

                if !closed {
                    return Err(Error::new(InvalidData, "unterminated format placeholder"));
                }
                if name.is_empty() {
                    return Err(Error::new(InvalidData, "empty format placeholder"));
                }

                let value = env
                    .get(&name)
                    .map(|(value, _)| value.borrow())
                    .ok_or_else(|| {
                        Error::new(InvalidData, format!("unknown format variable '{name}'"))
                    })?;
                output.push_str(&value_to_string(&value));
            }
            '}' => output.push('}'),
            _ => output.push(character),
        }
    }

    Ok(output)
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::Int(value) => value.to_string(),
        Value::Float(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Char(value) => value.to_string(),
        Value::Boolean(value) => value.to_string(),
        Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(value_to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Callee(function, _, _) => format!("<callee {function}>"),
        Value::Name(name) => name.clone(),
        Value::Object { name, .. } => format!("<object {name}>"),
        Value::Reference(value) => value_to_string(&value.borrow()),
        Value::None => "none".to_string(),
    }
}
