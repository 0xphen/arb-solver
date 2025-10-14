use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::mpsc::Sender;

use super::error::Error;
use arb_solver_core::GraphCSR;
use common::types::Edge;

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
    async fn run_stream(self, sender: Sender<Vec<Edge>>) -> Result<(), Error>;
}

pub type SharedGraph = Arc<RwLock<GraphCSR>>;

pub type JoinHandleResult = tokio::task::JoinHandle<Result<(), Error>>;

pub enum DataSource {
    SIM,
    CSV(String),
}
