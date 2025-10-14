use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{self, Duration};

use super::{error::Error, types::SharedGraph};
use arb_solver_core::{GraphCSR, traits::GraphSolver};

pub struct ArbSearcher<S> {
    solver: S,
    graph: SharedGraph,
    interval: u64, // interval in seconds
}

impl<S> ArbSearcher<S>
where
    S: GraphSolver,
{
    pub fn new(graph: SharedGraph, interval: u64, solver: S) -> Self {
        ArbSearcher {
            graph,
            interval,
            solver,
        }
    }

    pub async fn seacrh_for_arbs(self) -> Result<(), Error> {
        println!("Searcher ready.");

        let mut interval = time::interval(Duration::from_secs(self.interval));

        // The first tick occurs immediately, but we skip it to wait the full duration
        interval.tick().await;

        loop {
            interval.tick().await;

            let graph_snapshot = {
                let graph_guard = self.graph.read().await;
                graph_guard.clone()
            };

            // Only run the expensive search if the graph has meaningful data
            if graph_snapshot.num_nodes > 1 {
                println!("Searcher: Starting cycle search on new snapshot...");

                let cycle_result = self.solver.find_negative_cycle(
                    &graph_snapshot,
                    0,
                    graph_snapshot.num_nodes + 1,
                );

                match cycle_result {
                    Ok(Some(cycle)) => {
                        println!("Cycle FOUND! Path: {:?}", cycle.path);
                    }
                    Ok(None) => {
                        println!("Search complete: No arbitrage opportunities.");
                    }
                    Err(e) => {
                        eprintln!(
                            "Searcher Error: Graph cycle finder failed due to: {}. Continuing.",
                            e
                        );
                    }
                }
            } else {
                println!("Searcher: Graph too small to search for cycles. Skipping.");
            }
        }
    }
}
