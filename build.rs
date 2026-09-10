#[path = "src/build/lexer.rs"]
mod lexer;
#[path = "src/build/parser.rs"]
mod parser;

use std::{env, fs, path::PathBuf};

fn compile_library(path: &str) -> Vec<u8> {
    let source = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("could not read embedded Crust library {path}: {error}");
    });
    let mut lexer = lexer::Lexer::new(&source, path);
    let parser = parser::Parser::new(lexer.tokenize());
    parser
        .parse()
        .unwrap_or_else(|error| panic!("could not parse embedded Crust library {path}: {error}"))
        .to_bytes()
}

fn bytes_literal(bytes: &[u8]) -> String {
    let values = bytes
        .iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    format!("&[{values}]")
}

fn main() {
    println!("cargo:rerun-if-changed=src/exec/stdlib.cr");
    println!("cargo:rerun-if-changed=src/exec/envlib.cr");

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is not set"));
    let generated = format!(
        "pub static STDLIB_BYTES: &[u8] = {};\npub static ENVLIB_BYTES: &[u8] = {};\n",
        bytes_literal(&compile_library("src/exec/stdlib.cr")),
        bytes_literal(&compile_library("src/exec/envlib.cr")),
    );
    fs::write(out_dir.join("embedded_libraries.rs"), generated)
        .expect("could not write embedded Crust libraries");
}
