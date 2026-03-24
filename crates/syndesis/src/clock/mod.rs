/// Clock synchronization for multi-room streaming.
pub mod estimator;
pub mod scheduler;

pub use estimator::ClockEstimator;
pub use scheduler::SyncScheduler;
