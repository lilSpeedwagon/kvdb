use core::time;
use std::net;
use std::io;
use std::io::{Read, Write};
use std::time::Duration;

use crate::storage;
use crate::threads;
use crate::types;
use crate::server::models;
use crate::server::serialize;
use crate::types::Deserializable;
use crate::cmd_queue;

const SERVER_VERSION: u16 = 1u16;
const CMD_EXEC_TIMEOUT: Duration = Duration::from_secs(30 * 60);  // TODO: move to config


fn serialize_response(responses: Vec<models::ResponseCommand>) -> models::Result<Vec<u8>> {
    let command_count = responses.len();
    let mut body_buffer = Vec::new();
    for response in responses {
        match response {
            models::ResponseCommand::Get { value } => {
                body_buffer.write(&[b'g'])?;
                value.serialize(&mut body_buffer)?;
            },
            models::ResponseCommand::Set {} => {
                body_buffer.write(&[b's'])?;
            },
            models::ResponseCommand::Remove {} => {
                body_buffer.write(&[b'r'])?;
            },
            models::ResponseCommand::Reset {} => {
                body_buffer.write(&[b'z'])?;
            }
        };
    }

    let header =  models::ResponseHeader{
        version: SERVER_VERSION,
        reserved_1: 0u8,
        command_count: command_count as u16,
        body_size: body_buffer.len() as u32,
        reserved_2: 0u32,
    };

    let mut response_buffer = Vec::new();
    response_buffer.reserve(size_of::<models::ResponseHeader>() + body_buffer.len());
    header.version.serialize(&mut response_buffer)?;
    header.reserved_1.serialize(&mut response_buffer)?;
    header.command_count.serialize(&mut response_buffer)?;
    header.body_size.serialize(&mut response_buffer)?;
    header.reserved_2.serialize(&mut response_buffer)?;
    response_buffer.extend(body_buffer.iter());

    Ok(response_buffer)
}

fn handle_request(
    storage: &mut kv_log::KvLogStorage,
    request: models::Request,
) -> types::Result<Vec<types::CommandResult>> {
    let mut responses = Vec::new();

    for request_command in request.commands {
        let cmd = request_command.command;
        log::info!("Handling command {}", cmd);
        let response_command = match command {
            models::Command::Get { key } => {
                let value = storage.get(key)?;
                models::ResponseCommand::Get{value: value}
            },
            models::Command::Set { key, value } => {
                storage.set(key, value)?;
                models::ResponseCommand::Set{}
            },
            models::Command::Remove { key } => {
                storage.remove(key)?;
                models::ResponseCommand::Remove{}
            },
            models::Command::Reset { } => {
                storage.reset()?;
                models::ResponseCommand::Reset{}
            },
        };
        responses.push(response_command);
    }

    Ok(responses)
}

fn handle_command(
    queue_sender: cmd_queue::models::CmdQueueSender,
    command: types::Command,
    timeout: Duration,
) -> types::Result<types::CommandResult> {
    log::debug!("Queueing command {} for execution", command);

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
            log::error!("Command {} execution result is not received after {}s: {}", command, timeout.as_secs(), err);
            Err(Box::from(format!("Command execution timeout after {}s", timeout.as_secs())))
        }
    }
}

/// Handle a single incoming connection.
/// Runs an inner loop to read multiple requests within one connection while `keep_alive` is sent.
/// # Arguments
/// - `queue_sender` producer for db engine commands, part of MPSC queue
/// - `stream` raw TCP stream of the received connection
fn handle_connection(
    mut queue_sender: cmd_queue::models::CmdQueueSender,
    mut stream: net::TcpStream,
) -> types::Result<()> {
    log::debug!("Handling incoming connection");

    loop {
        let request = models::Request::deserialize(&mut stream)?;
        let header = request.header;
        if header.version > SERVER_VERSION {
            return Err(
                Box::from(
                    format!("Unsupported request version {}, server version: {}", header.version, SERVER_VERSION)
                )
            )
        }
        let keep_alive = request.header.keep_alive != 0;

        // Queue and handle all commands one-by-one.
        log::debug!("Handling request {}", request);
        let responses = vec![];
        responses.reserve(request.commands.len());
        for cmd in request.commands {
            let response = handle_command(queue_sender, cmd.command, CMD_EXEC_TIMEOUT)?;
            responses.push(response);
        }

        let response_data = serialize_response(responses)?;
        log::debug!("{}", String::from_utf8_lossy(&response_data));
        
        let mut writer = io::BufWriter::new(&mut stream);
        writer.write(response_data.as_slice())?;
        writer.flush()?;
        drop(writer);

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
