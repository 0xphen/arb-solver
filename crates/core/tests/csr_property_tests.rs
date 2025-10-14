use arb_solver_core::csr::GraphCSR;
use proptest::prelude::*;
use proptest::strategy::Strategy;

const NUM_NODES_STRATEGY: std::ops::Range<usize> = 1usize..10;

fn csr_strategy() -> impl Strategy<Value = (usize, Vec<(usize, usize, f64)>)> {
    NUM_NODES_STRATEGY.prop_flat_map(|num_nodes| {
        let edge_generator = (0usize..num_nodes, 0usize..num_nodes, 0.01f64..10.0);
        let edges_generator = prop::collection::vec(edge_generator, 0..50);

        (proptest::strategy::Just(num_nodes), edges_generator)
    })
}

proptest! {
    /// Property: node_pointers should be monotonic
    #[test]
    fn node_pointers_monotonic(
        (num_nodes, mut edges) in csr_strategy()
    ) {
        let csr = GraphCSR::from_edges(num_nodes, &mut edges, 5);
        for i in 0..csr.num_nodes {
            prop_assert!(csr.node_pointers[i] <= csr.node_pointers[i + 1]);
        }
    }

     /// Property: edge_targets and edge_weights length consistency
    #[test]
    fn edge_arrays_length_consistent((num_nodes, mut edges) in csr_strategy()) {
        let csr = GraphCSR::from_edges(num_nodes, &mut edges, 5);
        prop_assert_eq!(csr.edge_targets.len(), csr.edge_weights.len());
        prop_assert_eq!(csr.edge_targets.len(), csr.node_pointers[csr.num_nodes]); // In CSR, the last node pointer equals the total number of edges.
    }

     /// Property: all edges are included (by count)
    #[test]
    fn all_edges_included((num_nodes, mut edges) in csr_strategy()) {
      let size = edges.len();
        let csr = GraphCSR::from_edges(num_nodes, &mut edges, 5);
        prop_assert_eq!(csr.edge_targets.len(), size);
    }

    /// Property : Verifies the logarithmic transformation (-ln(rate)) is applied correctly
    /// and that the weights are in the correct CSR order (sorted by source node).
    #[test]
    fn edge_weights_transformed_correctly(
        (num_nodes, mut edges) in csr_strategy()
    ) {
        let csr = GraphCSR::from_edges(num_nodes, &mut edges, 5);

        let mut sorted_edges = edges;
        // The CSR constructor internally sorts by source node
        sorted_edges.sort_by_key(|e| e.0);

        let expected_weights: Vec<f64> = sorted_edges.iter().map(|&(_, _, r)| -r.ln()).collect();

        // Compares the final CSR weights to the correctly transformed and sorted input weights
        prop_assert_eq!(csr.edge_weights, expected_weights);
    }

    /// Property: nodes with no outgoing edges have node_pointers[i] == node_pointers[i+1]
    ///
    #[test]
    fn nodes_without_edges( (num_nodes, mut edges) in csr_strategy()) {
        let csr = GraphCSR::from_edges(num_nodes, &mut edges, 5);

        let mut has_edges = vec![false; num_nodes];
        for &(from, _, _) in &edges {
            if from < num_nodes { has_edges[from] = true; }
        }

        for (i, _item) in has_edges.iter().enumerate().take(num_nodes) {
            if !has_edges[i] {
                prop_assert_eq!(csr.node_pointers[i], csr.node_pointers[i+1]);
            }
        }
    }
}
