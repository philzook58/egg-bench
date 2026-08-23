use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "benchtool", about = "Utilities for egg-bench benchmarks")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Convert between benchmark formats.
    Convert {
        /// Input file, or '-' for stdin. Reads stdin when omitted.
        input: Option<PathBuf>,

        /// Write to this file instead of stdout.
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Input syntax.
        #[arg(long, value_enum, default_value_t = benchtool::Format::Egglog)]
        from: benchtool::Format,

        /// Output syntax.
        #[arg(long, value_enum, default_value_t = benchtool::Format::Cnf)]
        to: benchtool::Format,

        /// Collapse all SMT-LIB sorts into the egglog `Term` sort.
        #[arg(long)]
        untyped: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::Convert {
            input,
            output,
            from,
            to,
            untyped,
        } => {
            let (source, filename, input_name) = read_input(input.as_deref())?;
            let converted = if untyped {
                if from != benchtool::Format::Smtlib {
                    return Err("--untyped is only available with --from smtlib".into());
                }
                let egglog = benchtool::smtlib_to_egglog_untyped(&source)
                    .map_err(|error| format!("could not convert {input_name}: {error}"))?;
                if to == benchtool::Format::Egglog {
                    egglog
                } else {
                    benchtool::convert(&egglog, filename, benchtool::Format::Egglog, to)
                        .map_err(|error| format!("could not convert {input_name}: {error}"))?
                }
            } else {
                benchtool::convert(&source, filename, from, to)
                    .map_err(|error| format!("could not convert {input_name}: {error}"))?
            };

            if let Some(output) = output {
                fs::write(output, converted)?;
            } else {
                print!("{converted}");
            }
        }
    }

    Ok(())
}

fn read_input(input: Option<&Path>) -> Result<(String, Option<String>, String), io::Error> {
    match input {
        Some(path) if path.as_os_str() != "-" => Ok((
            fs::read_to_string(path)?,
            Some(path.to_string_lossy().into_owned()),
            path.display().to_string(),
        )),
        _ => {
            let mut source = String::new();
            io::stdin().read_to_string(&mut source)?;
            Ok((source, None, "stdin".to_owned()))
        }
    }
}
