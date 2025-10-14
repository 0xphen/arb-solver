use std::fmt;

#[derive(Debug)]
pub enum Error {
    /// Indicates an attempt to access a node index that exceeds the graph size (N).
    NodeIndexOutOfBounds(usize),

    /// Indicates a structural inconsistency found during graph processing or validation.
    InvalidGraph,

    /// Failed to trace the full cycle path, usually due to broken predecessor chains.
    CycleReconstructionFailed,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::NodeIndexOutOfBounds(n) => write!(f, "Node index {} is out of bounds.", n),

            Error::InvalidGraph => write!(f, "Graph structure is invalid or inconsistent."),

            Error::CycleReconstructionFailed => write!(
                f,
                "Cycle path reconstruction failed due to broken predecessor chain."
            ),
        }
    }
}

impl std::error::Error for Error {}
