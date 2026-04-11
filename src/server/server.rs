use std::net;
use std::io;
use std::io::Write;
use std::time::Duration;

use crate::threads;
use crate::types;
use crate::server::models;
use crate::types::Deserializable;
use crate::cmd_queue;
use crate::types::Serializable;

const SERVER_VERSION: u16 = 1u16;
const CMD_EXEC_TIMEOUT: Duration = Duration::from_secs(30 * 60);  // TODO: move to config


fn validate_request(request: &models::Request) -> types::Result<()> {
    let header = &request.header;
    if header.version > SERVER_VERSION {
        return Err(
            Box::from(
                format!("Unsupported request version {}, server version: {}", header.version, SERVER_VERSION)
            )
        )
    }

    let mut unique_ids = std::collections::HashSet::with_capacity(header.command_count as usize);
    for cmd in &request.commands {
        let seen = !unique_ids.insert(cmd.id);
        if seen {
            return Err(
                Box::from(
                    format!("Request command IDs are expected to be unique. ID '{}' is duplicated", cmd.id)
                )
            )
        }
    }

    Ok(())
}


/// Handle a single command from an incoming request.
/// # Arguments
/// - `queue_sender` producer for db engine commands, part of MPSC queue
/// - `command` command to handle
/// - `timeout` command processing timeout
fn handle_command(
    queue_sender: &mut cmd_queue::models::CmdQueueSender,
    command: types::Command,
    timeout: Duration,
) -> types::SafeResult<types::CommandResult> {
    let cmd_str = format!("{}", command);
    log::debug!("Queueing command {} for execution", cmd_str);

    // Send the command to the mpsc queue and attach a callback result receiver channel.
    let (response_sender, response_receiver) = std::sync::mpsc::channel();
    let command_to_queue = cmd_queue::models::QueuedCommand{
        command: command,
        callback_channel: response_sender,
    };
    queue_sender.send(command_to_queue);
    
    let recv_result = response_receiver.recv_timeout(timeout);
    match recv_result {
        Ok(cmd_result) => cmd_result,
        Err(err) => {
            log::error!("Command {} execution result is not received after {}s: {}", cmd_str, timeout.as_secs(), err);
            Err(Box::from(format!("Command execution timeout after {}s", timeout.as_secs())))
        }
    }
}

/// Handle a single incoming connection.
/// Runs an inner loop to read multiple requests within one connection while `keep_alive` is sent.
/// Each request can contain multiple commands. Each command is queued via MPSC queue to be processed
/// by the storage engine.
/// # Arguments
/// - `queue_sender` producer for db engine commands, part of MPSC queue
/// - `stream` raw TCP stream of the received connection
fn handle_connection(
    mut queue_sender: cmd_queue::models::CmdQueueSender,
    mut stream: net::TcpStream,
) -> types::Result<()> {
    log::debug!("Handling incoming connection");

    loop {
        // Parse request commands.
        let request = models::Request::deserialize(&mut stream)?;
        validate_request(&request);

        log::debug!("Handling request {}", request);
        let keep_alive = request.header.keep_alive != 0;
        
        // Queue and handle all commands one-by-one.
        let mut responses = vec![];
        responses.reserve(request.commands.len());
        for cmd in request.commands {
            let result = match handle_command(&mut queue_sender, cmd.command, CMD_EXEC_TIMEOUT) {
                Ok(result) => {
                    models::CommandResultOrError::Result { result: result }
                },
                Err(err) => {
                    models::CommandResultOrError::Error { error_message: format!("{}", err) }
                },
            };
            let response_command = models::ResponseCommand{
                id: cmd.id,
                result: result,
            };
            responses.push(response_command);
        }
        
        // Prepare and write back the response.
        let mut response_body_buffer = vec![];
        responses.serialize(&mut response_body_buffer)?;
        let body_size = response_body_buffer.len();

        let response_header = models::ResponseHeader{
            version: SERVER_VERSION,
            command_count: responses.len() as u16,
            body_size: body_size as u32,
            reserved: 0u32,
        };

        let mut writer = io::BufWriter::new(&mut stream);
        response_header.serialize(&mut writer)?;
        writer.write(&response_body_buffer)?;
        writer.flush()?;
        drop(writer);

        // Wait for more requests if keep-alive is set or close the connection.
        // TODO: keepalive timeout
        if keep_alive {
            log::debug!("Request handled, keep connection alive");
            continue;
        } else {
            break;
        }
    }

    log::debug!("Request handled, close connection");
    match stream.shutdown(std::net::Shutdown::Both) {
        Ok(_) => {},
        Err(err) => { log::warn!("Cannot close socket gracefully: {}", err); }
    }
    Ok(())
}

/// A server with custom TCP-based communication protocol for KV DB.
pub struct Server {
    thread_pool: Box<dyn threads::base::ThreadPool>,
    cmd_queue: cmd_queue::models::CmdQueueSender,
    cmd_exec_timeout: Duration,
}


impl Server {
    // TODO: migrate to async server implementation (like tokio)

    pub fn new(
        thread_pool: Box<dyn threads::base::ThreadPool>,
        cmd_queue: cmd_queue::models::CmdQueueSender,
        cmd_exec_timeout: Duration,
    ) -> Self {
        Server{
            thread_pool: thread_pool,
            cmd_queue: cmd_queue,
            cmd_exec_timeout: cmd_exec_timeout,
        }
    }

    /// Start listening for incoming connection on the selected host:port in a loop.
    /// Each connection will be handled in a thread taken from the server thread pool.
    pub fn listen(&mut self, host: String, port: u32) -> types::Result<()> {
        let addr = format!("{}:{}", host, port);
        let listener = net::TcpListener::bind(addr)?;

        for connection_result in listener.incoming() {
            match connection_result {
                Ok(stream) => {
                    let queue = self.cmd_queue.clone();
                    if let Err(err) = self.thread_pool.spawn(
                        Box::new(move || {
                            match handle_connection(queue, stream) {
                                Ok(_) => {},
                                Err(err) => { log::error!("Request handling error: {}", err) }
                            }
                        })
                    ) {
                        log::error!("Cannot spawn a new thread to handle connection: {}", err);    
                    }
                },
                Err(err) => {
                    log::error!("Cannot handle incoming connection: {}", err);
                }
            }
        }

        Ok(())
    }
}
