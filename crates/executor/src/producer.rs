use tokio::sync::mpsc::Sender;

use super::{error::Error, types::UpdateStreamer};
use common::types::Edge;

pub struct Producer<S: UpdateStreamer> {
    streamer: S,
}

impl<S> Producer<S>
where
    S: UpdateStreamer,
{
    pub fn new(streamer: S) -> Self {
        Producer { streamer }
    }

    pub fn run(self, sender: Sender<Vec<Edge>>) -> tokio::task::JoinHandle<Result<(), Error>> {
        println!("Producer ready.");
        tokio::spawn(async move { self.streamer.run_stream(sender).await })
    }
}
