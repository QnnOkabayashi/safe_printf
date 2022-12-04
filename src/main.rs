mod error;
mod ir;
mod lex;
mod parse;
use clap::Parser;
use error::SourceErrors;
use miette::{Context, IntoDiagnostic};
use std::fmt::Display;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;

/// Validate printf cases in C programs.
#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Cli {
    /// File to validate.
    filepath: PathBuf,

    /// Path to write optimized output to.
    #[arg(long = "optimize")]
    optimize_path: Option<PathBuf>,

    /// Path to write output with type casts format arguments to.
    #[arg(long = "typecast")]
    typecast_path: Option<PathBuf>,
}

fn main() -> miette::Result<()> {
    let cli = Cli::parse();

    let source = fs::read_to_string(&cli.filepath)
        .into_diagnostic()
        .wrap_err_with(|| format!("failed reading input at {}", cli.filepath.display()))?;

    match ir::IntermediateRepresentation::parse(&source) {
        Ok(repr) => {
            if let Some(optimize_path) = cli.optimize_path {
                write(repr.display_optimize(), "optimize", optimize_path)?;
            }

            if let Some(typecast_path) = cli.typecast_path {
                write(repr.display_typecast(), "typecast", typecast_path)?;
            }

            Ok(())
        }
        Err(errors) => Err(SourceErrors::new(cli.filepath, source, errors).into()),
    }
}

fn write(repr: impl Display, kind: &str, path: PathBuf) -> miette::Result<()> {
    let file = File::options()
        .create_new(true)
        .write(true)
        .open(&path)
        .into_diagnostic()
        .wrap_err_with(|| format!("Failed creating output for --{kind}: {}", path.display()))?;

    let mut writer = BufWriter::new(file);

    writeln!(&mut writer, "{}", repr)
        .into_diagnostic()
        .wrap_err_with(|| format!("Failed writing to file for --{kind}"))?;

    writer
        .flush()
        .into_diagnostic()
        .wrap_err_with(|| format!("Failed flushing buffered writer for --{kind}"))?;

    Ok(())
}
