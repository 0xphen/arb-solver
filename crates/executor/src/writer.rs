use std::sync::Arc;
use tokio::select;
use tokio::sync::watch;
use tokio::sync::{RwLock, mpsc::Receiver};

use super::error::Error;
use arb_solver_core::GraphCSR;
use common::types::Edge;

/// Async consumer that applies edge updates to the shared graph.
pub struct Writer {
    graph: Arc<RwLock<GraphCSR>>,
    receiver: Receiver<Vec<Edge>>,
    shutdown: watch::Receiver<()>, // signal for graceful shutdown
}

impl Writer {
    pub fn new(
        graph: Arc<RwLock<GraphCSR>>,
        receiver: Receiver<Vec<Edge>>,
        shutdown: watch::Receiver<()>,
    ) -> Self {
        Self {
            graph,
            receiver,
            shutdown,
        }
    }

    /// Run the writer asynchronously.
    ///
    /// Consumes batches from the receiver and applies them to the graph.
    /// Releases the write lock immediately after each batch.
    /// Exits gracefully when the receiver is closed or shutdown signal is received.
    pub async fn process_updates(mut self) -> Result<(), Error> {
        println!("Writer ready.");

        loop {
            select! {
                updates = self.receiver.recv() => {
                    match updates {
                        Some(updates) => {
                            {
                                let mut graph_guard = self.graph.write().await;
                                eprintln!("Edges {:?} added to graph", updates);
                                graph_guard.add_edges(updates);
                            }
                        }
                        None => {
                            println!("Receiver closed, shutting down writer.");
                            break;
                        }
                    }
                }

                _ = self.shutdown.changed() => {
                    println!("Shutdown signal received, stopping writer.");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Public method that spawns the Writer task onto the Tokio runtime.
    ///
    /// This function consumes the Writer instance (`self`) and returns a JoinHandle,
    /// allowing the pipeline orchestrator to monitor the task.
    pub fn spawn_task(self) -> tokio::task::JoinHandle<Result<(), Error>> {
        tokio::spawn(self.process_updates())
    }
}
