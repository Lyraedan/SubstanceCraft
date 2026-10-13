//! Headless SubstanceCraft CLI.

use std::process::ExitCode;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use substancecraft_engine::session::{lock, shared_session};

#[derive(Parser)]
#[command(name = "substancecraft-cli", about = "Headless SubstanceCraft tools", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List every command id, one per line.
    Commands,
    /// Run a command by id against a fresh session.
    Run {
        /// The command id.
        id: String,
        /// The parameters as a JSON value (default `null`).
        #[arg(long, default_value = "null")]
        params: String,
    },
    /// Serve the JSON control channel.
    Serve {
        /// The address to bind.
        #[arg(long, default_value = "127.0.0.1:8765")]
        addr: String,
    },
    /// Print version information.
    Info,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let registry = Arc::new(substancecraft_engine::registry());
    let session = shared_session("cli");

    match cli.command {
        Command::Commands => {
            for id in registry.ids() {
                println!("{id}");
            }
            ExitCode::SUCCESS
        }
        Command::Run { id, params } => {
            let params: serde_json::Value = match serde_json::from_str(&params) {
                Ok(value) => value,
                Err(err) => {
                    eprintln!("invalid --params JSON: {err}");
                    return ExitCode::from(2);
                }
            };
            let mut session = lock(&session);
            match registry.run(&id, &mut session, &params) {
                Ok(outcome) => {
                    if let Some(message) = outcome.message {
                        println!("{message}");
                    }
                    if let Some(data) = outcome.data {
                        println!("{}", serde_json::to_string_pretty(&data).unwrap_or_default());
                    }
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("error: {err}");
                    ExitCode::from(1)
                }
            }
        }
        Command::Serve { addr } => match substancecraft_automation::bind(&addr) {
            Ok(listener) => {
                eprintln!("control channel listening on {addr}");
                match substancecraft_automation::serve(listener, registry, session) {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(err) => {
                        eprintln!("error: {err}");
                        ExitCode::FAILURE
                    }
                }
            }
            Err(err) => {
                eprintln!("could not bind {addr}: {err}");
                ExitCode::FAILURE
            }
        },
        Command::Info => {
            println!("SubstanceCraft {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
    }
}
