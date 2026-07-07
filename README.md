External Sorting Engine 

A hardware-aware, zero-copy External Sorting Engine built from scratch in Rust, designed to optimize disk I/O throughput and parallel compute density on modern Apple Silicon architectures. 
This engine replaces high-level async abstractions with a pure, synchronous binary data pipeline, providing 100% architectural transparency and granular telemetry tuning.

* Zero-Parsing Binary Contract: Operates exclusively on pure binary streams of fixed-size 64-bit unsigned integers (`u64`), exactly 8 bytes per number on disk. No text parsing, no CSV overhead, and zero serialization layers (`serde`).
* Memory Ceiling Enforcement: Caps active RAM utilization using a strict runtime budget. Memory allocations are instantiated **exactly once** at startup and reused via logical slice clearing (`vec.clear()`) to eliminate kernel-level `malloc`/`mmap` allocation thrashing.
* Page-Aligned Vectorized I/O: Bypasses standard tiny-byte streams by wrapping disk interactions in custom `BufReader` and `BufWriter` block sizes scaled to multiples of the native **16 KB Apple Silicon virtual memory page**.
* Asymmetric Compute Awareness: Employs a parallel work-stealing sort via Rayon, engineered to be strictly tunable to isolate workloads on high-throughput Performance (P) cores and avoid the straggler effects of hybrid Efficiency (E) cores.

**Phase 1:** Chunking & Parallel In-Place Sort
The input file is streamed sequentially in 16 KB blocks into a pre-allocated RAM buffer until the strict `MEMORY_LIMIT_BYTES` is met. The engine invokes Rayon to segment the chunk across a hardware-bounded thread pool (`THREAD_COUNT`), performs a parallel unstable sort in-place, and flushes the data to a deterministically numbered run file (`run_00000.bin`).

**Phase 2:** Streaming K-Way Merge
All temporary run files are opened simultaneously, each backed by an isolated 16 KB page-aligned pre-fetch buffer. The engine streams elements on-demand through a binary **Min-Heap** priority queue to ensure the lowest element across all active runs is popped in $O(\log K)$ time and written to the final output file, ensuring the full dataset never materializes in memory.


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
