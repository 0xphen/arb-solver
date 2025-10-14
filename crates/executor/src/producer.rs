use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;

use super::types::UpdateStreamer;
use common::types::Edge;

pub struct Producer<S: UpdateStreamer> {
    streamer: S,
}

impl<S> Producer<S>
where
    S: UpdateStreamer + Send + 'static,
{
    pub fn new(streamer: S) -> Self {
        Self { streamer }
    }

    /// Spawn the producer task and return its JoinHandle
    pub fn spawn(self, sender: Sender<Vec<Edge>>) -> JoinHandle<()> {
        println!("Producer ready.");

        tokio::spawn(async move {
            if let Err(e) = self.streamer.run_stream(sender).await {
                eprintln!(
                    "Producer Task FAILED: Streamer encountered a critical error: {}",
                    e
                );
            }
        })
    }
}
