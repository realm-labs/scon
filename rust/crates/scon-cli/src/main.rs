use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use scon::{Diagnostic, FormatOptions, LoadOptions, Number, Value};

#[derive(Debug, Parser)]
#[command(name = "scon", version, about = "SCON configuration language tooling")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Parse and resolve SCON files, reporting diagnostics.
    Check { files: Vec<PathBuf> },
    /// Format SCON source while preserving comments and composition syntax.
    Fmt {
        files: Vec<PathBuf>,
        #[arg(long)]
        write: bool,
        #[arg(long)]
        check: bool,
    },
    /// Print the resolved canonical SCON document.
    Print { file: PathBuf },
    /// Print the resolved document as JSON.
    ToJson {
        file: PathBuf,
        #[arg(long)]
        compact: bool,
    },
    /// Print a resolved value at a dotted path.
    Get { file: PathBuf, path: String },
    /// Print CLI and core library versions.
    Version,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(AppExit::Diagnostics) => ExitCode::from(1),
        Err(AppExit::Usage(message)) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
        Err(AppExit::Io(message)) => {
            eprintln!("{message}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> Result<(), AppExit> {
    match cli.command {
        Command::Check { files } => check(files),
        Command::Fmt {
            files,
            write,
            check,
        } => fmt(files, write, check),
        Command::Print { file } => {
            let value = scon::parse_file(file).map_err(|err| {
                print_diagnostic(&scon::diagnostic_from_error(&err, ""));
                AppExit::Diagnostics
            })?;
            print!(
                "{}",
                scon::to_string_value(&value).map_err(|err| {
                    print_diagnostic(&scon::diagnostic_from_error(&err, ""));
                    AppExit::Diagnostics
                })?
            );
            Ok(())
        }
        Command::ToJson { file, compact } => {
            let value = scon::parse_file(file).map_err(|err| {
                print_diagnostic(&scon::diagnostic_from_error(&err, ""));
                AppExit::Diagnostics
            })?;
            let json = value_to_json(&value);
            if compact {
                println!(
                    "{}",
                    serde_json::to_string(&json).expect("JSON serialization failed")
                );
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json).expect("JSON serialization failed")
                );
            }
            Ok(())
        }
        Command::Get { file, path } => {
            let value = scon::parse_file(file).map_err(|err| {
                print_diagnostic(&scon::diagnostic_from_error(&err, ""));
                AppExit::Diagnostics
            })?;
            let value = scon::get_path(&value, &path).map_err(|err| {
                print_diagnostic(&scon::diagnostic_from_error(&err, ""));
                AppExit::Diagnostics
            })?;
            print!("{}", scon::to_string_fragment(value));
            Ok(())
        }
        Command::Version => {
            println!("scon-cli {}", env!("CARGO_PKG_VERSION"));
            println!("scon-core {}", scon::VERSION);
            Ok(())
        }
    }
}

fn check(files: Vec<PathBuf>) -> Result<(), AppExit> {
    if files.is_empty() {
        return Err(AppExit::Usage(
            "scon check requires at least one file".into(),
        ));
    }
    let mut failed = false;
    for file in files {
        let analysis = scon::analyze_file(&file, LoadOptions::default());
        for diagnostic in analysis.diagnostics {
            print_diagnostic(&diagnostic);
            failed = true;
        }
    }
    if failed {
        Err(AppExit::Diagnostics)
    } else {
        Ok(())
    }
}

fn fmt(files: Vec<PathBuf>, write: bool, check: bool) -> Result<(), AppExit> {
    if files.is_empty() {
        return Err(AppExit::Usage("scon fmt requires at least one file".into()));
    }
    if write && check {
        return Err(AppExit::Usage(
            "scon fmt cannot use --write and --check together".into(),
        ));
    }
    if !write && !check && files.len() > 1 {
        return Err(AppExit::Usage(
            "scon fmt without --write or --check accepts exactly one file".into(),
        ));
    }

    let mut changed = false;
    for file in files {
        let source = read_file(&file)?;
        let formatted = scon::format_source(&source, FormatOptions::default()).map_err(|err| {
            print_diagnostic(&scon::diagnostic_from_error(&err, &source));
            AppExit::Diagnostics
        })?;
        if check {
            if source != formatted {
                eprintln!("{} needs formatting", file.display());
                changed = true;
            }
        } else if write {
            if source != formatted {
                fs::write(&file, formatted)
                    .map_err(|err| AppExit::Io(format!("{}: {err}", file.display())))?;
            }
        } else {
            print!("{formatted}");
        }
    }

    if changed {
        Err(AppExit::Diagnostics)
    } else {
        Ok(())
    }
}

fn read_file(path: &Path) -> Result<String, AppExit> {
    fs::read_to_string(path).map_err(|err| AppExit::Io(format!("{}: {err}", path.display())))
}

fn print_diagnostic(diagnostic: &Diagnostic) {
    let file = diagnostic
        .file
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<input>".to_string());
    if let Some(range) = &diagnostic.range {
        eprintln!(
            "{file}:{}:{}: {:?}: {}",
            range.start.line + 1,
            range.start.character + 1,
            diagnostic.code,
            diagnostic.message
        );
    } else {
        eprintln!("{file}: {:?}: {}", diagnostic.code, diagnostic.message);
    }
}

fn value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Null => serde_json::Value::Null,
        Value::Bool(value) => serde_json::Value::Bool(*value),
        Value::Number(value) => match value {
            Number::I64(value) => serde_json::Value::Number((*value).into()),
            Number::U64(value) => serde_json::Value::Number((*value).into()),
            Number::F64(value) => serde_json::Number::from_f64(*value)
                .map(serde_json::Value::Number)
                .expect("SCON f64 numbers are finite"),
        },
        Value::String(value) => serde_json::Value::String(value.clone()),
        Value::Array(values) => {
            serde_json::Value::Array(values.iter().map(value_to_json).collect())
        }
        Value::Object(object) => serde_json::Value::Object(
            object
                .iter()
                .map(|(key, value)| (key.clone(), value_to_json(value)))
                .collect(),
        ),
    }
}

enum AppExit {
    Diagnostics,
    Usage(String),
    Io(String),
}
