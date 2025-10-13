use std::hint::black_box;
use std::time::Instant;

use perf_bench::*;

fn main() {
    let aos_data: EdgeAOS = generate_benchmark_edges_aos();

    let start_time = Instant::now();
    let mut checksum: f64 = 0.0;

    // The processor must jump in memory for each field (from, to, rate).
    for edge in aos_data {
        let new_rate = edge.rate * FEE_MULTIPLIER;
        checksum += new_rate;
    }

    let elapsed_time = start_time.elapsed();

    let final_checksum = black_box(checksum);

    println!("--- AoS Benchmark Results ({} Edges) ---", NUM_EDGES);
    println!("Checksum: {:.10}", final_checksum);
    println!("Elapsed Time: {:?}", elapsed_time);
}
