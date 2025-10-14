use tokio::sync::mpsc::Sender;

use super::{
    error::Error,
    types::{EdgeUpdate, UpdateStreamer},
};

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

    pub fn run(
        self,
        sender: Sender<Vec<EdgeUpdate>>,
    ) -> tokio::task::JoinHandle<Result<(), Error>> {
        tokio::spawn(async move { self.streamer.run_stream(sender).await })
    }
}
