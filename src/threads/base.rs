pub type Job = Box<dyn FnOnce() + Send + 'static>;

pub trait ThreadPool {
    fn spawn(&mut self, job: Job) -> Result<(), Box<dyn std::error::Error>>;
}
