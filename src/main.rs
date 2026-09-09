use clap::{Parser, Subcommand};

use crate::build::lexer::Lexer;

mod build;

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
                    eprintln!("Error: No 'main.cr' file found in the current directory or './src' directory.");
                    std::process::exit(1);
                }
            }
        }
    }
}

fn lex_and_parse_file(file_path: &str) {
    let content = std::fs::read_to_string(file_path).unwrap();
    let mut lexer = Lexer::new(&content, file_path);
    let tokens = lexer.tokenize();
}
