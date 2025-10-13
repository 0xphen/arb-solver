use std::hint::black_box;
use std::time::Instant;

use perf_bench::*;

fn main() {
    let soa_data: EdgeSOA = generate_benchmark_edges_aos().into();

    let start_time = Instant::now();
    let mut checksum: f64 = 0.0;

    // This loop only accesses the contiguous 'rate' vector, maximizing cache efficiency.
    for r in soa_data.rate {
        let new_rate = r * FEE_MULTIPLIER;
        checksum += new_rate;
    }

    let elapsed_time = start_time.elapsed();

    // 3. Print Results
    let final_checksum = black_box(checksum);

    println!("--- SoA Benchmark Results ({} Edges) ---", NUM_EDGES);
    println!("Checksum: {:.10}", final_checksum);
    println!("Elapsed Time: {:?}", elapsed_time);
}
