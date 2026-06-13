use std::io::Write;
use std::net;
use std::time;

use crate::types;
use crate::types::{Deserializable, Serializable};
use crate::proto;

// A simple KVDB client built on top of TCP.
pub struct Client {
    socket: Option<net::TcpStream>,
}

impl Drop for Client {
    fn drop(&mut self) {
        if let Some(err) = self.close().err() {
            log::error!("Cannot close client: {}", err);
        }
    }
}

impl Client {
    pub fn new() -> Self {
        Client{
            socket: None
        }
    }

    fn get_socket(&mut self) -> types::Result<&mut net::TcpStream> {
        if !self.is_connected() {
            return Err(Box::from(format!("Client socket is not connected")));
        }
        Ok(self.socket.as_mut().unwrap())
    }

    /// Connect to a remote server with the given address.
    pub fn connect(&mut self, host: String, port: u32, timeout: time::Duration) -> types::Result<()> {
        if self.is_connected() {
            log::debug!("The client socket is already in use. Reopen at {}:{}", host, port);
            self.close()?;
        }

        let addr = format!("{}:{}", host, port);
        log::debug!("Connecting to {}...", addr);
        let socket = net::TcpStream::connect(addr)?;
        socket.set_read_timeout(Some(timeout))?;
        self.socket = Some(socket);
        log::debug!("Connected. Read timeout {}s", timeout.as_secs_f32());
        Ok(())
    }

    /// Close the current connection if some.
    pub fn close(&mut self) -> types::Result<()> {
        if !self.is_connected() {
            log::debug!("The client socket is already closed.");
            return Ok(());
        }

        let socket = self.get_socket()?;
        if let Err(err) = socket.flush() {
            log::warn!("Cannot flush client socket: {}", err);
        }
        if let Err(err) = socket.shutdown(net::Shutdown::Both) {
            log::warn!("Cannot close client socket: {}", err);
        }
        self.socket = None;

        Ok(())
    }

    /// Returns `true` if the client is connected to a remote server.
    /// Note that there is no actual liveness check,
    /// it just checks if the connection was established previously.
    pub fn is_connected(&self) -> bool {
        return self.socket.is_some();
    }

    fn serialize(cmd: types::Command) -> types::Result<Vec<u8>> {
        let request_command = proto::models::RequestCommand{
            id: 0,
            command: cmd,
        };
        let commands = vec![request_command];
        let mut body_buffer: Vec<u8> = vec!();
        commands.serialize(&mut body_buffer)?;
        
        let header = proto::models::RequestHeader{
            version: proto::version::PROTO_VERSION,
            keep_alive: 0u8,  // TODO: support keep alive
            reserved: 0u8,
            command_count: commands.len() as u32,
            body_size: body_buffer.len() as u32,
            reserved2: 0u32,
        };
        
        let mut buffer = vec![];
        header.serialize(&mut buffer)?;
        buffer.append(&mut body_buffer);

        Ok(buffer)
    }

    /// Run a single command on a remote server.
    /// The client must be connected to a remote server.
    pub fn execute(&mut self, cmd: types::Command) -> types::Result<()> {
        let socket = self.get_socket()?;
        
        let request_data = Self::serialize(cmd)?;
        socket.write(&request_data)?;

        // TODO: recv and deserialize on fly

        Ok(())
    }
}
