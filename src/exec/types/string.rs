use std::io::{Error, ErrorKind::InvalidData};

use crate::exec::interpreter::Value;

pub fn eval_method(receiver: &Value, method: &str, args: Vec<Value>) -> Result<Value, Error> {
    let Value::String(value) = receiver else {
        return Err(Error::new(
            InvalidData,
            "string method called on a non-string value",
        ));
    };

    match method {
        "len" => {
            expect_args(method, &args, 0)?;
            Ok(Value::Int(value.chars().count() as i64))
        }
        "at" => {
            expect_args(method, &args, 1)?;
            let index = match args.first() {
                Some(Value::Int(index)) if *index >= 0 => *index as usize,
                Some(Value::Int(_)) => {
                    return Err(Error::new(
                        InvalidData,
                        "string::at() index must not be negative",
                    ));
                }
                Some(_) => {
                    return Err(Error::new(
                        InvalidData,
                        "string::at() index must be an integer",
                    ));
                }
                None => unreachable!(),
            };
            value
                .chars()
                .nth(index)
                .map(Value::Char)
                .ok_or_else(|| Error::new(InvalidData, "string::at() index is out of bounds"))
        }
        "as_char_array" => {
            expect_args(method, &args, 0)?;
            Ok(Value::Array(value.chars().map(Value::Char).collect()))
        }
        _ => Err(Error::new(
            InvalidData,
            format!("unknown string method '{method}'"),
        )),
    }
}

fn expect_args(method: &str, args: &[Value], expected: usize) -> Result<(), Error> {
    if args.len() != expected {
        return Err(Error::new(
            InvalidData,
            format!(
                "string::{method}() expected {expected} arguments but got {}",
                args.len()
            ),
        ));
    }
    Ok(())
}
