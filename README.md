# Scribe

Scribe is a high-performance, crash-resilient, and cross-platform native mobile logging engine written in Rust. 

Unlike traditional logging frameworks that suffer from JVM GC spikes, UI-thread I/O blockages, or crash-induced data loss, Scribe is designed from the ground up to combine Rust's memory safety with OS-level memory-mapped I/O (`mmap`). It provides a self-contained, C-ABI compliant native binary (`.so`, `.a`, `.xcframework`) that can be consumed directly by Android, iOS, and other cross-platform runtimes (like KMP, Flutter, or React Native).

---

## 🚀 Key Features

*   **Lock-Free Double `mmap` Buffers**: Uses atomic pointer swapping and an active-writer tracking algorithm to swap buffers instantly when full. Zero lock-contention, zero I/O blockages on the UI thread.
*   **High Crash Resiliency**: Leveraging OS-level `mmap` dirty page flushing, logs written to memory are preserved even if the application process crashes or is killed by the OS.
*   **Robust Half-Write Protection**: Every log frame is sealed with a Magic Header and CRC32 verification. Scribe's decoder automatically detects and discards corrupted trailing logs from incomplete writes during a crash.
*   **Multi-Process Safety**: Android apps often span multiple processes (e.g., `:push`, `:service`). Scribe isolates active `.mmap` buffers by binding the filename to the OS process identifier (PID), eliminating file-lock conflicts.
*   **Optimized Compression & Encryption**: Compresses raw log chunks using `zstd` (with support for custom dictionary training for log payloads) and encrypts them via software-optimized `ChaCha20-Poly1305` before disk serialization.
*   **Platform-Agnostic Core**: Zero callbacks to Swift/Kotlin runtimes. Native console output (like Android's `__android_log_write` and iOS's `os_log`) is handled directly in Rust using target-conditional compilation.
*   **Timber-Compatible API**: Tree-based plugin architecture with automatic tag detection from call stack, enabling familiar API for Android developers migrating from Timber.

---

## 📐 Architecture & Data Flow

```text
[ JVM / Swift Thread Pool ] 
             |  
             | Scribe.d(tag, msg)  <-- Low-cost, raw FFI (Zero-Copy)
             v
+--------------------------------------------------------+
| Scribe Native Engine (Rust Core)                       |
|                                                        |
|   +------------------------------------------------+   |
|   | DoubleBufferManager (Lock-free Swap)           |   |
|   |                                                |   |
|   |  [Active mmap Buffer A]  [Pending Buffer B]    |   |
|   +-------------+---------------------+------------+   |
|                 |                     |                |
|                 v                     |                |
|         (OS Auto Sync)                v                |
|                 |            [Zstd Compressor]         |
|                 |                     |                |
|                 |            [ChaCha20 Encryptor]      |
|                 v                     v                |
|         [Active .mmap file]   [Encrypted Log File]     |
+-----------------|---------------------|----------------+
                  v                     v
            =================================
                     [ Physical Disk ]
```

---

## 🛠️ Deep Dive: The Lock-Free Active-Writers Swapping

To avoid blocking UI threads when swapping full buffers, Scribe maintains an atomic counter of active writing threads. When a swap is triggered:

1. The active buffer index is swapped atomically (`active_index.fetch_xor(1)`). Incoming logs are instantly diverted to the fresh buffer.
2. The swapper thread spin-waits for existing writers in the old buffer to finish (`active_writers == 0`).
3. The old buffer is safely detached and handed over to the background Rust worker thread for compression and encryption, preventing any lock contention.

---

## 📦 Project Structure

```text
scribe/
├── Cargo.toml
├── src/
│   ├── lib.rs            # C-ABI & FFI exposed interface
│   ├── core.rs           # Lock-free atomic concurrency queues
│   ├── storage.rs        # Memory-mapped I/O (mmap) & DoubleBufferManager
│   ├── compress.rs       # Stream compression using zstd
│   ├── encrypt.rs        # AEAD Encryption using ChaCha20-Poly1305
│   └── platform/         # Platform-specific OS adapters (completely in Rust)
│       ├── android.rs    # Logcat integration via __android_log_write
│       └── ios.rs        # iOS Console output via os_log
```

---

## 💻 Quick Start

### FFI (C/C++/Swift/Kotlin)

```c
// Initialize
scribe_init("/path/to/logs");

// Plant a DebugTree for development (outputs to console)
scribe_plant_debug_tree(2);  // 2 = Info level minimum

// Write logs
scribe_write(2, "MyTag", "Application started");  // 2 = Info level

// Flush to disk
scribe_flush();

// Cleanup
scribe_destroy();
```

### Rust API

```rust
use scribe::{scribe_i, scribe_d, scribe_e, plant, DebugTree};

// Plant a DebugTree for console output
plant(Box::new(DebugTree::new()));

// Log with automatic tag detection
scribe_i!("Server started on port {}", 8080);
scribe_d!("Processing request from {}", client_addr);
scribe_e!("Failed to connect: {}", error);

// Or use explicit tags
scribe_tag_i!("network", "Connection established");
scribe_tag_e!("database", "Query failed: {}", sql);
```

### Timber-Compatible API

Scribe provides a familiar API for developers migrating from Timber:

```rust
use scribe::{plant, DebugTree, tag};

// Development setup
#[cfg(debug_assertions)]
plant(Box::new(DebugTree::new()));

// Automatic tag from call stack
scribe_d!("This is a debug message");

// Temporary tag with chaining
tag("MyTag").i("One-time custom tag");

// Thread-local tag
tag("Network").plant();
scribe_i!("Uses Network tag");
scribe_d!("Still uses Network tag");
```

---

## 📚 Documentation

- **[API Reference](API-REFERENCE.md)** - Complete FFI and Rust API documentation
- **[Timber Comparison](TIMBER-COMPARISON.md)** - Feature comparison with Timber
- **[Test Coverage](TEST-COVERAGE.md)** - Testing statistics and coverage report
- **[Project Status](FINAL-STATUS.md)** - Current implementation status and roadmap

---

## ⚠️ Robust Error & Resource Fallbacks

*   **OutOfSpace (Disk-Full)**: If allocating disk space for `.mmap` buffers fails (`ENOSPC`), Scribe automatically falls back to an in-memory lock-free Ring Buffer. This gracefully trades crash-resiliency to prevent application aborts.
*   **Thread Safety**: All storage managers implement Rust's `Send + Sync`, ensuring absolute compile-time data-race safety.

---

## 📄 License

Scribe is available under the MIT License.
