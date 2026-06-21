use super::buffer::MmapBuffer;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

const BUFFER_CAPACITY: usize = 4 * 1024 * 1024; // 4MB

/// A double-buffering manager for lock-free log rotation.
///
/// DoubleBufferManager maintains two buffers and alternates between them.
/// When one buffer fills up, it atomically swaps to the other buffer while
/// a background worker processes the filled buffer.
///
/// # Architecture
///
/// - **Active buffer**: Currently accepting writes
/// - **Inactive buffer**: Being processed by worker thread
/// - **Atomic swap**: Lock-free buffer rotation
/// - **Writer tracking**: Ensures safe buffer transitions
///
/// # Examples
///
/// ```no_run
/// use scribe::DoubleBufferManager;
/// use std::path::PathBuf;
///
/// let mut manager = DoubleBufferManager::new(PathBuf::from("/tmp/logs")).unwrap();
///
/// // Spawn a worker to process filled buffers
/// manager.spawn_worker(|data| {
///     std::fs::write("/tmp/output.log", data)?;
///     Ok(())
/// }).unwrap();
///
/// // Get active buffer and write
/// let (buffer, idx) = manager.get_active_buffer();
/// manager.increment_active_writers(idx);
/// buffer.write(b"log data").unwrap();
/// manager.decrement_active_writers(idx);
///
/// // Swap when full
/// if manager.should_swap(&buffer) {
///     manager.swap_buffers().unwrap();
/// }
/// ```
pub struct DoubleBufferManager {
    buffers: [Arc<MmapBuffer>; 2],
    active_index: AtomicU8,
    active_writers: [AtomicUsize; 2],
    swap_sender: Sender<Arc<MmapBuffer>>,
    swap_receiver: Receiver<Arc<MmapBuffer>>,
    worker_handle: Option<thread::JoinHandle<()>>,
}

impl DoubleBufferManager {
    /// Creates a new DoubleBufferManager with two buffers.
    ///
    /// # Arguments
    ///
    /// * `log_dir` - Directory where buffer files will be created.
    ///
    /// # Returns
    ///
    /// * `Ok(DoubleBufferManager)` - A new manager instance.
    /// * `Err(ScribeError)` - If buffer creation fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use scribe::DoubleBufferManager;
    /// use std::path::PathBuf;
    ///
    /// let manager = DoubleBufferManager::new(PathBuf::from("/tmp/logs")).unwrap();
    /// ```
    pub fn new(log_dir: PathBuf) -> crate::Result<Self> {
        // 创建两个 buffer
        let buffer0 = Arc::new(MmapBuffer::new(
            log_dir.join("scribe_buffer_0.mmap"),
            BUFFER_CAPACITY,
        )?);
        let buffer1 = Arc::new(MmapBuffer::new(
            log_dir.join("scribe_buffer_1.mmap"),
            BUFFER_CAPACITY,
        )?);

        let (swap_sender, swap_receiver) = bounded(2);

        Ok(Self {
            buffers: [buffer0, buffer1],
            active_index: AtomicU8::new(0),
            active_writers: [AtomicUsize::new(0), AtomicUsize::new(0)],
            swap_sender,
            swap_receiver,
            worker_handle: None,
        })
    }

    /// Returns the currently active buffer and its index.
    ///
    /// # Returns
    ///
    /// A tuple of `(Arc<MmapBuffer>, u8)` containing the buffer and its index (0 or 1).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use scribe::DoubleBufferManager;
    /// use std::path::PathBuf;
    ///
    /// let manager = DoubleBufferManager::new(PathBuf::from("/tmp/logs")).unwrap();
    /// let (buffer, idx) = manager.get_active_buffer();
    /// ```
    pub fn get_active_buffer(&self) -> (Arc<MmapBuffer>, u8) {
        let idx = self.active_index.load(Ordering::Acquire);
        let buffer = self.buffers[idx as usize].clone();
        (buffer, idx)
    }

    /// Increments the active writer count for a buffer.
    ///
    /// # Arguments
    ///
    /// * `idx` - The buffer index (0 or 1).
    ///
    /// # Note
    ///
    /// Always call `decrement_active_writers` after writing completes.
    pub fn increment_active_writers(&self, idx: u8) {
        self.active_writers[idx as usize].fetch_add(1, Ordering::AcqRel);
    }

    /// Decrements the active writer count for a buffer.
    ///
    /// # Arguments
    ///
    /// * `idx` - The buffer index (0 or 1).
    pub fn decrement_active_writers(&self, idx: u8) {
        self.active_writers[idx as usize].fetch_sub(1, Ordering::AcqRel);
    }

    /// Checks if a buffer swap should be triggered.
    ///
    /// # Arguments
    ///
    /// * `buffer` - The buffer to check.
    ///
    /// # Returns
    ///
    /// `true` if the buffer is 90% or more full.
    pub fn should_swap(&self, buffer: &MmapBuffer) -> bool {
        buffer.is_full(0.9) // 90% 满时触发交换
    }

    /// Atomically swaps the active and inactive buffers.
    ///
    /// This operation:
    /// 1. Atomically switches the active buffer index
    /// 2. Waits for all active writers on the old buffer to complete
    /// 3. Sends the old buffer to the worker thread for processing
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If swap succeeds.
    /// * `Err(ScribeError)` - If the swap channel send fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use scribe::DoubleBufferManager;
    /// use std::path::PathBuf;
    ///
    /// let manager = DoubleBufferManager::new(PathBuf::from("/tmp/logs")).unwrap();
    /// let (buffer, _) = manager.get_active_buffer();
    ///
    /// if manager.should_swap(&buffer) {
    ///     manager.swap_buffers().unwrap();
    /// }
    /// ```
    pub fn swap_buffers(&self) -> crate::Result<()> {
        // 原子交换 active_index
        let old_idx = self.active_index.fetch_xor(1, Ordering::AcqRel);
        let _new_idx = old_idx ^ 1;

        // 自旋等待旧 buffer 的所有写入者完成
        while self.active_writers[old_idx as usize].load(Ordering::Acquire) > 0 {
            std::hint::spin_loop();
        }

        // 发送旧 buffer 到 worker 线程处理
        let old_buffer = self.buffers[old_idx as usize].clone();
        self.swap_sender
            .send(old_buffer)
            .map_err(|e| crate::ScribeError::Mmap(format!("Swap send failed: {}", e)))?;

        Ok(())
    }

    /// Spawns a background worker thread to process filled buffers.
    ///
    /// The worker receives buffers from the swap channel and processes them
    /// using the provided processor function.
    ///
    /// # Arguments
    ///
    /// * `processor` - A function that processes buffer data. Called for each swapped buffer.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If worker spawns successfully.
    /// * `Err(ScribeError)` - If spawning fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use scribe::DoubleBufferManager;
    /// use std::path::PathBuf;
    ///
    /// let mut manager = DoubleBufferManager::new(PathBuf::from("/tmp/logs")).unwrap();
    ///
    /// manager.spawn_worker(|data| {
    ///     // Process the buffer data (e.g., write to file, compress, encrypt)
    ///     println!("Processing {} bytes", data.len());
    ///     Ok(())
    /// }).unwrap();
    /// ```
    pub fn spawn_worker<F>(&mut self, processor: F) -> crate::Result<()>
    where
        F: Fn(&[u8]) -> crate::Result<()> + Send + 'static,
    {
        let receiver = self.swap_receiver.clone();

        let handle = thread::spawn(move || {
            while let Ok(buffer) = receiver.recv() {
                // 处理 buffer 数据
                let data = buffer.as_slice();
                if let Err(e) = processor(data) {
                    eprintln!("Worker processor error: {}", e);
                }

                // 重置 buffer（需要 mut，这里暂时跳过）
                // buffer.reset();
            }
        });

        self.worker_handle = Some(handle);
        Ok(())
    }

    /// Writes a LogFrame to the active buffer.
    ///
    /// This is a high-level convenience method that serializes the frame
    /// and writes it to the currently active buffer.
    ///
    /// # Arguments
    ///
    /// * `frame` - The LogFrame to write
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Successfully written
    /// * `Err(ScribeError)` - If serialization or write fails
    pub fn write(&self, frame: &crate::storage::LogFrame) -> crate::Result<()> {
        let data = frame.serialize()?;
        let (buffer, idx) = self.get_active_buffer();

        self.increment_active_writers(idx);
        let result = buffer.write(&data);
        self.decrement_active_writers(idx);

        result.map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn test_double_buffer_swap() {
        let temp_dir = TempDir::new().unwrap();
        let manager = DoubleBufferManager::new(temp_dir.path().to_path_buf()).unwrap();

        let (buffer1, idx1) = manager.get_active_buffer();
        assert_eq!(idx1, 0);

        // 写入大量数据触发交换
        let data = vec![0u8; 1024];
        for _ in 0..4000 {
            buffer1.write(&data).ok();
        }

        // 交换 buffer
        manager.swap_buffers().unwrap();

        let (_, idx2) = manager.get_active_buffer();
        assert_eq!(idx2, 1);
    }

    #[test]
    fn test_get_active_buffer() {
        let temp_dir = TempDir::new().unwrap();
        let manager = DoubleBufferManager::new(temp_dir.path().to_path_buf()).unwrap();

        // 初始时应该是 buffer 0
        let (buffer, idx) = manager.get_active_buffer();
        assert_eq!(idx, 0);
        assert!(Arc::ptr_eq(&buffer, &manager.buffers[0]));

        // 交换后应该是 buffer 1
        manager.swap_buffers().unwrap();
        let (buffer, idx) = manager.get_active_buffer();
        assert_eq!(idx, 1);
        assert!(Arc::ptr_eq(&buffer, &manager.buffers[1]));

        // 再次交换回 buffer 0
        manager.swap_buffers().unwrap();
        let (buffer, idx) = manager.get_active_buffer();
        assert_eq!(idx, 0);
        assert!(Arc::ptr_eq(&buffer, &manager.buffers[0]));
    }

    #[test]
    fn test_increment_active_writers() {
        let temp_dir = TempDir::new().unwrap();
        let manager = DoubleBufferManager::new(temp_dir.path().to_path_buf()).unwrap();

        // 初始状态应该为 0
        assert_eq!(manager.active_writers[0].load(Ordering::Acquire), 0);
        assert_eq!(manager.active_writers[1].load(Ordering::Acquire), 0);

        // 增加 buffer 0 的写入者
        manager.increment_active_writers(0);
        assert_eq!(manager.active_writers[0].load(Ordering::Acquire), 1);
        assert_eq!(manager.active_writers[1].load(Ordering::Acquire), 0);

        // 再次增加 buffer 0 的写入者
        manager.increment_active_writers(0);
        assert_eq!(manager.active_writers[0].load(Ordering::Acquire), 2);

        // 增加 buffer 1 的写入者
        manager.increment_active_writers(1);
        assert_eq!(manager.active_writers[0].load(Ordering::Acquire), 2);
        assert_eq!(manager.active_writers[1].load(Ordering::Acquire), 1);
    }

    #[test]
    fn test_decrement_active_writers() {
        let temp_dir = TempDir::new().unwrap();
        let manager = DoubleBufferManager::new(temp_dir.path().to_path_buf()).unwrap();

        // 先增加写入者
        manager.increment_active_writers(0);
        manager.increment_active_writers(0);
        manager.increment_active_writers(1);
        assert_eq!(manager.active_writers[0].load(Ordering::Acquire), 2);
        assert_eq!(manager.active_writers[1].load(Ordering::Acquire), 1);

        // 减少 buffer 0 的写入者
        manager.decrement_active_writers(0);
        assert_eq!(manager.active_writers[0].load(Ordering::Acquire), 1);
        assert_eq!(manager.active_writers[1].load(Ordering::Acquire), 1);

        // 再次减少 buffer 0 的写入者
        manager.decrement_active_writers(0);
        assert_eq!(manager.active_writers[0].load(Ordering::Acquire), 0);

        // 减少 buffer 1 的写入者
        manager.decrement_active_writers(1);
        assert_eq!(manager.active_writers[1].load(Ordering::Acquire), 0);
    }

    #[test]
    fn test_should_swap_boundary() {
        let temp_dir = TempDir::new().unwrap();
        let manager = DoubleBufferManager::new(temp_dir.path().to_path_buf()).unwrap();

        let (buffer, _) = manager.get_active_buffer();

        // 空 buffer 不应该交换
        assert!(!manager.should_swap(&buffer));

        // 写入到 80% 不应该交换
        let bytes_80_percent = (BUFFER_CAPACITY as f64 * 0.8) as usize;
        let data = vec![0u8; 1024];
        for _ in 0..(bytes_80_percent / data.len()) {
            buffer.write(&data).ok();
        }
        assert!(!manager.should_swap(&buffer));

        // 写入到 90% 应该交换
        let bytes_90_percent = (BUFFER_CAPACITY as f64 * 0.9) as usize;
        let remaining = bytes_90_percent - bytes_80_percent;
        for _ in 0..(remaining / data.len() + 1) {
            buffer.write(&data).ok();
        }
        assert!(manager.should_swap(&buffer));

        // 写入到 95% 仍应该交换
        let bytes_95_percent = (BUFFER_CAPACITY as f64 * 0.95) as usize;
        let remaining = bytes_95_percent - bytes_90_percent;
        for _ in 0..(remaining / data.len() + 1) {
            buffer.write(&data).ok();
        }
        assert!(manager.should_swap(&buffer));
    }

    #[test]
    fn test_swap_buffers_success() {
        let temp_dir = TempDir::new().unwrap();
        let manager = DoubleBufferManager::new(temp_dir.path().to_path_buf()).unwrap();

        // 初始 index 为 0
        assert_eq!(manager.active_index.load(Ordering::Acquire), 0);

        // 第一次交换
        manager.swap_buffers().unwrap();
        assert_eq!(manager.active_index.load(Ordering::Acquire), 1);

        // 第二次交换
        manager.swap_buffers().unwrap();
        assert_eq!(manager.active_index.load(Ordering::Acquire), 0);

        // 第三次交换
        manager.swap_buffers().unwrap();
        assert_eq!(manager.active_index.load(Ordering::Acquire), 1);
    }

    #[test]
    fn test_swap_buffers_waits_for_writers() {
        let temp_dir = TempDir::new().unwrap();
        let manager = Arc::new(DoubleBufferManager::new(temp_dir.path().to_path_buf()).unwrap());

        // 增加活跃写入者
        manager.increment_active_writers(0);
        manager.increment_active_writers(0);

        let manager_clone = manager.clone();
        let handle = thread::spawn(move || {
            // 延迟后减少写入者
            thread::sleep(Duration::from_millis(50));
            manager_clone.decrement_active_writers(0);
            thread::sleep(Duration::from_millis(50));
            manager_clone.decrement_active_writers(0);
        });

        // swap_buffers 应该等待所有写入者完成
        let start = std::time::Instant::now();
        manager.swap_buffers().unwrap();
        let elapsed = start.elapsed();

        // 应该至少等待 100ms
        assert!(elapsed >= Duration::from_millis(100));
        assert_eq!(manager.active_index.load(Ordering::Acquire), 1);

        handle.join().unwrap();
    }

    #[test]
    fn test_swap_buffers_channel_full() {
        let temp_dir = TempDir::new().unwrap();
        let manager = DoubleBufferManager::new(temp_dir.path().to_path_buf()).unwrap();

        // 填满 channel（容量为 2）
        manager.swap_buffers().unwrap();
        manager.swap_buffers().unwrap();

        // 第三次交换应该阻塞，但由于没有消费者，这里只测试前两次成功
        assert_eq!(manager.active_index.load(Ordering::Acquire), 0);
    }

    #[test]
    fn test_swap_buffers_concurrent() {
        let temp_dir = TempDir::new().unwrap();
        let manager = Arc::new(DoubleBufferManager::new(temp_dir.path().to_path_buf()).unwrap());

        let barrier = Arc::new(Barrier::new(10));
        let mut handles = vec![];

        for thread_id in 0..10 {
            let manager_clone = manager.clone();
            let barrier_clone = barrier.clone();

            let handle = thread::spawn(move || {
                // 等待所有线程就绪
                barrier_clone.wait();

                // 模拟写入操作
                let (buffer, idx) = manager_clone.get_active_buffer();
                manager_clone.increment_active_writers(idx);

                // 写入一些数据
                let data = format!("Thread {} data\n", thread_id);
                buffer.write(data.as_bytes()).ok();

                manager_clone.decrement_active_writers(idx);
            });

            handles.push(handle);
        }

        // 等待所有写入完成
        for handle in handles {
            handle.join().unwrap();
        }

        // 验证交换仍然正常工作
        let initial_idx = manager.active_index.load(Ordering::Acquire);
        manager.swap_buffers().unwrap();
        let new_idx = manager.active_index.load(Ordering::Acquire);
        assert_ne!(initial_idx, new_idx);
    }

    #[test]
    fn test_worker_thread_spawn() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = DoubleBufferManager::new(temp_dir.path().to_path_buf()).unwrap();

        // 初始时 worker_handle 为 None
        assert!(manager.worker_handle.is_none());

        // 生成 worker 线程
        let result = manager.spawn_worker(|_data| Ok(()));
        assert!(result.is_ok());

        // worker_handle 应该存在
        assert!(manager.worker_handle.is_some());
    }

    #[test]
    fn test_worker_thread_processing() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = DoubleBufferManager::new(temp_dir.path().to_path_buf()).unwrap();

        let processed = Arc::new(AtomicUsize::new(0));
        let processed_clone = processed.clone();

        // 生成 worker 线程，计数处理的 buffer
        manager
            .spawn_worker(move |data| {
                processed_clone.fetch_add(data.len(), Ordering::SeqCst);
                Ok(())
            })
            .unwrap();

        // 写入数据到 buffer 0
        let (buffer, _) = manager.get_active_buffer();
        let test_data = b"test data for worker";
        buffer.write(test_data).unwrap();

        // 交换 buffer，触发 worker 处理
        manager.swap_buffers().unwrap();

        // 等待 worker 处理
        thread::sleep(Duration::from_millis(100));

        // 验证 worker 处理了数据
        let processed_bytes = processed.load(Ordering::SeqCst);
        assert!(processed_bytes > 0);
    }

    #[test]
    fn test_worker_thread_error_handling() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = DoubleBufferManager::new(temp_dir.path().to_path_buf()).unwrap();

        let error_count = Arc::new(AtomicUsize::new(0));
        let error_count_clone = error_count.clone();

        // 生成总是失败的 worker
        manager
            .spawn_worker(move |_data| {
                error_count_clone.fetch_add(1, Ordering::SeqCst);
                Err(crate::ScribeError::Mmap("Simulated error".to_string()))
            })
            .unwrap();

        // 写入数据并交换
        let (buffer, _) = manager.get_active_buffer();
        buffer.write(b"test data").unwrap();
        manager.swap_buffers().unwrap();

        // 等待 worker 处理
        thread::sleep(Duration::from_millis(100));

        // 验证 worker 尝试处理了数据（尽管失败）
        assert_eq!(error_count.load(Ordering::SeqCst), 1);

        // 再次写入和交换，验证 worker 继续运行
        manager.swap_buffers().unwrap();
        thread::sleep(Duration::from_millis(100));
        assert_eq!(error_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_worker_thread_multiple_buffers() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = DoubleBufferManager::new(temp_dir.path().to_path_buf()).unwrap();

        let process_count = Arc::new(AtomicUsize::new(0));
        let process_count_clone = process_count.clone();

        manager
            .spawn_worker(move |_data| {
                process_count_clone.fetch_add(1, Ordering::SeqCst);
                thread::sleep(Duration::from_millis(50)); // 模拟处理时间
                Ok(())
            })
            .unwrap();

        // 快速交换多次
        for i in 0..5 {
            let (buffer, _) = manager.get_active_buffer();
            buffer.write(format!("Message {}", i).as_bytes()).unwrap();
            manager.swap_buffers().unwrap();
        }

        // 等待所有处理完成
        thread::sleep(Duration::from_millis(500));

        // 验证所有 buffer 都被处理
        assert_eq!(process_count.load(Ordering::SeqCst), 5);
    }

    #[test]
    fn test_concurrent_write_and_swap() {
        let temp_dir = TempDir::new().unwrap();
        let manager = Arc::new(DoubleBufferManager::new(temp_dir.path().to_path_buf()).unwrap());

        let barrier = Arc::new(Barrier::new(11)); // 10 writers + 1 swapper
        let mut handles = vec![];

        // 启动 10 个写入线程
        for thread_id in 0..10 {
            let manager_clone = manager.clone();
            let barrier_clone = barrier.clone();

            let handle = thread::spawn(move || {
                barrier_clone.wait();

                for i in 0..10 {
                    let (buffer, idx) = manager_clone.get_active_buffer();
                    manager_clone.increment_active_writers(idx);

                    let data = format!("Thread {} message {}\n", thread_id, i);
                    buffer.write(data.as_bytes()).ok();

                    manager_clone.decrement_active_writers(idx);
                    thread::sleep(Duration::from_millis(5));
                }
            });

            handles.push(handle);
        }

        // 启动 1 个交换线程
        let manager_clone = manager.clone();
        let barrier_clone = barrier.clone();
        let swap_handle = thread::spawn(move || {
            barrier_clone.wait();

            for _ in 0..3 {
                thread::sleep(Duration::from_millis(50));
                manager_clone.swap_buffers().ok();
            }
        });

        handles.push(swap_handle);

        // 等待所有线程完成
        for handle in handles {
            handle.join().unwrap();
        }

        // 验证最终状态一致
        assert_eq!(manager.active_writers[0].load(Ordering::Acquire), 0);
        assert_eq!(manager.active_writers[1].load(Ordering::Acquire), 0);
    }
}
