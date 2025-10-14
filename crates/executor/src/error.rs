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

    #[error("CSV data parsing error: {0}")]
    CsvParseError(#[from] csv::Error),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}
