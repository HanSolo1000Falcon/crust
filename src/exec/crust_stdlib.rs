use std::io::{Error, ErrorKind::InvalidData};

use crate::exec::interpreter::{Env, Value};

pub fn eval_std_call(function_name: String, args: Vec<Value>, env: &Env) -> Result<Value, Error> {
    match function_name.as_str() {
        "println" => std_println(args, env),
        "print" => std_print(args, env),
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

                let value = env.get(&name).map(|(value, _)| value).ok_or_else(|| {
                    Error::new(InvalidData, format!("unknown format variable '{name}'"))
                })?;
                output.push_str(&value_to_string(value));
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
        Value::None => "none".to_string(),
    }
}
