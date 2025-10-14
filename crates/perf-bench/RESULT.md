## Performance Optimization: Array of Structs (AoS) vs. Structure of Arrays (SoA)

**Observed Results (100,000 Edges):**
| Layout | Elapsed Time |
|:---|:---|
| **AoS (Array of Structs)** | 119.594µs |
| **SoA (Structure of Arrays)** | 114.445µs |

The **Structure of Arrays (SoA)** layout performed marginally better in this benchmark. as a result of superior cache locality.

---

### Analysis of Layout Performance

The difference in execution time is primarily due to **cache locality**:

* **AoS (Array of Structs):** In AoS, data related to a single edge (e.g., `[from, to, rate]`) is stored contiguously. When processing only the edge weights (`rate`), the CPU loads the surrounding, **irrelevant** `from` and `to` indices into the high-speed **CPU cache** (poor spatial locality). This forces more cache lines to be loaded over time, slowing down the processor.
* **SoA (Structure of Arrays):** In the SoA approach, all 100,000 edge weights are grouped together in one continuous array (`rate_array`). When the CPU fetches data to process the weights, the cache lines are populated almost exclusively with the necessary rate values. This minimizes **cache misses** and keeps the processing pipelines fed efficiently, resulting in the slight time savings observed.