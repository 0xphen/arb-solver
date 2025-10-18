/// Represents a cycle in a weighted directed graph.
///
/// This struct stores both the sequence of edges forming the cycle
/// and metrics useful for analyzing the cycle, such as the product
/// of edge weights and the transformed sum (e.g., negative log for
/// detecting profitable/arbitrage cycles).
///
/// Fields:
/// - `path`: The sequence of edges forming the cycle.
/// - `rates`: Original weights of the edges along the cycle.
/// - `product_rate`: Cumulative product of all rates along the cycle (useful for profit calculation).
/// - `transformed_profit`: Sum of transformed weights (e.g., `-ln(rate)`); negative values may indicate profit.
#[derive(Debug, Clone)]
pub struct WeightedCycle {
    pub path: Vec<Edge>,
    pub rates: Vec<f64>,
    pub log_rate_sum: f64,
}

impl WeightedCycle {
    /// Returns the actual profit multiplier (∏ rate_i) for the cycle.
    ///
    /// Internally, the cycle stores the transformed sum: ∑ w_i where w_i = -ln(rate_i).
    /// The product is recovered via the inverse operation: rate_product = e^(-sum(w_i)).
    ///
    /// Example:
    /// ```text
    /// If original rates are [2.0, 3.0, 4.0] (∏=24.0),
    /// stored sum (log_rate_sum) = -ln(24.0) ≈ -3.178.
    /// product_rate = exp(-(-3.178)) = 24.0
    /// ```
    pub fn product_rate(&self) -> f64 {
        (-self.log_rate_sum).exp()
    }

    /// Returns true if the cycle is profitable (product_rate > 1.0).
    pub fn is_profitable(&self) -> bool {
        self.product_rate() > 1.0
    }
}

/// Type alias for a single edge list: (from, to, rate)
pub type Edge = (usize, usize, f64);
