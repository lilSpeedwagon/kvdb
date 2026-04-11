use rayon;

use crate::threads::base;

pub struct RayonThreadPool {
    internal_pool: rayon::ThreadPool,
}

/// A thread pool wrapper for the `rayon` implementation. 
impl RayonThreadPool {
    pub fn new(size: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let pool = rayon::ThreadPoolBuilder::new().num_threads(size).build()?;
        Ok(RayonThreadPool { internal_pool: pool })
    }
}

impl base::ThreadPool for RayonThreadPool {
fn spawn(&mut self, job: base::Job) -> Result<(), Box<dyn std::error::Error>> {
        self.internal_pool.install(job);
        Ok(())
    }
}
