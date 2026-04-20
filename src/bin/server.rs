use std::time::Duration;

use clap;
use clap::{Parser, ValueEnum};
use simple_logger;

use kvdb::storage;
use kvdb::cmd_queue;
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

/// Run a new thread serving storage engine and waiting for commands to be executed via an MPSC channel.
fn run_storage_worker(
    cmd_receiver: std::sync::mpsc::Receiver<cmd_queue::models::QueuedCommand>,
) -> std::thread::JoinHandle<()>{
    std::thread::spawn(move || {
        // TODO: make storage type configureable
        // TODO: pass storage args here
        let storage_engine = Box::new(storage::mem::MemStorage::new());
        let mut storage_queue_worker = cmd_queue::queue::StorageCommandQueueWorker::new(
            cmd_receiver, storage_engine
        );
        storage_queue_worker.run_in_loop();
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

     // TODO: move to config
    const THREAD_POOL_SIZE: usize = 4;
    const EXEC_TIMEOUT: Duration = Duration::from_secs(60);

    // Prepare MPSC channel.
    let (sender, receiver) = std::sync::mpsc::channel();

    // Run storage engine process.
    let storage_thread = run_storage_worker(receiver);

    // Start the server
    log::info!("Starting server at {}:{} with at {}", cli.host, cli.port, cli.path);
    let thread_pool = Box::new(rayon::RayonThreadPool::new(THREAD_POOL_SIZE)?);
    let mut server = server::Server::new(thread_pool, sender, EXEC_TIMEOUT);
    server.listen(cli.host, cli.port)?;

    // TODO: graceful termination, for example Arc<bool> to exit infinite loops
    storage_thread.join().expect("Cannot gracefully shutdown storage engine");

    return Ok(());
}
