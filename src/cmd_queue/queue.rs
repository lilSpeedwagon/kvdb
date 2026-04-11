use crate::storage;
use crate::types;
use crate::cmd_queue::models;


/// A queue processor accepting storage commands from a MPSC channel and executing them one-by-one in a loop.
/// It acts as an async task buffer between multiple server workers threads and a single-threaded storage engine.
struct StorageCommandQueue {
    cmd_queue: models::CmdQueueReceiver,
    engine: Box<dyn storage::base::KvStorage>,
}


impl StorageCommandQueue {

    pub fn new(
        cmd_queue: models::CmdQueueReceiver,
        storage_engine: Box<dyn storage::base::KvStorage>,
    ) -> Self {
        StorageCommandQueue { cmd_queue: cmd_queue, engine: storage_engine }
    }

    /// Runs storage processor loop until complete.
    /// The processor will wait for incoming storage commands from the command queue
    /// and process them one by one.
    pub fn run_in_loop(&mut self) {
        let is_running = true;
        log::info!("Storage engine is waiting for commands to execute");

        while is_running {
            let next_cmd = self.cmd_queue.recv();
            match next_cmd {
                Ok(cmd) => {
                    let cmd_result = self.handle_command(cmd.command);
                    if let Err(err) = &cmd_result {
                        log::error!("Command failed: {}", err);
                    }
                    cmd.callback_channel.send(cmd_result);
                },
                Err(err) => {
                    log::error!("Failed to receive next storage engine task: {}", err)
                }
            }
        }

        log::info!("Storage engine loop stops");
    }

    /// Pass a single queued command for execution to the storage engine.
    fn handle_command(&mut self, cmd: types::Command) -> types::Result<types::CommandResult> {
        log::debug!("Executing {}", cmd);
        match cmd {
            types::Command::Get { key } => {
                let value = self.engine.get(key)?;
                Ok(types::CommandResult::Get { value: value })
            },
            types::Command::Set { key, value } => {
                self.engine.set(key, value);
                Ok(types::CommandResult::Set {})
            },
            types::Command::Remove { key } => {
                self.engine.remove(key);
                Ok(types::CommandResult::Remove {})
            }
        }
    }
}
