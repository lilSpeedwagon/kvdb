use clap;
use clap::{Parser, ValueEnum};
use simple_logger;

use kvdb::storage::{base, lsm, mem, worker};
use kvdb::types;
use kvdb::server::server;
use kvdb::threads::rayon;


#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Server hostname
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    host: String,
    /// Server port
    #[arg(short = 'P', long, default_value = "4000")]
    port: u32,
    /// Storage path
    #[arg(short, long, default_value = "./")]
    path: String,
    /// Set log level
    #[arg(short, long, default_value = "info")]
    log_level: LogLevel,
}

#[derive(Clone, ValueEnum)]
enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

fn run_storage_process(storage_processor: worker::StorageProcessor) -> std::thread::JoinHandle<()>{
    std::thread::spawn(move || {
        storage_processor.run_in_loop();
    })
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

    let (sender, receiver) = std::sync::mpsc::channel();

    // TODO: make storage type configureable
    let storage = Box::new(mem::MemStorage::new());
    let storage_processor = worker::StorageProcessor::new(receiver, storage);
    let storage_thread = run_storage_process(storage_processor);

    log::info!("Starting server at {}:{} with at {}", cli.host, cli.port, cli.path);
    const THREAD_POOL_SIZE: usize = 4; // TODO: move to config
    let thread_pool = Box::new(rayon::RayonThreadPool::new(THREAD_POOL_SIZE)?);
    let mut server = server::Server::new(thread_pool, sender);
    server.listen(cli.host, cli.port)?;

    // TODO: graceful termination
    storage_thread.join()?;

    return Ok(());
}
