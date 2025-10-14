## 🚀 Overview

The solver finds **profitable cycles** (negative cycles in log-space) using the **Shortest Path Faster Algorithm (SPFA)** — an optimized variant of Bellman-Ford.

### Why SPFA?

- SPFA relaxes edges **only for nodes that can potentially improve a shortest path**, skipping redundant computations.
- Much faster than standard Bellman–Ford on **sparse or semi-sparse graphs**.
- Works on **additive weights**, so multiplicative rates (e.g., exchange rates) must be transformed as:

\[
w = -\ln(r)
\]

This converts the **multiplicative product of rates** into a **sum of edge weights**.  
A **profitable cycle** (∏ rᵢ > 1) becomes a **negative cycle** in log-space (∑ -ln(rᵢ) < 0).

---

## Core Components

### 1. Graph Representation (CSR)

The graph is stored in **Compressed Sparse Row (CSR)** format:

| Field | Description |
|-------|-------------|
| `node_pointers` | Index ranges for each node’s outgoing edges |
| `edge_targets` | Destination nodes of each edge |
| `edge_weights` | Transformed weights `-ln(rate)` |
| `edge_source_by_index` | Reverse mapping for O(1) lookup of an edge’s source node |

**Advantages:**

- Fast iteration over outgoing edges.
- Better cache locality.
- **O(1) edge-to-source lookup** for cycle reconstruction.
- Supports **dynamic batch updates** with amortized rebuild cost.

> The `edge_source_by_index` enables quick tracing during cycle reconstruction. New edges are buffered and rebuilt only when the batch exceeds a threshold (`rebuild_limit`).

---

### 2. Arbitrage Detection via SPFA

SPFA iterates over the CSR graph, relaxing edges and tracking predecessor edges (`pred_edge_idx`) for each node.  

**Algorithmic Steps:**

1. **Initialization:**  
   All nodes start with `distance = 0` to detect cycles even in disconnected components.

2. **Relaxation:**  
   Propagate relaxations through a queue.

3. **Cycle Detection:**  
   If a node’s relaxation count ≥ `hop_cap` (typically |V|), SPFA detects a negative cycle and reconstructs it.

**Output:**  

- `path`: Sequence of `(u, v, rate)` edges in the cycle  
- `rates`: Original rates of edges in the cycle  
- `log_rate_sum`: Sum of transformed weights along the cycle

---

### 3. Cycle Reconstruction

Reconstruction is done in two stages:

1. **Walk Backward:**  
   Walk back |V| steps through predecessors to ensure entry into the cycle.

2. **Loop Reconstruction:**  
   Trace edges until returning to the same node, forming a complete cycle.

> Guarantees the recovered path represents the actual negative cycle, not just a downstream node.

---

### 4. Asynchronous Pipeline

`arb-solver` supports a **robust async pipeline** for continuous monitoring:

| Component | Role | Mechanism |
|-----------|------|-----------|
| **Producer** (`SimStreamer`, `CsvStreamer`) | Ingests or generates edge batches | Uses bounded MPSC channels for backpressure |
| **Writer** | Applies updates to the CSR graph | Implements a **two-phase lock** on `Arc<RwLock<GraphCSR>>` |
| **Searcher** | Detects arbitrage | Acquires a snapshot of the graph (`O(1)` clone) and runs SPFA asynchronously |

**Two-Phase Locking Strategy:**

1. **Phase 1 (Extract):** Briefly lock to extract pending updates.  
2. **Unlocked Work:** Sorting and deduplication occur without holding the lock, minimizing reader blocking.  
3. **Phase 2 (Commit):** Briefly lock to commit rebuilt CSR arrays.

This ensures **continuous reading** with minimal blocking while handling dynamic updates efficiently.

---

### 5. Key Features

- Optimized SPFA for **sparse and semi-sparse graphs**
- **CSR layout** for fast traversal and low memory overhead
- **Negative cycle reconstruction** with O(1) edge-to-source lookup
- **Dynamic updates with batching**, amortizing rebuild costs
- Fully **async-compatible** pipeline with backpressure
---
