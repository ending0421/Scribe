use crate::Result;

pub trait LogOutput: Send + Sync {
    fn write(&self, data: &[u8]) -> Result<()>;
    fn flush(&self) -> Result<()>;
}

pub struct DiskOutput {
    file_path: std::path::PathBuf,
    file: std::sync::Mutex<std::fs::File>,
}

impl DiskOutput {
    pub fn new(file_path: std::path::PathBuf) -> Result<Self> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;

        Ok(Self {
            file_path,
            file: std::sync::Mutex::new(file),
        })
    }
}

impl LogOutput for DiskOutput {
    fn write(&self, data: &[u8]) -> Result<()> {
        use std::io::Write;
        let mut file = self.file.lock().unwrap();
        file.write_all(data)?;
        Ok(())
    }

    fn flush(&self) -> Result<()> {
        use std::io::Write;
        let mut file = self.file.lock().unwrap();
        file.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{NamedTempFile, TempDir};
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn test_disk_output() {
        let temp_file = NamedTempFile::new().unwrap();
        let output = DiskOutput::new(temp_file.path().to_path_buf()).unwrap();

        let data = b"test data";
        output.write(data).unwrap();
        output.flush().unwrap();

        let content = std::fs::read(temp_file.path()).unwrap();
        assert_eq!(content, data);
    }

    #[test]
    fn test_multiple_writes() {
        let temp_file = NamedTempFile::new().unwrap();
        let output = DiskOutput::new(temp_file.path().to_path_buf()).unwrap();

        output.write(b"line1\n").unwrap();
        output.write(b"line2\n").unwrap();
        output.write(b"line3\n").unwrap();
        output.flush().unwrap();

        let content = std::fs::read(temp_file.path()).unwrap();
        assert_eq!(content, b"line1\nline2\nline3\n");
    }

    #[test]
    fn test_large_write() {
        let temp_file = NamedTempFile::new().unwrap();
        let output = DiskOutput::new(temp_file.path().to_path_buf()).unwrap();

        // 1MB 数据
        let data = vec![b'x'; 1024 * 1024];
        output.write(&data).unwrap();
        output.flush().unwrap();

        let content = std::fs::read(temp_file.path()).unwrap();
        assert_eq!(content, data);
    }

    #[test]
    fn test_permission_error() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("readonly_dir").join("output.log");

        // 创建只读目录
        let readonly_dir = temp_dir.path().join("readonly_dir");
        std::fs::create_dir(&readonly_dir).unwrap();
        std::fs::set_permissions(&readonly_dir, std::fs::Permissions::from_mode(0o444)).unwrap();

        // 尝试在只读目录中创建文件应该失败
        let result = DiskOutput::new(file_path);
        assert!(result.is_err());

        // 清理：恢复权限以便删除
        std::fs::set_permissions(&readonly_dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[test]
    fn test_concurrent_writes() {
        use std::sync::Arc;
        use std::thread;

        let temp_file = NamedTempFile::new().unwrap();
        let output = Arc::new(DiskOutput::new(temp_file.path().to_path_buf()).unwrap());

        let mut handles = vec![];

        // 10 个线程同时写入
        for i in 0..10 {
            let output_clone = Arc::clone(&output);
            let handle = thread::spawn(move || {
                let data = format!("thread_{}\n", i);
                output_clone.write(data.as_bytes()).unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        output.flush().unwrap();

        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        // 应该有 10 行
        assert_eq!(lines.len(), 10);

        // 每个线程的数据都应该存在
        for i in 0..10 {
            let expected = format!("thread_{}", i);
            assert!(lines.iter().any(|line| *line == expected));
        }
    }

    #[test]
    fn test_flush_empty() {
        let temp_file = NamedTempFile::new().unwrap();
        let output = DiskOutput::new(temp_file.path().to_path_buf()).unwrap();

        // 不写入任何数据，直接 flush 应该成功
        output.flush().unwrap();

        let content = std::fs::read(temp_file.path()).unwrap();
        assert!(content.is_empty());
    }

    #[test]
    fn test_append_mode() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        // 第一次写入
        {
            let output = DiskOutput::new(path.clone()).unwrap();
            output.write(b"first\n").unwrap();
            output.flush().unwrap();
        }

        // 第二次写入（应该追加）
        {
            let output = DiskOutput::new(path.clone()).unwrap();
            output.write(b"second\n").unwrap();
            output.flush().unwrap();
        }

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "first\nsecond\n");
    }

    #[test]
    fn test_write_empty_data() {
        let temp_file = NamedTempFile::new().unwrap();
        let output = DiskOutput::new(temp_file.path().to_path_buf()).unwrap();

        output.write(&[]).unwrap();
        output.flush().unwrap();

        let content = std::fs::read(temp_file.path()).unwrap();
        assert!(content.is_empty());
    }
}
