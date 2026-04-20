use crate::storage;
use crate::types;
use crate::cmd_queue::models;


/// A queue worker accepting storage commands from a MPSC channel and executing them one-by-one in a loop.
/// It acts as an async task buffer between multiple server workers threads and a single-threaded storage engine.
pub struct StorageCommandQueueWorker {
    cmd_queue: models::CmdQueueReceiver,
    engine: Box<dyn storage::base::KvStorage>,
}


impl StorageCommandQueueWorker {

    pub fn new(
        cmd_queue: models::CmdQueueReceiver,
        storage_engine: Box<dyn storage::base::KvStorage>,
    ) -> Self {
        StorageCommandQueueWorker { cmd_queue: cmd_queue, engine: storage_engine }
    }

    /// Runs storage processor loop until complete.
    /// The processor will wait for incoming storage commands from the command queue
    /// and process them one by one.
    pub fn run_in_loop(&mut self) {
        let is_running = true;
        log::info!("Storage engine is waiting for commands to execute");

        while is_running {
            if let Err(err) = self.handle_next_command() {
                log::error!("Cannot handle next command in the queue, the result might be lost: {}", err);
            }
        }

        log::info!("Storage engine loop stops");
    }

    /// Read the next command from the command queue and handle it.
    /// If command fails, an error result is written back.
    /// If queue management fails, the command is dropped and the error is logged.
    fn handle_next_command(&mut self) -> types::Result<()> {
        let next_queued_command = self.cmd_queue.recv()?;
        let command_to_execute = next_queued_command.command;
        log::debug!("Next command for execution: {}", command_to_execute);
        let cmd_result = self.handle_command(command_to_execute);
        
        let result_to_return = match cmd_result {
            Ok(result) => types::SafeResult::Ok(result),
            Err(err) => {
                log::error!("Command failed: {}", err);
                let error_msg = err.to_string();
                types::SafeResult::Err(Box::from(error_msg))
            },
        };
        next_queued_command.callback_channel.send(result_to_return)?;
        Ok(())
    }

    /// Pass a single queued command for execution to the storage engine.
    fn handle_command(&mut self, cmd: types::Command) -> types::Result<types::CommandResult> {
        match cmd {
            types::Command::Get { key } => {
                let value = self.engine.get(key)?;
                Ok(types::CommandResult::Get { value: value })
            },
            types::Command::Set { key, value } => {
                self.engine.set(key, value)?;
                Ok(types::CommandResult::Set {})
            },
            types::Command::Remove { key } => {
                self.engine.remove(key)?;
                Ok(types::CommandResult::Remove {})
            }
        }
    }
}
