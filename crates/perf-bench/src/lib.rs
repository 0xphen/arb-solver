// ----------------------------
// Task 2: Benchmark Layouts
// ----------------------------

/// Array of Structs (AoS) - Individual edge data is contiguous.
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub rate: f64,
}

pub type EdgeAOS = Vec<Edge>;

/// Struct of Arrays (SoA) - All fields of the same type are contiguous.
pub struct EdgeSOA {
    pub from: Vec<usize>,
    pub to: Vec<usize>,
    pub rate: Vec<f64>,
}

impl From<EdgeAOS> for EdgeSOA {
    fn from(aos: EdgeAOS) -> Self {
        let mut from = Vec::with_capacity(aos.len());
        let mut to = Vec::with_capacity(aos.len());
        let mut rate = Vec::with_capacity(aos.len());
        for edge in aos {
            from.push(edge.from);
            to.push(edge.to);
            rate.push(edge.rate);
        }
        EdgeSOA { from, to, rate }
    }
}

impl From<EdgeSOA> for EdgeAOS {
    fn from(soa: EdgeSOA) -> Self {
        soa.from
            .into_iter()
            .zip(soa.to)
            .zip(soa.rate)
            .map(|((from, to), rate)| Edge { from, to, rate })
            .collect()
    }
}

pub const NUM_EDGES: usize = 100_000;
pub const FEE_MULTIPLIER: f64 = 0.997; // 30 basis points fee (1 - 0.0030)

/// Generates a vector of edges in the Array of Structs (AoS) format.
///
/// The rate calculation is slightly varied to ensure the compiler cannot
/// optimize away the sum operation during benchmarking
pub fn generate_benchmark_edges_aos() -> EdgeAOS {
    (0..NUM_EDGES)
        .map(|i| Edge {
            from: i,
            to: i + 1,
            // Rate is > 1.0 and varied slightly by index for realism/compiler avoidance
            rate: 1.0001 + (i as f64) * 1e-12,
        })
        .collect()
}
