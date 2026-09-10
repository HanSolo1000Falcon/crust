use std::io::{Error, ErrorKind::InvalidData};

use crate::exec::interpreter::{Env, Value};

pub fn eval_env_call(
    function_name: String,
    args: Vec<Value>,
    env: &Env,
    program_args: Vec<String>,
) -> Result<Value, Error> {
    match function_name.as_str() {
        "exit" => env_exit(args),
        "args" => env_args(args, program_args),
        _ => Err(Error::new(
            InvalidData,
            format!("the function '{function_name}' is not a part of the env lib"),
        )),
    }
}

fn env_exit(args: Vec<Value>) -> Result<Value, Error> {
    let args_len = args.len();
    if args_len != 1 {
        return Err(Error::new(
            InvalidData,
            format!("expected 1 argument on env::exit() but got {args_len}"),
        ));
    }

    match args[0] {
        Value::Int(exit_code) => {
            if exit_code != 0 {
                eprintln!("program exited with non-zero exit code {exit_code}");
            }
            std::process::exit(exit_code as i32);
        }
        _ => Err(Error::new(
            InvalidData,
            "env::exit() takes an integer as it's only argument",
        )),
    }
}

fn env_args(args: Vec<Value>, program_args: Vec<String>) -> Result<Value, Error> {
    let args_len = args.len();
    if args_len != 0 {
        return Err(Error::new(
            InvalidData,
            format!("expected 0 arguments on env::args() but got {args_len}"),
        ));
    }

    let mut args_as_values = vec![];
    for arg in program_args {
        args_as_values.push(Value::String(arg));
    }

    Ok(Value::Array(args_as_values))
}
