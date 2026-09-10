use std::io::{Error, ErrorKind::InvalidData};

use crate::exec::interpreter::Value;

pub fn eval_method(receiver: &mut Value, method: &str, args: Vec<Value>) -> Result<Value, Error> {
    let Value::Array(values) = receiver else {
        return Err(Error::new(
            InvalidData,
            "array method called on a non-array value",
        ));
    };

    match method {
        "at" => {
            expect_args(method, &args, 1)?;
            let index = array_index(&args[0], method)?;
            values.get(index).cloned().ok_or_else(|| {
                Error::new(
                    InvalidData,
                    format!("array index {index} is out of bounds for std::array::{method}()"),
                )
            })
        }
        "push" => {
            expect_args(method, &args, 1)?;
            values.push(args.into_iter().next().unwrap());
            Ok(Value::None)
        }
        "set" => {
            expect_args(method, &args, 2)?;
            let index = array_index(&args[0], method)?;
            if index >= values.len() {
                return Err(Error::new(
                    InvalidData,
                    format!("array index {index} is out of bounds for std::array::{method}()"),
                ));
            }
            values[index] = args[1].clone();
            Ok(Value::None)
        }
        "len" => {
            expect_args(method, &args, 0)?;
            Ok(Value::Int(values.len() as i64))
        }
        "rm" => {
            expect_args(method, &args, 1)?;
            let index = array_index(&args[0], method)?;
            if index >= values.len() {
                return Err(Error::new(
                    InvalidData,
                    format!("array index {index} is out of bounds for std::array::{method}()"),
                ));
            }
            Ok(values.remove(index))
        }
        _ => Err(Error::new(
            InvalidData,
            format!("unknown array method '{method}'"),
        )),
    }
}

fn expect_args(method: &str, args: &[Value], expected: usize) -> Result<(), Error> {
    if args.len() != expected {
        return Err(Error::new(
            InvalidData,
            format!(
                "array::{method}() expected {expected} arguments but got {}",
                args.len()
            ),
        ));
    }
    Ok(())
}

fn array_index(value: &Value, method: &str) -> Result<usize, Error> {
    match value {
        Value::Int(index) if *index >= 0 => Ok(*index as usize),
        Value::Int(_) => Err(Error::new(
            InvalidData,
            format!("array::{method}() index must not be negative"),
        )),
        _ => Err(Error::new(
            InvalidData,
            format!("array::{method}() index must be an integer"),
        )),
    }
}
