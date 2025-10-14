# 🧮 arb-solver

`arb-solver` is a **Rust project** for efficiently detecting **profitable arbitrage cycles** in directed weighted graphs. It uses an **optimized Bellman–Ford variant (SPFA)** for fast negative cycle detection, essential for finding arbitrage opportunities where the product of edge weights (rates) exceeds one.

The project also employs a **Compressed Sparse Row (CSR)** graph representation, ensuring memory and computational efficiency.

---

## 📦 Project Structure

`arb-solver` is a **multi-crate workspace**, designed for modularity and performance analysis.

| Crate Name                 | Description                                                                                                                |
| -------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| **core**                   | Implements the main logic, including the CSR graph structure and the SPFA arbitrage detection algorithm.                   |
| **common**                 | Provides shared utilities used across other crates.                                                                        |
| **executor**               | Handles project execution, input parsing (CSV or simulation), and coordinates the core logic via an **async pipeline**.    |
| **perf-bench**             | Dedicated crate for benchmarking different data layouts, such as **Array-of-Structs (AoS)** vs **Struct-of-Arrays (SoA)**. |
| **[root-level workspace]** | Top-level workspace that aggregates and manages the other crates.                                                          |

---

## 🔨 Build and Usage

### Clone & Build the Repository

```bash
git clone git@github.com:0xphen/arb-solver.git
cd arb-solver

cargo build
```

---

### 🧪 Running Tests

Execute **unit tests** across all crates:

```bash
cargo test
```

---

### 📊 Running Benchmarks

Compare performance of different data layouts with the `perf-bench` crate:

| Layout                     | Command                                             |
| -------------------------- | --------------------------------------------------- |
| **Struct-of-Arrays (SoA)** | `cargo run --release --bin bench_soa -p perf-bench` |
| **Array-of-Structs (AoS)** | `cargo run --release --bin bench_aos -p perf-bench` |

---

### 🚀 Running the Executor

The `executor` crate can be run in **two main modes**:

#### 1️⃣ Simulation Mode

Runs the executor using a **randomly generated internal graph** :

```bash
cargo run --release -p executor -- sim
```

#### 2️⃣ CSV Input Mode

Runs the executor using a graph provided via a parsed **local CSV file**.

The CSV file must be formatted as:

```
from,to,rate
```

Example:

```csv
0,1,0.92
1,2,150.5
2,0,0.0074
```

Run the executor:

```bash
cargo run --release -p executor -- csv <path_to_csv_file>
```
 ⚠️ Important: The [`Config.toml`](crates/executor/Config.toml) file configures various aspects of the executor system, including the searcher, writer, simulator, producer, and executor. It controls batch sizes, processing intervals, backpressure behavior, and simulation parameters, making it the central configuration for the system in all modes—including CSV input and simulation.
