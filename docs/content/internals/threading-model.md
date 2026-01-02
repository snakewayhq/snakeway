# Threading Model

Snakeway utilizes a highly efficient, multi-worker threading model inherited from the Pingora framework. This model is
designed to maximize CPU utilization and handle large volumes of concurrent traffic with minimal context switching.

### Multi-Worker Architecture

When Snakeway starts, it spawns a pool of worker threads. The number of workers can be explicitly configured in
`snakeway.toml` or left for Pingora to decide based on the available CPU cores.

- **Load Distribution**: Incoming connections are distributed across these worker threads using an efficient OS-level
  listener.
- **Isolation**: Each worker operates independently, handling its own set of active connections. This ensures that a
  single blocked worker (due to a heavy computation or I/O) does not stop the entire server from processing traffic.

### Asynchronous Runtime

Within each worker thread, Snakeway uses an asynchronous runtime (built on **Tokio**). This allows a single thread to
manage thousands of concurrent requests by non-blockingly switching between them during I/O operations (like waiting for
an upstream response).

### Thread Safety: `Send + Sync`

Because requests can be processed across different worker threads, Snakeway's core components and all Devices must be
thread-safe.

- **`Send`**: Components must be capable of being moved between threads.
- **`Sync`**: Components must be safe to be accessed from multiple threads simultaneously.

In Rust terms, this means all Device implementations must implement the `Send + Sync` traits. This is a key requirement
for ensuring that the global `DeviceRegistry` can be shared safely across all workers.

### The Hot Path

The "hot path"—the code that executes for every request—is carefully optimized to minimize locking and contention. We
use high-performance concurrency primitives like `Arc` (Atomic Reference Counting) and `ArcSwap` to ensure that data
access is as fast as possible, even in highly multi-threaded environments.
