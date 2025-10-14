use common::error::Error;
use common::types::Edge;

pub enum AddEdgeResult {
    Success,
    RebuildNeeded(Vec<Edge>),
}

/// Graph in Compressed Sparse Row (CSR) format for fast graph traversal.
///
/// CSR format stores outgoing edges of each node contiguously in memory:
/// - `node_pointers[u]..node_pointers[u+1]` → edges from node `u`
/// - `edge_targets[i]` -> target node of edge `i`
/// - `edge_weights[i]` -> weight of edge `i`
/// - `edge_source_by_index[i]` -> source node of edge `i`
///
/// This structure allows O(1) edge lookup per node and compact memory usage.
/// Pending updates are batched and applied on rebuild to maintain efficiency.
#[derive(Debug, Clone)]
pub struct GraphCSR {
    pub num_nodes: usize,
    pub node_pointers: Vec<usize>,
    pub edge_targets: Vec<usize>,
    pub edge_weights: Vec<f64>,
    pub edge_source_by_index: Vec<usize>,
    pub rebuild_limit: usize,
    pub pending_updates: Vec<Edge>,
}

impl GraphCSR {
    /// Creates a new CSR graph from a list of edges `(src, dst, rate)`.
    ///
    /// Each edge weight is transformed as `-ln(rate)` for the SPFA algorithm,
    /// which works with negative weights.
    ///
    /// Edges are stored sorted by source node to ensure contiguous blocks
    /// for each node and fast traversal.
    ///
    /// # Arguments
    /// - `num_nodes`: total number of nodes (graph indices: 0..num_nodes-1)
    /// - `edges`: slice of `(src, dst, rate)` tuples
    /// - `rebuild_limit`: number of pending updates before triggering rebuild
    ///
    /// # Returns
    /// A fully initialized `GraphCSR` instance.
    pub fn from_edges(num_nodes: usize, edges: &mut [Edge], rebuild_limit: usize) -> Self {
        edges.sort_by_key(|(src, _, _)| *src);

        let (node_pointers, edge_targets, edge_weights, edge_source_by_index) =
            Self::build_csr_from_edges(num_nodes, edges);

        Self {
            num_nodes,
            node_pointers,
            edge_targets,
            edge_weights,
            edge_source_by_index,
            rebuild_limit,
            pending_updates: Vec::new(),
        }
    }

    /// Internal helper to construct all necessary arrays for the Compressed Sparse Row (CSR) format.
    ///
    /// This function uses the efficient two-pass counting technique to build the CSR index
    /// and applies a negative-log transformation to each exchange rate, preparing the graph
    /// for shortest-path or arbitrage detection algorithms.
    ///
    /// # Arguments
    /// * `num_nodes`: The total number of vertices (|V|).
    /// * `edges`: A slice of raw edge tuples `(u, v, rate)`.
    ///
    /// # Returns
    /// A tuple containing the four core arrays:
    /// 1. `node_pointers`: Stores the starting index of each node’s outgoing edges
    ///    in the flattened edge arrays (size |V| + 1).
    /// 2. `edge_targets`: Stores the destination node `v` for each edge.
    /// 3. `edge_weights`: Stores the transformed edge weights `w = -ln(rate)` for use by the SPFA solver.
    /// 4. `edge_source_by_index`: Maps each edge index back to its source node `u`.
    ///
    ///    This array enables **O(1) reverse lookups** from any edge index to its originating source node,
    ///    which is essential during path or cycle reconstruction (e.g., tracing a negative cycle)
    ///    without needing a costly binary search over `node_pointers`.  
    ///    Although multiple edges may share the same source (producing duplicates),
    ///    it ensures fast and direct edge-to-source mapping for efficient graph traversal and debugging.
    fn build_csr_from_edges(
        num_nodes: usize,
        edges: &[Edge],
    ) -> (Vec<usize>, Vec<usize>, Vec<f64>, Vec<usize>) {
        let m = edges.len();
        let mut node_pointers = vec![0; num_nodes + 1];

        for &(u, _, _) in edges {
            node_pointers[u + 1] += 1;
        }

        for i in 1..=num_nodes {
            node_pointers[i] += node_pointers[i - 1];
        }

        let mut edge_targets = vec![0; m];
        let mut edge_weights = vec![0.0; m];
        let mut edge_source_by_index = vec![0; m];

        let mut cursor = node_pointers.clone();

        for &(u, v, rate) in edges {
            let pos = cursor[u]; // Get the next available position for node 'u'
            edge_weights[pos] = -rate.ln();
            edge_targets[pos] = v;
            edge_source_by_index[pos] = u;

            // Advance the cursor for node 'u' to point to the next free slot.
            cursor[u] += 1;
        }

        (
            node_pointers,
            edge_targets,
            edge_weights,
            edge_source_by_index,
        )
    }

    /// O(1) lookup for the source node of a given edge index.
    ///
    /// # Errors
    /// Returns `Error::InvalidGraph` if `edge_idx` is out of bounds.
    pub fn get_edge_source_node(&self, edge_idx: usize) -> Result<usize, Error> {
        self.edge_source_by_index
            .get(edge_idx)
            .copied()
            .ok_or(Error::InvalidGraph)
    }

    /// Adds multiple edges to the graph in a single batch update.
    ///
    /// Instead of immediately rebuilding the CSR structure on every edge insertion,
    /// new edges are first accumulated in `pending_updates`. Once the number of
    /// pending edges reaches the configured `rebuild_limit`, the graph is rebuilt
    /// in one pass for efficiency.
    ///
    /// # Why batching?
    /// Building the CSR structure (`rebuild()`) requires sorting and recomputing
    /// indexing arrays, which is **O(E log E)** and can be expensive if done
    /// after every single edge insertion. Batching allows many updates to be
    /// applied together, amortizing this cost and keeping rebuilds efficient.
    ///
    /// # Trade-off
    /// - **Pros:** Fewer rebuilds, better performance for frequent insertions.
    /// - **Cons:** Newly added edges are not reflected in the CSR graph
    ///   until the next rebuild, so there’s a slight delay in graph consistency.
    ///
    /// This design is ideal when edges are added in bursts and immediate consistency is not required.
    #[allow(dead_code)]
    fn add_edges(&mut self, edges: Vec<Edge>) {
        let size = edges.len();
        self.pending_updates.extend(edges);
        println!("{} edges added to graph pending buffer", size);

        if self.pending_updates.len() >= self.rebuild_limit {
            println!("Graph rebuild limit reached. Re-building CSR");
            self.rebuild();
        }
    }

    /// Attempts to add a batch of new edges to the internal buffer.
    /// If the buffer limit is reached, it atomically extracts (via O(1) swap)
    /// the full accumulated edge list and signals that a rebuild is required.
    pub fn add_edges_and_extract_data(&mut self, edges: Vec<Edge>) -> AddEdgeResult {
        self.pending_updates.extend(edges);

        if self.pending_updates.len() >= self.rebuild_limit {
            let edges_to_rebuild = std::mem::take(&mut self.pending_updates);

            return AddEdgeResult::RebuildNeeded(edges_to_rebuild);
        }
        AddEdgeResult::Success
    }

    /// Initiates a full, in-place CSR rebuild using the *pending updates* buffer.
    ///
    /// **WARNING:** This is an internal convenience function. In the two-phase
    /// concurrency model, the external Writer should call `rebuild_with_edges`
    /// instead to prevent excessive lock times.
    #[allow(dead_code)]
    fn rebuild(&mut self) {
        let new_edges = std::mem::take(&mut self.pending_updates);

        self.rebuild_with_edges(new_edges)
    }

    /// Fully rebuilds the CSR structure by incorporating a new set of edges.
    ///
    /// This is the **public interface** for the Writer's Phase 2 commit.
    /// Steps involve extracting existing CSR edges, merging them with `new_edges`,
    /// sorting/deduplicating, recomputing the node count, and committing the
    /// new CSR arrays. The cost is high (O(E log E)).
    pub fn rebuild_with_edges(&mut self, new_edges: Vec<Edge>) {
        let mut edges: Vec<(usize, usize, f64)> =
            Vec::with_capacity(self.edge_targets.len() + new_edges.len());

        // Extract existing edges
        for src in 0..self.num_nodes {
            let start = self.node_pointers[src];
            let end = self.node_pointers[src + 1];
            for j in start..end {
                let dst = self.edge_targets[j];
                let rate = (-self.edge_weights[j]).exp();
                edges.push((src, dst, rate));
            }
        }

        let mut new_edges = new_edges;
        edges.append(&mut new_edges);

        //Sort and deduplicate by (src, dst)
        edges.sort_by_key(|&(src, dst, _)| (src, dst));
        edges.reverse();
        edges.dedup_by_key(|(src, dst, _)| (*src, *dst));

        let num_nodes = edges
            .iter()
            .flat_map(|&(u, v, _)| [u, v])
            .max()
            .map_or(0, |max_id| max_id + 1);

        let (node_pointers, edge_targets, edge_weights, edge_source_by_index) =
            Self::build_csr_from_edges(num_nodes, &edges);

        self.num_nodes = num_nodes;
        self.node_pointers = node_pointers;
        self.edge_targets = edge_targets;
        self.edge_weights = edge_weights;
        self.edge_source_by_index = edge_source_by_index;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64;

    #[test]
    fn from_edges_creates_correct_csr_for_small_graph() {
        let mut edges = vec![(2, 1, 0.99), (0, 2, 1.1), (0, 1, 0.9)]; // Un-sorted edges
        let csr = GraphCSR::from_edges(3, &mut edges, 3);

        assert_eq!(csr.node_pointers, vec![0, 2, 2, 3]);
        assert_eq!(csr.edge_targets, vec![2, 1, 1]);

        let expected_weights: Vec<f64> = edges.iter().map(|&(_, _, r)| -r.ln()).collect();
        assert_eq!(csr.edge_weights, expected_weights);
        assert_eq!(csr.num_nodes, 3);
        assert!(csr.pending_updates.is_empty());
        assert_eq!(csr.rebuild_limit, 3);
    }

    #[test]
    fn node_with_no_outgoing_edges() {
        let mut edges = vec![(0, 2, 1.0)];
        let csr = GraphCSR::from_edges(3, &mut edges, 3);

        assert_eq!(csr.node_pointers, vec![0, 1, 1, 1]);
        assert_eq!(csr.edge_targets, vec![2]);
        assert_eq!(csr.edge_weights, vec![-1.0f64.ln()]);
    }

    #[test]
    fn single_node_graph() {
        let csr = GraphCSR::from_edges(1, &mut [], 1);

        assert_eq!(csr.num_nodes, 1);
        assert_eq!(csr.node_pointers, vec![0, 0]);
        assert!(csr.edge_targets.is_empty());
    }

    #[test]
    fn empty_graph() {
        let csr = GraphCSR::from_edges(0, &mut [], 1);

        assert_eq!(csr.num_nodes, 0);
        assert_eq!(csr.node_pointers, vec![0]);
        assert!(csr.edge_targets.is_empty());
    }

    #[test]
    fn multiple_edges_from_same_node() {
        let mut edges = vec![(0, 1, 1.0), (0, 2, 2.0), (0, 3, 3.0)];
        let csr = GraphCSR::from_edges(4, &mut edges, 3);

        assert_eq!(csr.node_pointers, vec![0, 3, 3, 3, 3]);
        assert_eq!(csr.edge_targets, vec![1, 2, 3]);
    }

    #[test]
    fn edge_weight_transformation() {
        let mut edges = vec![(0, 1, 0.5), (1, 2, 2.0), (2, 0, 1.5)];
        let csr = GraphCSR::from_edges(3, &mut edges, 3);

        let expected_weights: Vec<f64> = edges.iter().map(|&(_, _, r)| -r.ln()).collect();
        assert_eq!(csr.edge_weights, expected_weights);
    }

    #[test]
    fn rebuild_merges_pending_updates_correctly() {
        let mut csr = GraphCSR::from_edges(3, &mut [(0, 1, 1.0), (1, 2, 1.5)], 2);

        csr.pending_updates = vec![(2, 0, 2.0)];
        csr.rebuild();

        assert_eq!(csr.edge_targets.len(), 3);
        assert_eq!(csr.edge_targets.iter().sum::<usize>(), 0 + 1 + 2);

        // The pending buffer must be empty after rebuild() runs.
        assert!(csr.pending_updates.is_empty());
    }

    #[test]
    fn rebuild_deduplicates_by_keeping_latest() {
        let mut csr = GraphCSR::from_edges(2, &mut [(0, 1, 1.0)], 2);
        csr.pending_updates = vec![(0, 1, 2.0)];
        csr.rebuild();

        assert_eq!(csr.edge_targets, vec![1]);
        assert_eq!(csr.edge_weights, vec![-2.0f64.ln()]);

        assert!(csr.pending_updates.is_empty());
    }

    #[test]
    fn rebuild_is_idempotent_when_empty() {
        let csr_original = GraphCSR::from_edges(2, &mut [(0, 1, 1.0)], 2);
        let mut csr = csr_original.clone();

        csr.rebuild();
        assert_eq!(csr.node_pointers, csr_original.node_pointers);
        assert_eq!(csr.edge_targets, csr_original.edge_targets);
        assert_eq!(csr.edge_weights, csr_original.edge_weights);
    }

    #[test]
    fn rebuild_on_empty_graph() {
        let mut csr = GraphCSR::from_edges(0, &mut [], 1);
        csr.pending_updates = vec![(0, 1, 1.0)];
        csr.rebuild();

        assert_eq!(csr.num_nodes, 2);
        assert_eq!(csr.edge_targets, vec![1]);
        assert_eq!(csr.node_pointers, vec![0, 1, 1]);
    }

    #[test]
    fn rebuild_handles_large_graphs() {
        let mut edges: Vec<_> = (0..1000).map(|i| (i, (i + 1) % 1000, 1.1)).collect();
        let mut csr = GraphCSR::from_edges(1000, &mut edges, 1000);

        csr.pending_updates = (0..1000).map(|i| (i, (i + 2) % 1000, 1.2)).collect();
        csr.rebuild();

        assert_eq!(csr.num_nodes, 1000);
        assert_eq!(csr.edge_targets.len(), 2000);
    }

    #[test]
    fn add_edges_does_not_trigger_rebuild_when_below_limit() {
        let mut edges = vec![(0, 1, 1.0)];
        let mut csr = GraphCSR::from_edges(2, &mut edges, 3);

        csr.add_edges(vec![(1, 0, 2.0)]);

        assert_eq!(csr.pending_updates.len(), 1);
        assert_eq!(csr.edge_targets.len(), 1); // CSR arrays should be unchanged
    }

    #[test]
    fn add_edges_triggers_rebuild_when_limit_exceeded() {
        let mut edges = vec![(0, 1, 1.0)];
        let mut csr = GraphCSR::from_edges(2, &mut edges, 1);

        csr.add_edges(vec![(1, 0, 2.0)]);

        assert!(csr.pending_updates.is_empty()); // Buffer cleared after internal rebuild
        assert_eq!(csr.edge_targets.len(), 2);
    }

    #[test]
    fn rebuild_with_edges_does_not_touch_pending_buffer() {
        let mut csr = GraphCSR::from_edges(2, &mut [(0, 1, 1.0)], 2);

        csr.pending_updates = vec![(1, 0, 0.5)];
        let pending_len_before = csr.pending_updates.len();

        let rebuild_data = vec![(0, 1, 2.0)];

        csr.rebuild_with_edges(rebuild_data);

        assert_eq!(csr.edge_weights.len(), 1);
        assert_eq!(csr.edge_weights[0], -2.0f64.ln());

        assert_eq!(csr.pending_updates.len(), pending_len_before);
        assert_eq!(csr.pending_updates, vec![(1, 0, 0.5)]);
    }

    #[test]
    fn extract_data_and_rebuild_leaves_buffer_empty() {
        let mut csr = GraphCSR::from_edges(2, &mut [(0, 1, 1.0)], 1);
        let updates = vec![(1, 0, 2.0)];

        let result = csr.add_edges_and_extract_data(updates);

        assert!(csr.pending_updates.is_empty());

        let extracted_edges = match result {
            AddEdgeResult::RebuildNeeded(edges) => edges,
            _ => panic!("Expected RebuildNeeded result"),
        };

        csr.rebuild_with_edges(extracted_edges);

        assert_eq!(csr.edge_targets.len(), 2);
    }
}
