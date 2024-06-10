use http;

use clap::{Parser, Subcommand, ValueEnum};

pub(crate) const SHORT_VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));

#[derive(Parser, Debug)]
#[command(author, version = SHORT_VERSION, long_version = SHORT_VERSION, about = "tsumori-rs", long_about = None)]
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

    /// Log level
    #[clap(short, long, value_name = "LOG_LEVEL", default_value_t = LogLevel::Info)]
    log_level: LogLevel,
}

#[derive(Debug, Copy, Clone, ValueEnum, Eq, PartialEq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl core::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Trace => write!(f, "trace"),
            Self::Debug => write!(f, "debug"),
            Self::Info => write!(f, "info"),
            Self::Warn => write!(f, "warn"),
            Self::Error => write!(f, "error"),
        }
    }
}

impl ServerCommand {
    fn execute(&self) -> Result<(), String> {
        crate::http::run_server(http::ServerConfig {
            port: self.port,
            req_timeout: self.req_timeout,
            metrics_port: self.metrics_port,
            log_level: self.log_level.to_string(),
        });
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
