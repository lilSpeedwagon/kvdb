use std::fmt;

use crate::types;

impl types::Deserializable for types::Value {
    fn deserialize(stream: &mut dyn std::io::Read) -> types::Result<Self> {
        let value_type = u8::deserialize(stream)? as char;
        match value_type {
            's' => {
                let value: String = types::Deserializable::deserialize(stream)?;
                Ok(types::Value::String { value })
            },
            _ => Err(Box::from(format!("Unknown value type: {}", value_type)))
        }
    }
}


impl types::Deserializable for types::Command {
    fn deserialize(stream: &mut dyn std::io::Read) -> types::Result<Self> {
        let cmd_type = u8::deserialize(stream)? as char;
        match cmd_type {
            's' => {
                let key: String = types::Deserializable::deserialize(stream)?;
                let value: String = types::Deserializable::deserialize(stream)?;
                Ok(types::Command::Set { key: key, value: value as Value })
            }
        }
    }
}

/// Server request header with metadata.
/// Reserved fields are used for protocol evolution.
pub struct RequestHeader {
    pub version: u16,
    pub keep_alive: u8,
    pub reserved: u8,
    pub command_count: u32,
    pub body_size: u32,
    pub reserved2: u32,
}

impl types::Deserializable for RequestHeader {
    fn deserialize(stream: &mut dyn std::io::Read) -> types::Result<Self> {
        Ok(
            RequestHeader{
                version: types::Deserializable::deserialize(stream)?,
                keep_alive: types::Deserializable::deserialize(stream)?,
                reserved: types::Deserializable::deserialize(stream)?,
                command_count: types::Deserializable::deserialize(stream)?,
                body_size: types::Deserializable::deserialize(stream)?,
                reserved2: types::Deserializable::deserialize(stream)?,
            }
        )
    }
}

pub struct RequestCommand {
    pub id: u32,
    pub command: types::Command,
}

impl types::Deserializable for RequestCommand {
    fn deserialize(stream: &mut dyn std::io::Read) -> types::Result<Self> {
        Ok(
            RequestCommand {
                id: types::Deserializable::deserialize(stream)?,
                command: types::Deserializable::deserialize(stream)?,
            }
        )
    }
}

/// A single server request.
pub struct Request {
    pub header: RequestHeader,
    pub commands: Vec<RequestCommand>,
}

impl types::Deserializable for Request {
    fn deserialize(stream: &mut dyn std::io::Read) -> types::Result<Self> {
        let header: RequestHeader = types::Deserializable::deserialize(stream)?;
        // TODO: limit max request body size for both announced size in header and the real size.

        let mut commands = vec![];
        for _ in 0..header.command_count {
            let cmd: RequestCommand = types::Deserializable::deserialize(stream)?;
            commands.push(cmd);
        }

        Ok(
            Request{
                header: header,
                commands: commands,
            }
        )
    }
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "<version={}; keep_alive={}; command_count={}, body_size={}>",
            self.header.version,
            self.header.keep_alive,
            self.header.command_count,
            self.header.body_size,
        )
    }
}

pub struct ResponseHeader {
    pub version: u8,
    pub reserved_1: u8,
    pub command_count: u16,
    pub body_size: u32,
    pub reserved_2: u32,
}

pub struct Response {
    pub header: ResponseHeader,
    pub commands: Vec<types::CommandResult>,
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "<version={}; command_count={}; body_size={}>",
            self.header.version,
            self.header.command_count,
            self.header.body_size,
        )
    }
}
