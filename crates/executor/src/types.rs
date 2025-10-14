use tokio::sync::mpsc::Sender;

use super::error::Error;

/// A trait defining the contract for any source that generates and streams updates
/// into the main processing pipeline.
///
/// This trait is designed for **decoupling** the Producer task from the specific
/// data source (e.g., CSV file vs. simulated data).
///
/// The trait bounds (`Send`, `Sync`, `'static`) are mandatory to ensure the
/// implementation can be safely executed by the multi-threaded asynchronous runtime (Tokio).
#[async_trait::async_trait]
pub trait UpdateStreamer: Send + Sync + 'static {
    async fn run_stream(self, sender: Sender<Vec<EdgeUpdate>>) -> Result<(), Error>;
}

#[derive(Debug, Clone, Copy)]
pub struct EdgeUpdate {
    pub from: usize,   // Source Node ID
    pub to: usize,     // Target Node ID
    pub new_rate: f64, // The rate that replaces the old one
}
