use clap::{Parser, Subcommand};

use crate::{
    build::{
        lexer::Lexer,
        parser::{Parser as CrustParser, Program},
    },
    exec::interpreter::Interpreter,
};

mod build;
mod exec;

#[derive(Debug, Parser)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[clap(
        about = "Build the project to a *.crust file, if no input file is specified it searches the current folder and the ./src folder for a file named 'main.cr'"
    )]
    Build {
        #[clap(short, long)]
        file: Option<String>,
    },
    #[clap(about = "Executes a *.crust file")]
    Exec { file: String, args: Vec<String> },
    #[clap(about = "Displays the AST of a *.crust file")]
    GetTree { file: String },
}

fn main() {
    let args = Args::parse();
    match args.command {
        Command::Build { file } => {
            if let Some(file) = file {
                lex_and_parse_file(&file);
            } else {
                let mut found = false;

                if std::path::Path::new("./src").exists() {
                    for entry in std::fs::read_dir("./src").unwrap() {
                        let entry = entry.unwrap();
                        let path = entry.path();
                        if path.is_file()
                            && path.file_name().map_or(false, |name| name == "main.cr")
                        {
                            found = true;
                            lex_and_parse_file(&path.to_string_lossy());
                        }
                    }
                } else {
                    for entry in std::fs::read_dir(".").unwrap() {
                        let entry = entry.unwrap();
                        let path = entry.path();
                        if path.is_file()
                            && path.file_name().map_or(false, |name| name == "main.cr")
                        {
                            found = true;
                            lex_and_parse_file(&path.to_string_lossy());
                        }
                    }
                }

                if !found {
                    eprintln!(
                        "Error: No 'main.cr' file found in the current directory or './src' directory."
                    );
                    std::process::exit(1);
                }
            }
        }
        Command::Exec { file, args } => {
            let executable_content = std::fs::read(&file).unwrap_or_else(|_| {
                eprintln!("Error: Could not read the file '{}'.", file);
                std::process::exit(1);
            });
            let program = Program::from_bytes(&executable_content).unwrap_or_else(|_| {
                eprintln!("Error: Could not parse the executable file '{}'.", file);
                std::process::exit(1);
            });
            let mut interpreter = Interpreter::new(program);
            interpreter.run(args);
        }
        Command::GetTree { file } => {
            let content = std::fs::read(&file).unwrap_or_else(|_| {
                eprintln!("error: Could not read the file '{}'.", file);
                std::process::exit(1);
            });
            let program = Program::from_bytes(&content).unwrap_or_else(|_| {
                eprintln!("error: Could not parse the executable file '{}'.", file);
                std::process::exit(1);
            });
            println!("{:#?}", program.items);
        }
    }
}

fn lex_and_parse_file(file_path: &str) {
    let content = std::fs::read_to_string(file_path).unwrap();
    let mut lexer = Lexer::new(&content, file_path);
    let tokens = lexer.tokenize();
    let parser = CrustParser::new(tokens);
    let stmt = match parser.parse() {
        Ok(stmt) => stmt,
        Err(e) => {
            eprintln!("ran into an error when parsing:\n{}", e);
            std::process::exit(1);
        }
    };
    let executable_name = file_path
        .split('/')
        .last()
        .unwrap_or("output.crust")
        .replace(".cr", ".crust");
    std::fs::write(&executable_name, stmt.to_bytes()).unwrap();
    println!("Successfully built {} to {}", file_path, executable_name);
}
