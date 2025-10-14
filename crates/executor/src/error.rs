use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Channel sender failed: Receiver has been dropped.")]
    ChannelSendFailed,
}

// impl std::error::Error for Error {}
