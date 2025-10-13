use super::csr::GraphCSR;
use common::{error::Error, types::WeightedCycle};

/// Trait for graph solvers capable of detecting negative cycles.
pub trait GraphSolver {
    /// Detects a negative cycle reachable from `source`.
    ///
    /// Returns `Ok(Some(cycle))` if a negative cycle is found,
    /// `Ok(None)` if none exists, or `Err(e)` on failure.
    fn find_negative_cycle(
        &self,
        graph: &GraphCSR,
        source: usize,
        hop_cap: usize,
    ) -> Result<Option<WeightedCycle>, Error>;
}
