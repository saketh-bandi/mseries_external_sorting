External Sorting Engine 

External Sorting Engine built from scratch in Rust, designed to optimize disk I/O throughput and parallel computing on Apple M5 architecture. 

Operates on pure binary streams of fixed-size 64-bit unsigned integers, exactly 8 bytes per number on disk. 
Memory Ceiling: Caps active RAM utilization using a strict runtime budget.
Page-Aligned Vectorized I/O: Bypasses standard tiny-byte streams by wrapping disk interactions in block sizes scaled to multiples of the 16 KB virtual memory page.
Compute Awareness: Employs a parallel sort via Rayon, engineered to isolate workloads on high-throughput performance cores and avoid the straggler effects of hybrid efficiency cores.

Phase 1: Chunking & Parallel In-Place Sort
The input file is streamed sequentially in 16 KB blocks into a pre-allocated RAM buffer until the strict `MEMORY_LIMIT_BYTES` is met. The engine invokes Rayon to segment the chunk across a hardware-bounded thread pool (`THREAD_COUNT`), performs a parallel unstable sort in-place, and flushes the data to a deterministically numbered run file (`run_00000.bin`).

Phase 2: Streaming K-Way Merge
All temporary run files are opened simultaneously, each backed by an isolated 16 KB page-aligned pre-fetch buffer. The engine streams elements on-demand through a binary Min-Heap priority queue to ensure the lowest element across all active runs is popped in O(NlogK) time and written to the final output file.


## Structure

```text
external-sorter/
├── data_utils.py          # Python data suite (high-entropy generator & verifier)
├── Cargo.toml             # Workspace definition
├── src/
│   ├── main.rs            # End-to-end CLI driver interface
│   ├── lib.rs             # Library root exposing reusable phase modules
│   ├── config.rs          # Central tuning parameter constraints
│   ├── phase1.rs          # Bounded chunk splitting & parallel sorting logic
│   ├── phase2.rs          # Streaming Min-Heap K-way merging engine
│   └── bin/
│       └── benchmark.rs   # Comprehensive matrix sweeping benchmark harness
