use config::ConfigError;
use thiserror::Error;

use common::error::Error as ArbSolverError;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Channel sender failed: Receiver has been dropped.")]
    ChannelSendFailed,

    #[error("Graph processing error: {0}")]
    GraphError(#[from] ArbSolverError),

    #[error("Configuration error: {0}")]
    ConfigLoadError(String),
}
