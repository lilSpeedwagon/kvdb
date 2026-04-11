use crate::types;


/// A thread safe variant of result.
pub type SafeResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Channel sender for a single executed command result.
pub type CmdResultSender = std::sync::mpsc::Sender<SafeResult<types::CommandResult>>;
/// Channel receiver for a single executed command result.
pub type CmdResultReceiver = std::sync::mpsc::Sender<SafeResult<types::CommandResult>>;

/// Dataclass for a single command ququed for execution with a callback sender.
pub struct QueuedCommand {
    pub command: types::Command,
    pub callback_channel: CmdResultSender,
}

/// Channel sender for a single command to be executed.
pub type CmdQueueSender = std::sync::mpsc::Sender<QueuedCommand>;
/// Channel receiver for a single command to be executed.
pub type CmdQueueReceiver = std::sync::mpsc::Receiver<QueuedCommand>;
