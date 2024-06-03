
use clap::{Parser, Subcommand};

pub(crate) const SHORT_VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));

#[derive(Parser, Debug)]
#[command(author, version = SHORT_VERSION, long_version = SHORT_VERSION, about = "tsumori-bridge", long_about = None)]
struct Cli {
    /// The command to run
    #[clap(subcommand)]
    command: Commands,
}

/// Commands to be executed
#[derive(Debug, Subcommand)]
enum Commands {
    /// Run http server.
    #[command(name = "server")]
    Server(ServerCommand),
}

#[derive(Debug, Parser)]
struct ServerCommand {
    /// The port to listen on
    #[clap(short, long, value_name = "PORT", default_value = "8080")]
    port: u16,

    /// The request timeout in seconds
    #[clap(short, long, value_name = "TIMEOUT", default_value = "10")]
    req_timeout: u8,

    /// The port to listen on for metrics
    #[clap(short, long, value_name = "METRICS_PORT", default_value = "9090")]
    metrics_port: u16,
}

impl ServerCommand {
    fn execute(&self) -> Result<(), String> {
        Ok(())
    }
}

fn main() {
    let opt = Cli::parse();
    if let Err(err) = match opt.command {
        Commands::Server(command) => command.execute(),
    } {
        eprintln!("Error: {err:?}");
        std::process::exit(1);
    }
}
