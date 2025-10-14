use async_trait::async_trait;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use tokio::sync::mpsc::Sender;
use tokio::time::{self, Duration};

use super::error::Error;
use super::types::UpdateStreamer;
use common::types::Edge;

/// Interval between simulation updates in milliseconds.
const SIMULATION_INTERVAL_MS: u64 = 100;

/// Maximum fluctuation applied to edge rates (0.5 bps).
const RATE_FLUCTUATION: f64 = 0.000005;

/// Produces synthetic edge updates for simulation purposes.
///
/// Generates batches of `EdgeUpdate` events with randomized
/// source/target nodes and rate fluctuations, and sends them
/// over a Tokio bounded channel for processing.
pub struct SimulatorStreamer {
    pub total_nodes: usize, // total nodes in the network
    pub batch_size: usize,  // number of updates per batch
}

#[async_trait]
impl UpdateStreamer for SimulatorStreamer {
    /// Runs the simulation asynchronously.
    ///
    /// Periodically generates batches of edge updates and sends
    /// them via the provided `Sender`. Backpressure is handled
    /// naturally via awaiting on `sender.send()`. Exits gracefully
    /// if the receiver is dropped.
    async fn run_stream(self, sender: Sender<Vec<Edge>>) -> Result<(), Error> {
        let mut interval = time::interval(Duration::from_millis(SIMULATION_INTERVAL_MS));

        let mut rng: SmallRng = SmallRng::from_os_rng();

        let rate_range = -RATE_FLUCTUATION..=RATE_FLUCTUATION;
        let node_range = 0..self.total_nodes;

        loop {
            interval.tick().await;

            // Generate a batch of edge updates
            let updates: Vec<Edge> = (0..self.batch_size)
                .map(|_| {
                    let from = rng.random_range(node_range.clone());
                    let to = rng.random_range(node_range.clone());
                    let fluctuation = rng.random_range(rate_range.clone());
                    let new_rate = 1.0 + fluctuation;

                    (from, to, new_rate)
                })
                .collect();

            // Send batch, exit if receiver has been dropped
            println!("Producer sent {} updates.", updates.len());
            if sender.send(updates).await.is_err() {
                println!("Simulator shutting down: Writer receiver dropped.");
                return Err(Error::ChannelSendFailed);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    use tokio::time::{Duration, timeout};

    /// SimulatorStreamer can be created correctly.
    #[test]
    fn test_simulator_creation() {
        let sim = SimulatorStreamer {
            total_nodes: 10,
            batch_size: 5,
        };
        assert_eq!(sim.total_nodes, 10);
        assert_eq!(sim.batch_size, 5);
    }

    /// SimulatorStreamer generates correct number of updates in a batch.
    #[tokio::test]
    async fn test_batch_size() {
        let sim = SimulatorStreamer {
            total_nodes: 10,
            batch_size: 5,
        };

        let (tx, mut rx) = mpsc::channel(10);

        // Run simulator for one tick using timeout to avoid infinite loop
        tokio::spawn(async move {
            let _ = sim.run_stream(tx).await;
        });

        // Receive first batch
        let updates = timeout(Duration::from_millis(200), rx.recv())
            .await
            .expect("Did not receive batch")
            .expect("Channel closed");

        assert_eq!(updates.len(), 5);
    }

    /// All generated node indices are within bounds.
    #[tokio::test]
    async fn test_node_indices_in_bounds() {
        let sim = SimulatorStreamer {
            total_nodes: 10,
            batch_size: 50,
        };

        let (tx, mut rx) = mpsc::channel(10);

        tokio::spawn(async move {
            let _ = sim.run_stream(tx).await;
        });

        let updates = timeout(Duration::from_millis(200), rx.recv())
            .await
            .expect("Did not receive batch")
            .expect("Channel closed");

        for (u, v, w) in updates {
            assert!(u < 10, "from node out of bounds");
            assert!(v < 10, "to node out of bounds");
            assert!(
                w >= 1.0 - RATE_FLUCTUATION && w <= 1.0 + RATE_FLUCTUATION,
                "rate out of bounds"
            );
        }
    }
}
