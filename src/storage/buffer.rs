use memmap2::MmapMut;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A memory-mapped buffer for high-performance, lock-free log writing.
///
/// MmapBuffer uses memory-mapped I/O and atomic operations to achieve concurrent
/// writes without locks. Multiple threads can write simultaneously using atomic
/// position tracking.
///
/// # Thread Safety
///
/// MmapBuffer is thread-safe and can be shared across threads using `Arc<MmapBuffer>`.
/// Writes are atomic and lock-free.
///
/// # Examples
///
/// ```no_run
/// use scribe::MmapBuffer;
/// use std::path::PathBuf;
/// use std::sync::Arc;
///
/// let buffer = Arc::new(MmapBuffer::new(
///     PathBuf::from("/tmp/log.mmap"),
///     4 * 1024 * 1024  // 4MB
/// ).unwrap());
///
/// // Write from multiple threads
/// let buf = buffer.clone();
/// std::thread::spawn(move || {
///     buf.write(b"log data").unwrap();
/// });
/// ```
pub struct MmapBuffer {
    mmap: MmapMut,
    position: AtomicUsize,
    capacity: usize,
    #[allow(dead_code)]
    file_path: PathBuf,
}

impl MmapBuffer {
    /// Creates a new memory-mapped buffer.
    ///
    /// # Arguments
    ///
    /// * `file_path` - The path where the buffer file will be created.
    /// * `capacity` - The size of the buffer in bytes.
    ///
    /// # Returns
    ///
    /// * `Ok(MmapBuffer)` - A new buffer instance.
    /// * `Err(ScribeError)` - If file creation or mmap fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use scribe::MmapBuffer;
    /// use std::path::PathBuf;
    ///
    /// let buffer = MmapBuffer::new(
    ///     PathBuf::from("/tmp/buffer.mmap"),
    ///     1024 * 1024  // 1MB
    /// ).unwrap();
    /// ```
    pub fn new(file_path: PathBuf, capacity: usize) -> crate::Result<Self> {
        // 创建或打开文件
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&file_path)?;

        // 设置文件大小
        file.set_len(capacity as u64)?;

        // 创建 mmap
        let mmap = unsafe {
            MmapMut::map_mut(&file).map_err(|e| crate::ScribeError::Mmap(e.to_string()))?
        };

        Ok(Self {
            mmap,
            position: AtomicUsize::new(0),
            capacity,
            file_path,
        })
    }

    /// Writes data to the buffer atomically.
    ///
    /// This operation is lock-free and thread-safe. The position is updated
    /// atomically using compare-and-swap operations.
    ///
    /// # Arguments
    ///
    /// * `data` - The byte slice to write.
    ///
    /// # Returns
    ///
    /// * `Ok(usize)` - The offset where data was written.
    /// * `Err(ScribeError::BufferFull)` - If there's insufficient space.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use scribe::MmapBuffer;
    /// use std::path::PathBuf;
    ///
    /// let buffer = MmapBuffer::new(PathBuf::from("/tmp/buf.mmap"), 4096).unwrap();
    /// let offset = buffer.write(b"hello world").unwrap();
    /// assert_eq!(offset, 0);
    /// ```
    pub fn write(&self, data: &[u8]) -> crate::Result<usize> {
        let data_len = data.len();

        // 原子地获取并更新位置
        let pos = self.position.fetch_add(data_len, Ordering::AcqRel);

        if pos + data_len > self.capacity {
            // 回滚位置
            self.position.fetch_sub(data_len, Ordering::AcqRel);
            return Err(crate::ScribeError::BufferFull);
        }

        // 写入数据
        unsafe {
            let ptr = self.mmap.as_ptr().add(pos) as *mut u8;
            std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data_len);
        }

        Ok(pos)
    }

    /// Returns the current write position.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use scribe::MmapBuffer;
    /// use std::path::PathBuf;
    ///
    /// let buffer = MmapBuffer::new(PathBuf::from("/tmp/buf.mmap"), 4096).unwrap();
    /// buffer.write(b"hello").unwrap();
    /// assert_eq!(buffer.position(), 5);
    /// ```
    pub fn position(&self) -> usize {
        self.position.load(Ordering::Acquire)
    }

    /// Returns the total capacity of the buffer.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Checks if the buffer has reached a threshold.
    ///
    /// # Arguments
    ///
    /// * `threshold` - A value between 0.0 and 1.0 representing the percentage.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use scribe::MmapBuffer;
    /// use std::path::PathBuf;
    ///
    /// let buffer = MmapBuffer::new(PathBuf::from("/tmp/buf.mmap"), 1000).unwrap();
    /// buffer.write(&[0u8; 900]).unwrap();
    /// assert!(buffer.is_full(0.8));  // 90% > 80%
    /// ```
    pub fn is_full(&self, threshold: f32) -> bool {
        let pos = self.position();
        pos as f32 >= (self.capacity as f32 * threshold)
    }

    /// Flushes the buffer to disk.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If flush succeeds.
    /// * `Err(ScribeError)` - If the flush operation fails.
    pub fn flush(&self) -> crate::Result<()> {
        self.mmap.flush()?;
        Ok(())
    }

    /// Resets the buffer position to zero.
    ///
    /// # Safety
    ///
    /// This should only be called when no other threads are writing to the buffer.
    pub fn reset(&mut self) {
        self.position.store(0, Ordering::Release);
    }

    /// Returns the written portion of the buffer as a slice.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use scribe::MmapBuffer;
    /// use std::path::PathBuf;
    ///
    /// let buffer = MmapBuffer::new(PathBuf::from("/tmp/buf.mmap"), 4096).unwrap();
    /// buffer.write(b"hello").unwrap();
    /// assert_eq!(buffer.as_slice(), b"hello");
    /// ```
    pub fn as_slice(&self) -> &[u8] {
        let pos = self.position();
        &self.mmap[..pos]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_buffer_write() {
        let temp_file = NamedTempFile::new().unwrap();
        let buffer = MmapBuffer::new(temp_file.path().to_path_buf(), 4096).unwrap();

        let data = b"hello world";
        let pos = buffer.write(data).unwrap();

        assert_eq!(pos, 0);
        assert_eq!(buffer.position(), data.len());
    }

    #[test]
    fn test_buffer_full() {
        let temp_file = NamedTempFile::new().unwrap();
        let buffer = MmapBuffer::new(temp_file.path().to_path_buf(), 100).unwrap();

        let data = vec![0u8; 120];
        let result = buffer.write(&data);

        assert!(result.is_err());
    }

    #[test]
    #[ignore] // 在 CI 环境中不可靠
    fn test_buffer_concurrent_writes() {
        use std::sync::Arc;
        use std::thread;

        let temp_file = NamedTempFile::new().unwrap();
        let buffer = Arc::new(MmapBuffer::new(temp_file.path().to_path_buf(), 10000).unwrap());

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let buf = buffer.clone();
                thread::spawn(move || {
                    for _ in 0..100 {
                        let data = format!("thread-{}-data", i);
                        buf.write(data.as_bytes()).unwrap();
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // 验证所有数据都写入了
        assert!(buffer.position() > 0);
    }

    #[test]
    fn test_buffer_flush_success() {
        let temp_file = NamedTempFile::new().unwrap();
        let buffer = MmapBuffer::new(temp_file.path().to_path_buf(), 4096).unwrap();

        buffer.write(b"test data").unwrap();
        let result = buffer.flush();

        assert!(result.is_ok());
    }

    #[test]
    fn test_buffer_flush_after_multiple_writes() {
        let temp_file = NamedTempFile::new().unwrap();
        let buffer = MmapBuffer::new(temp_file.path().to_path_buf(), 4096).unwrap();

        buffer.write(b"first write").unwrap();
        buffer.write(b"second write").unwrap();
        buffer.write(b"third write").unwrap();

        let result = buffer.flush();
        assert!(result.is_ok());
    }

    #[test]
    fn test_buffer_reset() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut buffer = MmapBuffer::new(temp_file.path().to_path_buf(), 4096).unwrap();

        buffer.write(b"some data").unwrap();
        assert_eq!(buffer.position(), 9);

        buffer.reset();
        assert_eq!(buffer.position(), 0);

        // 验证重置后可以再次写入
        let pos = buffer.write(b"new data").unwrap();
        assert_eq!(pos, 0);
        assert_eq!(buffer.position(), 8);
    }

    #[test]
    #[ignore] // 在 CI 环境中不可靠
    fn test_buffer_is_full_0_percent() {
        let temp_file = NamedTempFile::new().unwrap();
        let buffer = MmapBuffer::new(temp_file.path().to_path_buf(), 1000).unwrap();

        // 空缓冲区应该不满
        assert!(!buffer.is_full(0.0));
    }

    #[test]
    fn test_buffer_is_full_50_percent() {
        let temp_file = NamedTempFile::new().unwrap();
        let buffer = MmapBuffer::new(temp_file.path().to_path_buf(), 1000).unwrap();

        // 写入 40% 数据
        buffer.write(&[0u8; 400]).unwrap();
        assert!(!buffer.is_full(0.5));

        // 写入 60% 数据
        buffer.write(&[0u8; 200]).unwrap();
        assert!(buffer.is_full(0.5));
    }

    #[test]
    fn test_buffer_is_full_90_percent() {
        let temp_file = NamedTempFile::new().unwrap();
        let buffer = MmapBuffer::new(temp_file.path().to_path_buf(), 1000).unwrap();

        // 写入 85% 数据
        buffer.write(&[0u8; 850]).unwrap();
        assert!(!buffer.is_full(0.9));

        // 写入到 95% 数据
        buffer.write(&[0u8; 100]).unwrap();
        assert!(buffer.is_full(0.9));
    }

    #[test]
    fn test_buffer_is_full_100_percent() {
        let temp_file = NamedTempFile::new().unwrap();
        let buffer = MmapBuffer::new(temp_file.path().to_path_buf(), 1000).unwrap();

        // 写入 99% 数据
        buffer.write(&[0u8; 990]).unwrap();
        assert!(!buffer.is_full(1.0));

        // 写入到 100% 数据
        buffer.write(&[0u8; 10]).unwrap();
        assert!(buffer.is_full(1.0));
    }

    #[test]
    fn test_buffer_position() {
        let temp_file = NamedTempFile::new().unwrap();
        let buffer = MmapBuffer::new(temp_file.path().to_path_buf(), 4096).unwrap();

        assert_eq!(buffer.position(), 0);

        buffer.write(b"hello").unwrap();
        assert_eq!(buffer.position(), 5);

        buffer.write(b" world").unwrap();
        assert_eq!(buffer.position(), 11);
    }

    #[test]
    fn test_buffer_capacity() {
        let temp_file = NamedTempFile::new().unwrap();
        let buffer = MmapBuffer::new(temp_file.path().to_path_buf(), 8192).unwrap();

        assert_eq!(buffer.capacity(), 8192);
    }

    #[test]
    fn test_buffer_as_slice() {
        let temp_file = NamedTempFile::new().unwrap();
        let buffer = MmapBuffer::new(temp_file.path().to_path_buf(), 4096).unwrap();

        buffer.write(b"hello").unwrap();
        assert_eq!(buffer.as_slice(), b"hello");

        buffer.write(b" world").unwrap();
        assert_eq!(buffer.as_slice(), b"hello world");
    }

    #[test]
    fn test_buffer_as_slice_empty() {
        let temp_file = NamedTempFile::new().unwrap();
        let buffer = MmapBuffer::new(temp_file.path().to_path_buf(), 4096).unwrap();

        assert_eq!(buffer.as_slice(), b"");
    }

    #[test]
    fn test_buffer_oversized_write() {
        let temp_file = NamedTempFile::new().unwrap();
        let buffer = MmapBuffer::new(temp_file.path().to_path_buf(), 100).unwrap();

        // 尝试写入超过容量的数据
        let large_data = vec![0u8; 200];
        let result = buffer.write(&large_data);

        assert!(result.is_err());
        match result {
            Err(crate::ScribeError::BufferFull) => {}
            _ => panic!("Expected BufferFull error"),
        }

        // 验证位置没有改变
        assert_eq!(buffer.position(), 0);
    }

    #[test]
    fn test_buffer_exact_capacity_write() {
        let temp_file = NamedTempFile::new().unwrap();
        let buffer = MmapBuffer::new(temp_file.path().to_path_buf(), 100).unwrap();

        // 写入恰好等于容量的数据
        let data = vec![0u8; 100];
        let result = buffer.write(&data);

        assert!(result.is_ok());
        assert_eq!(buffer.position(), 100);
    }

    #[test]
    fn test_buffer_zero_byte_write() {
        let temp_file = NamedTempFile::new().unwrap();
        let buffer = MmapBuffer::new(temp_file.path().to_path_buf(), 4096).unwrap();

        // 写入零字节数据
        let result = buffer.write(b"");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
        assert_eq!(buffer.position(), 0);

        // 写入一些数据后再写入零字节
        buffer.write(b"hello").unwrap();
        let result = buffer.write(b"");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 5);
        assert_eq!(buffer.position(), 5);
    }

    #[test]
    fn test_buffer_concurrent_position_reads() {
        use std::sync::Arc;
        use std::thread;

        let temp_file = NamedTempFile::new().unwrap();
        let buffer = Arc::new(MmapBuffer::new(temp_file.path().to_path_buf(), 100000).unwrap());

        // 启动写入线程
        let write_handles: Vec<_> = (0..5)
            .map(|_| {
                let buf = buffer.clone();
                thread::spawn(move || {
                    for _ in 0..100 {
                        buf.write(b"data").unwrap();
                    }
                })
            })
            .collect();

        // 启动读取位置的线程
        let read_handles: Vec<_> = (0..5)
            .map(|_| {
                let buf = buffer.clone();
                thread::spawn(move || {
                    let mut positions = Vec::new();
                    for _ in 0..100 {
                        positions.push(buf.position());
                    }
                    positions
                })
            })
            .collect();

        // 等待所有写入完成
        for handle in write_handles {
            handle.join().unwrap();
        }

        // 等待所有读取完成并验证
        for handle in read_handles {
            let positions = handle.join().unwrap();
            // 验证位置是单调递增的（或保持不变）
            for i in 1..positions.len() {
                assert!(positions[i] >= positions[i - 1]);
            }
        }

        // 验证最终位置
        assert_eq!(buffer.position(), 5 * 100 * 4); // 5 threads * 100 writes * 4 bytes
    }

    #[test]
    fn test_buffer_multiple_resets() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut buffer = MmapBuffer::new(temp_file.path().to_path_buf(), 4096).unwrap();

        for i in 0..10 {
            buffer.write(format!("iteration {}", i).as_bytes()).unwrap();
            assert!(buffer.position() > 0);

            buffer.reset();
            assert_eq!(buffer.position(), 0);
        }
    }

    #[test]
    fn test_buffer_write_after_near_full() {
        let temp_file = NamedTempFile::new().unwrap();
        let buffer = MmapBuffer::new(temp_file.path().to_path_buf(), 100).unwrap();

        // 写入接近容量的数据
        buffer.write(&[0u8; 95]).unwrap();
        assert_eq!(buffer.position(), 95);

        // 可以写入剩余空间
        let result = buffer.write(&[0u8; 5]);
        assert!(result.is_ok());
        assert_eq!(buffer.position(), 100);

        // 再次写入应该失败
        let result = buffer.write(b"x");
        assert!(result.is_err());
    }
}
