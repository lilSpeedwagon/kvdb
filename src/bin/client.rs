use clap;
use clap::{Parser, ValueEnum};
use simple_logger;

use kvdb::client::client;
use kvdb::types;


#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Command to execute
    #[command(subcommand)]
    command: Command,
    /// Server hostname
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    host: String,
    /// Server port
    #[arg(short = 'P', long, default_value = "4000")]
    port: u32,
    /// Set log level
    #[arg(short, long, default_value = "info")]
    log_level: LogLevel,
    /// Read timeout in seconds
    #[arg(short, long, default_value = "60")]
    timeout: u32,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Set value `value` for the `key`
    Set {
        /// Key
        key: String,
        /// Value
        value: String,
    },
    /// Get the value with the given `key`
    Get {
        /// Key to get
        key: String,
    },
    /// Remove the value with the given `key` if found
    Remove {
        /// Key to remove
        key: String,
    }
}

#[derive(Clone, ValueEnum)]
enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

fn main() -> Result<(), Box::<dyn std::error::Error>> {
    let cli = Cli::parse();

    let log_level = match cli.log_level {
        LogLevel::Debug => log::LevelFilter::Debug,
        LogLevel::Info => log::LevelFilter::Info,
        LogLevel::Warning => log::LevelFilter::Warn,
        LogLevel::Error => log::LevelFilter::Error,
    };
    simple_logger::SimpleLogger::new().with_level(log_level).init().unwrap();

    let host = cli.host;
    let port = cli.port;
    let timeout = std::time::Duration::from_secs(cli.timeout as u64);

    log::debug!("Connecting to {}:{}...", host, port);
    let mut client = client::Client::new();
    client.connect(host, port, timeout)?;
    
    let command = match cli.command {
        Command::Get { key } => types::Command::Get { key },
        Command::Set { key, value } => types::Command::Set {
            key: key,
            value: kvdb::types::Value::String { value: value },
        },
        Command::Remove { key } => types::Command::Remove { key },
    };
    log::debug!("Executing {}...", command);
    let result = client.execute(command)?;

    match result {
        types::CommandResult::Get { value } => {
            match value {
                Option::Some(val) => {
                    log::info!("{}", val);
                },
                Option::None => {
                    log::info!("None");
                }
            }
        },
        types::CommandResult::Set {  } => {
            log::info!("OK");
        },
        types::CommandResult::Remove {  } => {
            log::info!("OK");
        },
    }

    return Ok(());
}
