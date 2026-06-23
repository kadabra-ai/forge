use clap::{Parser, Subcommand};
use harpoon_diagnostics::render_diagnostics;
use harpoon_serial_json::serialize_to_json;
use std::process;

use kermlc::compile_source;

#[derive(Parser)]
#[command(name = "kermlc", about = "KerML compiler")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse, resolve, and validate a KerML file
    Check {
        /// Path to the KerML source file
        file: String,
        /// Path to stdlib directory (optional; uses built-in minimal stdlib if omitted)
        #[arg(long)]
        stdlib: Option<String>,
    },
    /// Compile a KerML file to output format
    Compile {
        /// Path to the KerML source file
        file: String,
        /// Output file path
        #[arg(short, long)]
        output: String,
        /// Output format (json)
        #[arg(short, long, default_value = "json")]
        format: String,
        /// Path to stdlib directory (optional)
        #[arg(long)]
        stdlib: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Check { file, stdlib: _ } => {
            process::exit(run_check(&file));
        }
        Command::Compile {
            file,
            output,
            format,
            stdlib: _,
        } => {
            process::exit(run_compile(&file, &output, &format));
        }
    }
}

fn read_source(file_path: &str) -> Option<String> {
    match std::fs::read_to_string(file_path) {
        Ok(s) => Some(s),
        Err(e) => {
            eprintln!("error: could not read `{}`: {}", file_path, e);
            None
        }
    }
}

fn run_check(file_path: &str) -> i32 {
    let Some(source) = read_source(file_path) else {
        return 1;
    };

    let compiled = compile_source(&source, file_path);

    let diagnostics = compiled.sink.diagnostics();
    if !diagnostics.is_empty() {
        render_diagnostics(&compiled.source_map, diagnostics);
    }

    if compiled.sink.has_errors() { 1 } else { 0 }
}

fn run_compile(file_path: &str, output_path: &str, format: &str) -> i32 {
    if format != "json" {
        eprintln!(
            "error: unsupported format `{}`, only `json` is supported",
            format
        );
        return 1;
    }

    let Some(source) = read_source(file_path) else {
        return 1;
    };

    let compiled = compile_source(&source, file_path);

    let diagnostics = compiled.sink.diagnostics();
    if !diagnostics.is_empty() {
        render_diagnostics(&compiled.source_map, diagnostics);
    }

    if compiled.sink.has_errors() {
        return 1;
    }

    let json = serialize_to_json(&compiled.model, &compiled.interner);

    match std::fs::write(output_path, &json) {
        Ok(()) => {
            eprintln!("wrote {}", output_path);
            0
        }
        Err(e) => {
            eprintln!("error: could not write `{}`: {}", output_path, e);
            1
        }
    }
}
