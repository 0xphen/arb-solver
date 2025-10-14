use super::csr::GraphCSR;
use super::traits::GraphSolver;
use common::{
    error::Error,
    types::{Edge, WeightedCycle},
};
use std::collections::VecDeque;
use std::f64;

/// Solver implementing the Shortest Path Faster Algorithm (SPFA) for single-source shortest paths
/// and negative cycle detection.
pub struct SPFASolver;

impl SPFASolver {
    /// Reconstructs a negative cycle in the graph after SPFA detects it.
    ///
    /// When SPFA detects a negative cycle, it flags a node that has been relaxed
    /// too many times (`count[v] > hop_cap`). This node may not be directly on the
    /// cycle itself—it could be downstream. This function traces back predecessors
    /// to reliably locate a node within the cycle and reconstruct the entire cycle.
    ///
    /// # Arguments
    /// * `start` - Node flagged by SPFA as part of a potential negative cycle.
    /// * `pred_edge_idx` - Array of optional CSR edge indices representing the predecessor
    ///                     edge for each node (from SPFA relaxations).
    /// * `graph` - The graph in CSR format, containing edge targets, weights, and source mapping.
    ///
    /// # Returns
    /// `Result<WeightedCycle, Error>` containing:
    /// * `path` - Sequence of edges `(u, v, rate)` forming the negative cycle in forward order.
    /// * `rates` - Vector of original rates corresponding to each edge in the cycle.
    /// * `log_rate_sum` - Sum of transformed weights (`-ln(rate)`) along the cycle.
    ///
    /// # Errors
    /// Returns `Error::InvalidGraph` if `start` is out of bounds, or
    /// `Error::CycleReconstructionFailed` if the cycle cannot be reconstructed.
    pub fn reconstruct_cycle(
        &self,
        start: usize,
        pred_edge_idx: &[Option<usize>],
        graph: &GraphCSR,
    ) -> Result<WeightedCycle, Error> {
        let num_nodes = graph.num_nodes;
        if start >= num_nodes {
            return Err(Error::InvalidGraph);
        }

        // Trace backwards up by `num_nodes` steps to ensure we reach a node
        // inside the negative cycle.
        let mut trace_node = start;
        for _ in 0..num_nodes {
            let edge_idx = pred_edge_idx[trace_node].ok_or(Error::CycleReconstructionFailed)?;
            trace_node = graph.get_edge_source_node(edge_idx)?;
        }

        let cycle_start_node = trace_node;
        let mut cycle_edge_indices: Vec<usize> = Vec::new();
        let mut current_node = cycle_start_node;

        loop {
            let edge_idx = pred_edge_idx[current_node].ok_or(Error::CycleReconstructionFailed)?;
            cycle_edge_indices.push(edge_idx);

            let source_node = graph.get_edge_source_node(edge_idx)?;
            current_node = source_node;

            if current_node == cycle_start_node {
                break;
            }
        }

        cycle_edge_indices.reverse();

        let len = cycle_edge_indices.len();
        let mut path: Vec<Edge> = Vec::with_capacity(len);
        let mut rates: Vec<f64> = Vec::with_capacity(len);
        let mut log_rate_sum = 0.0f64;

        for &edge_idx in &cycle_edge_indices {
            let weight = graph.edge_weights[edge_idx];
            let v = graph.edge_targets[edge_idx];
            let u = graph.get_edge_source_node(edge_idx)?;

            let rate = (-weight).exp();
            path.push((u, v, rate));
            rates.push(rate);
            log_rate_sum += weight;
        }

        Ok(WeightedCycle {
            path,
            rates,
            log_rate_sum,
        })
    }
}

impl GraphSolver for SPFASolver {
    /// Finds the shortest path from `source` and detects the first reachable negative cycle (SPFA).
    ///
    /// # Parameters
    /// - `graph`: The CSR data structure for fast edge traversal.
    /// - `source`: Starting node ID.
    /// - `hop_cap`: Max relaxations per node (typically N).
    ///
    /// # Returns
    /// - `Ok(Some(cycle))` → Profitable cycle found.
    /// - `Ok(None)` → No negative cycle found.
    /// - `Err(e)` → Error occurred.
    fn find_profitable_cycle(
        &self,
        graph: &GraphCSR,
        source: usize,
        hop_cap: usize,
    ) -> Result<Option<WeightedCycle>, Error> {
        if source >= graph.num_nodes {
            return Err(Error::NodeIndexOutOfBounds(source));
        }

        let num_nodes = graph.num_nodes;
        let mut distance = vec![f64::INFINITY; num_nodes];
        let mut count = vec![0; num_nodes]; // Tracks relaxations/hops
        let mut in_queue = vec![false; num_nodes];

        // Stores the CSR index of the predecessor edge.
        let mut pred_edge_idx = vec![None; num_nodes];

        let mut queue = VecDeque::with_capacity(num_nodes);

        // distance[source] = 0.0;
        // queue.push_back(source);
        // in_queue[source] = true;

        // To guarantee detection of any negative cycle in the entire graph, regardless of
        // whether the arbitrary 'source' node can reach it (i.e., handling disconnected components),
        // we initialize all nodes to a distance of 0.0 and add them to the queue.
        // This simulates connecting a virtual zero-weight source to every node.
        for i in 0..num_nodes {
            distance[i] = 0.0;
            queue.push_back(i);
            in_queue[i] = true;
        }

        // SPFA Loop: Propagate distances while the queue is not empty.
        while let Some(u) = queue.pop_front() {
            in_queue[u] = false;

            let start = graph.node_pointers[u];
            let end = graph.node_pointers[u + 1];

            // Traverse edges u -> v
            // 'i' is the CSR index of the edge (u,v)
            for i in start..end {
                let v = graph.edge_targets[i];
                let weight = graph.edge_weights[i];
                if distance[u] + weight < distance[v] {
                    distance[v] = distance[u] + weight;
                    pred_edge_idx[v] = Some(i);

                    count[v] += 1;
                    if count[v] >= hop_cap {
                        let cycle = self.reconstruct_cycle(v, &pred_edge_idx, graph)?;
                        return Ok(Some(cycle));
                    }

                    if !in_queue[v] {
                        queue.push_back(v);
                        in_queue[v] = true;
                    }
                }
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod spfa_tests {
    use super::*;
    use common::types::Edge;

    fn build_graph(edges: &mut [Edge], num_nodes: usize) -> GraphCSR {
        GraphCSR::from_edges(num_nodes, edges, edges.len())
    }

    #[test]
    fn reconstruct_cycle_small_graph() {
        let mut edges = vec![(0, 1, 1.0), (1, 2, 0.5), (2, 0, 0.5)];
        let graph = build_graph(&mut edges, 3);

        let pred_edge_idx = vec![Some(2), Some(0), Some(1)];
        let solver = SPFASolver;
        let cycle = solver.reconstruct_cycle(0, &pred_edge_idx, &graph).unwrap();

        assert_eq!(cycle.path.len(), 3);
        assert_eq!(cycle.path[0], (0, 1, 1.0));
    }

    #[test]
    fn spfa_detects_simple_negative_cycle() {
        let mut edges = vec![(1, 0, 2.0), (0, 1, 2.0)];
        let graph = build_graph(&mut edges, 2);

        let solver = SPFASolver;

        let cycle = solver.find_profitable_cycle(&graph, 0, 2).unwrap();
        assert!(cycle.is_some());

        let cycle = cycle.unwrap();
        assert_eq!(cycle.path, vec![(1, 0, 2.0), (0, 1, 2.0)]);
        assert!(cycle.log_rate_sum < 0.0);
    }

    #[test]
    fn spfa_no_negative_cycle_returns_none() {
        let mut edges = vec![(0, 1, 1.0), (1, 2, 1.2), (2, 3, 1.2)];
        let graph = build_graph(&mut edges, 4);
        let solver = SPFASolver;

        let cycle = solver.find_profitable_cycle(&graph, 0, 4).unwrap();
        assert!(cycle.is_none());
    }

    #[test]
    fn spfa_single_node_graph() {
        let graph = build_graph(&mut [], 1);
        let solver = SPFASolver;

        let cycle = solver.find_profitable_cycle(&graph, 0, 1).unwrap();
        assert!(cycle.is_none());
    }

    #[test]
    fn spfa_empty_graph_returns_error() {
        let graph = build_graph(&mut [], 0);
        let solver = SPFASolver;

        let result = solver.find_profitable_cycle(&graph, 0, 1);
        assert!(result.is_err());
    }

    // ----------------------------
    // Stress and edge-case tests
    // ----------------------------

    #[test]
    fn spfa_large_linear_graph_no_cycle() {
        let n = 1000;
        let mut edges: Vec<Edge> = (0..n - 1).map(|i| (i, i + 1, 1.0)).collect();
        let graph = build_graph(&mut edges, n);
        let solver = SPFASolver;

        let cycle = solver.find_profitable_cycle(&graph, 0, n).unwrap();
        assert!(cycle.is_none());
    }

    #[test]
    fn spfa_large_circular_graph_negative_cycle() {
        let n = 1000;
        let mut edges: Vec<(usize, usize, f64)> = (0..n)
            .map(|i| {
                let next = (i + 1) % n;
                let rate = 1.001;
                (i, next, rate)
            })
            .collect();

        let graph = build_graph(&mut edges, n);

        let solver = SPFASolver;
        let cycle = solver.find_profitable_cycle(&graph, 0, n + 1).unwrap();
        assert!(cycle.is_some());
        let cycle = cycle.unwrap();
        assert!(cycle.log_rate_sum < 0.0);
        assert!(cycle.path.len() <= n);
    }

    #[test]
    fn spfa_detects_arbitrage_in_disconnected_component() {
        let mut edges: Vec<Edge> = vec![
            // Reachable from default source (Node 0), but unprofitable.
            // Product: 1.0 * 0.5 * 0.5 = 0.25 (LOSS / Positive Cycle)
            (0, 1, 1.0),
            (1, 2, 0.5),
            (2, 0, 0.5),
            // Disconnected from source 0, but profitable.
            // Product: 1.0 * 1.1 = 1.1 (PROFIT / Negative Cycle)
            (3, 4, 1.0),
            (4, 3, 1.1),
        ];
        let graph = build_graph(&mut edges, 5);

        let solver = SPFASolver;

        let cycle_option = solver
            .find_profitable_cycle(&graph, 0, 5)
            .expect("SPFA execution returned an unexpected error.");

        // Ensure the profitable cycle was found (cycle_option is Some).
        assert!(
            cycle_option.is_some(),
            "SPFA failed to detect the guaranteed arbitrage cycle (nodes 3-4-3)."
        );

        let cycle = cycle_option.unwrap();

        assert!(
            cycle.product_rate() > 1.0,
            "The found cycle must be financially profitable (Product > 1.0)."
        );
        assert!(
            cycle.log_rate_sum < 0.0,
            "The log sum must be negative (Negative Cycle proof)."
        );

        let nodes_in_cycle: Vec<usize> = cycle.path.iter().map(|(u, _, _)| *u).collect();

        assert!(
            nodes_in_cycle.contains(&3),
            "The cycle path must originate from node 3 (part of the profitable cycle)."
        );
        assert!(
            nodes_in_cycle.contains(&4),
            "The cycle path must originate from node 4 (part of the profitable cycle)."
        );
    }

    #[test]
    fn spfa_disconnected_graph_detects_cycle_only_in_component() {
        let mut edges: Vec<Edge> = vec![
            // Component 1: Non-profitable (Break-even / Zero-weight cycle)
            (0, 1, 1.0),
            (1, 0, 1.0),
            // Component 2: Profitable (NEGATIVE cycle)
            // 0.5 * 2.1 = 1.05 > 1.0
            (2, 3, 0.5),
            (3, 2, 2.1), // Rate > 1.0 to guarantee profit
        ];
        let graph = build_graph(&mut edges, 4);
        let solver = SPFASolver;

        // NOTE: If global search is implemented, both calls will return the SAME cycle.
        // We only assert the existence of the profitable cycle (Component 2).

        // Check if the overall graph contains a negative cycle (Component 2).
        let cycle_option = solver.find_profitable_cycle(&graph, 0, 4).unwrap();

        assert!(
            cycle_option.is_some(),
            "SPFA failed to detect the guaranteed arbitrage cycle (nodes 2-3-2)."
        );

        let cycle = cycle_option.unwrap();

        assert_eq!(
            cycle.path.len(),
            2,
            "The profitable cycle must be 2 edges long."
        );
        assert!(
            cycle.product_rate() > 1.0,
            "The detected cycle must be financially profitable."
        );

        let nodes: Vec<usize> = cycle.path.iter().map(|(u, _, _)| *u).collect();
        assert!(
            nodes.contains(&2) && nodes.contains(&3),
            "The cycle path must include nodes 2 and 3."
        );
    }

    #[test]
    fn spfa_random_negative_cycle_large_graph() {
        let n = 50;
        // Set all forward path edges to break-even (rate 1.0, weight 0.0)
        let mut edges: Vec<Edge> = (0..n - 1).map(|i| (i, i + 1, 1.0)).collect();

        // This creates a NEGATIVE cycle at the end of the path.
        edges.push((n - 1, n - 2, 0.5)); // Loss/Fee (weight > 0)
        edges.push((n - 2, n - 1, 2.1)); // Profit (weight < 0)

        let graph = build_graph(&mut edges, n);
        let solver = SPFASolver;

        let cycle_option = solver.find_profitable_cycle(&graph, n - 1, n).unwrap();

        assert!(
            cycle_option.is_some(),
            "SPFA failed to find the guaranteed profitable cycle."
        );

        let cycle = cycle_option.unwrap();

        assert!(
            cycle.log_rate_sum < 0.0,
            "The detected cycle must have a negative weight sum."
        );
    }

    #[test]
    fn spfa_chain_with_multiple_negative_cycles() {
        let mut edges: Vec<Edge> = vec![
            // Cycle 1: Profitable (NEGATIVE CYCLE) - Product: 0.5 * 2.1 = 1.05
            (0, 1, 0.5), // Loss component
            (1, 0, 2.1), // Profit component (rate > 1.0)
            (1, 2, 1.0), // Neutral connection
            // Cycle 2: Non-profitable (LOSS/Positive Cycle)
            (2, 3, 0.8),
            (3, 2, 0.7),
        ];

        let graph = build_graph(&mut edges, 4);
        let solver = SPFASolver;

        let cycle_result = solver
            .find_profitable_cycle(&graph, 0, 4)
            .expect("SPFA execution returned an error.");

        assert!(
            cycle_result.is_some(),
            "SPFA failed to detect the guaranteed arbitrage cycle (0-1-0)."
        );

        let cycle = cycle_result.unwrap();

        assert_eq!(cycle.path.len(), 2, "The cycle length should be 2 edges.");

        assert!(
            cycle.log_rate_sum < 0.0,
            "The detected cycle must have a negative weight sum."
        );
    }
}
