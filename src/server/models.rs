use std::{fmt, io::Write, process::CommandArgs};

use crate::types;


impl types::Deserializable for types::Value {
    fn deserialize(stream: &mut dyn std::io::Read) -> types::Result<Self> {
        let value_type = u8::deserialize(stream)? as char;
        match value_type {
            's' => {
                let value: String = types::Deserializable::deserialize(stream)?;
                Ok(types::Value::String { value })
            },
            _ => Err(Box::from(format!("Unknown value type: '{}'", value_type)))
        }
    }
}


impl types::Deserializable for types::Command {
    fn deserialize(stream: &mut dyn std::io::Read) -> types::Result<Self> {
        let cmd_type = u8::deserialize(stream)? as char;
        match cmd_type {
            's' => {
                let key: String = types::Deserializable::deserialize(stream)?;
                let value_str: String = types::Deserializable::deserialize(stream)?;
                let value = types::Value::String { value: value_str };
                Ok(types::Command::Set { key: key, value: value })
            },
            'g' => {
                let key: String = types::Deserializable::deserialize(stream)?;
                Ok(types::Command::Get { key: key })
            },
            'r' => {
                let key: String = types::Deserializable::deserialize(stream)?;
                Ok(types::Command::Remove { key: key })
            },
            _ => {
                Err(Box::from(format!("Unknown command type '{}'", cmd_type)))
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

impl types::Serializable for types::Value {
    fn serialize(&self, stream: &mut dyn std::io::Write) -> types::Result<()> {
        match self {
            types::Value::String { value } => {
                value.serialize(stream)?;
            },
        }
        Ok(())
    }
}

impl types::Serializable for types::CommandResult {
    fn serialize(&self, stream: &mut dyn std::io::Write) -> types::Result<()> {
        let mut buffer = vec![];
        match self {
            types::CommandResult::Get { value } => {
                buffer.write(&[b'g'])?;
                value.serialize(&mut buffer)?;
            },
            types::CommandResult::Set {} => {
                buffer.write(&[b's'])?;
            },
            types::CommandResult::Remove {} => {
                buffer.write(&[b'r'])?;
            },
        };
        
        stream.write(&buffer)?;
        Ok(())
    }
}

pub struct ResponseHeader {
    pub version: u16,
    pub command_count: u16,
    pub body_size: u32,
    pub reserved: u32,
}

impl types::Serializable for ResponseHeader {
    fn serialize(&self, stream: &mut dyn std::io::Write) -> types::Result<()> {
        self.version.serialize(stream)?;
        self.command_count.serialize(stream)?;
        self.body_size.serialize(stream)?;
        self.reserved.serialize(stream)?;
        Ok(())
    }
}

pub enum CommandResultOrError {
    Result { result: types::CommandResult },
    Error { error_message: String },    
}

impl types::Serializable for CommandResultOrError {
    fn serialize(&self, stream: &mut dyn std::io::Write) -> types::Result<()> {
        let mut buffer = vec![];
        
        match self {
            CommandResultOrError::Result{ result } => {
                let is_ok = [1u8];
                buffer.write(&is_ok)?;
                result.serialize(&mut buffer);
            },
            CommandResultOrError::Error{ error_message } => {
                let is_ok = [0u8];
                buffer.write(&is_ok)?;
                error_message.serialize(&mut buffer);
            },
        }
        
        stream.write(&buffer)?;
        Ok(())
    }
}

pub struct ResponseCommand {
    pub id: u32,
    pub result: CommandResultOrError,
}

impl types::Serializable for ResponseCommand {
    fn serialize(&self, stream: &mut dyn std::io::Write) -> types::Result<()> {
        self.id.serialize(stream)?;
        self.result.serialize(stream)?;
        Ok(())
    }
}

pub struct Response {
    pub header: ResponseHeader,
    pub commands: Vec<ResponseCommand>,
}

impl types::Serializable for Response {
    fn serialize(&self, stream: &mut dyn std::io::Write) -> types::Result<()> {
        self.header.serialize(stream)?;
        self.commands.serialize(stream)?;
        Ok(())
    }
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
