use tokio::select;
use tokio::sync::mpsc::Receiver;
use tokio::sync::watch;

use super::error::Error;
use super::types::SharedGraph;
use arb_solver_core::csr::AddEdgeResult;
use common::types::Edge;

/// Async consumer that applies edge updates to the shared graph.
pub struct Writer {
    graph: SharedGraph,
    receiver: Receiver<Vec<Edge>>,
    batch_buffer: Vec<Edge>,
    batch_capacity: usize,
    shutdown: watch::Receiver<()>,
}

impl Writer {
    pub fn new(
        graph: SharedGraph,
        receiver: Receiver<Vec<Edge>>,
        shutdown: watch::Receiver<()>,
        batch_capacity: usize,
    ) -> Self {
        Self {
            graph,
            receiver,
            shutdown,
            batch_capacity,
            batch_buffer: Vec::with_capacity(batch_capacity),
        }
    }

    /// Flushes accumulated edge updates to the shared graph using a **Two-Phase Lock** strategy.
    ///
    /// Phase 1 (short lock): Atomically transfers pending updates out of the graph if a rebuild is needed.
    /// Unlocked Work: We **sort the edges** here (outside the lock) to perform the high-cost computation
    ///                without blocking readers.
    /// Phase 2 (short lock): Acquires lock briefly to commit the final, rebuilt graph state.
    async fn flush(&mut self) -> Result<(), Error> {
        if self.batch_buffer.is_empty() {
            return Ok(());
        }

        let rebuild_data = {
            println!("Flushing {} edges to graph", self.batch_buffer.len());

            let mut graph = self.graph.write().await;
            graph.add_edges_and_extract_data(std::mem::take(&mut self.batch_buffer))
        };

        if let AddEdgeResult::RebuildNeeded(mut edges) = rebuild_data {
            // We sort the edges for optimal efficiency before re-acquiring the lock
            edges.sort_by_key(|(src, _, _)| *src);
            println!("Initiating graph rebuild...");

            {
                let mut graph = self.graph.write().await;
                graph.rebuild_with_edges(edges);
            }
            println!("Graph rebuild complete.");
        }

        Ok(())
    }

    /// Run the writer asynchronously.
    ///
    /// Consumes batches from the receiver and applies them to the graph.
    /// Releases the write lock immediately after each batch.
    /// Exits when the receiver is closed or shutdown signal is received.
    pub async fn process_updates(mut self) -> Result<(), Error> {
        println!("Writer ready.");

        loop {
            select! {
                updates = self.receiver.recv() => {
                    match updates {
                        Some(updates) => {
                          self.batch_buffer.extend(updates);
                          if self.batch_buffer.len() >= self.batch_capacity {
                            self.flush().await?;
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
