use crate::types;


/// Channel sender for a single executed command result.
pub type CmdResultSender = std::sync::mpsc::Sender<types::Result<types::CommandResult>>;
/// Channel receiver for a single executed command result.
pub type CmdResultReceiver = std::sync::mpsc::Sender<types::Result<types::CommandResult>>;

/// Dataclass for a single command ququed for execution with a callback sender.
pub struct QueuedCommand {
    pub command: types::Command,
    pub callback_channel: CmdResultSender,
}

/// Channel sender for a single command to be executed.
pub type CmdQueueSender = std::sync::mpsc::Sender<QueuedCommand>;
/// Channel receiver for a single command to be executed.
pub type CmdQueueReceiver = std::sync::mpsc::Receiver<QueuedCommand>;
