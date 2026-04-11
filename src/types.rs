use std::fmt;


pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// A thread safe variant of result.
pub type SafeResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub type Key = String;

#[derive(Clone)]
pub enum Value {
    String { value: String },
}

pub enum Command {
    Get { key: Key },
    Set { key: Key, value: Value },
    Remove { key: Key },
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: limit output length for long keys/values
        match self {
            Command::Get { key } => {
                write!(f, "<Get key={}>", key)
            },
            Command::Set { key, value } => {
                let val_str = match value {
                    Value::String { value } => value
                };
                write!(f, "<Set key={} value={}>", key, val_str)
            },
            Command::Remove { key } => {
                write!(f, "<Remove key={}>", key)
            }
        }
    }
}

pub enum CommandResult {
    Get { value: Option<Value> },
    Set {},
    Remove {},
}

pub trait Serializable {
    fn serialize(&self, stream: &mut dyn std::io::Write) -> Result<()>;
}

pub trait Deserializable {
    fn deserialize(stream: &mut dyn std::io::Read) -> Result<Self> where Self: Sized;
}
