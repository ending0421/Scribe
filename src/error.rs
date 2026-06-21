use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScribeError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("mmap error: {0}")]
    Mmap(String),

    #[error("Disk full")]
    DiskFull,

    #[error("Buffer full")]
    BufferFull,

    #[error("Already initialized")]
    AlreadyInitialized,

    #[error("Not initialized")]
    NotInitialized,

    #[error("Invalid log frame")]
    InvalidFrame,

    #[error("CRC mismatch")]
    CrcMismatch,

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Encryption error: {0}")]
    Encryption(String),
}

pub type Result<T> = std::result::Result<T, ScribeError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_error_mmap() {
        let error = ScribeError::Mmap("mmap failed".to_string());
        assert_eq!(error.to_string(), "mmap error: mmap failed");
    }

    #[test]
    fn test_error_disk_full() {
        let error = ScribeError::DiskFull;
        assert_eq!(error.to_string(), "Disk full");
    }

    #[test]
    fn test_error_buffer_full() {
        let error = ScribeError::BufferFull;
        assert_eq!(error.to_string(), "Buffer full");
    }

    #[test]
    fn test_error_already_initialized() {
        let error = ScribeError::AlreadyInitialized;
        assert_eq!(error.to_string(), "Already initialized");
    }

    #[test]
    fn test_error_not_initialized() {
        let error = ScribeError::NotInitialized;
        assert_eq!(error.to_string(), "Not initialized");
    }

    #[test]
    fn test_error_invalid_frame() {
        let error = ScribeError::InvalidFrame;
        assert_eq!(error.to_string(), "Invalid log frame");
    }

    #[test]
    fn test_error_crc_mismatch() {
        let error = ScribeError::CrcMismatch;
        assert_eq!(error.to_string(), "CRC mismatch");
    }

    #[test]
    fn test_error_compression() {
        let error = ScribeError::Compression("zstd error".to_string());
        assert_eq!(error.to_string(), "Compression error: zstd error");
    }

    #[test]
    fn test_error_encryption() {
        let error = ScribeError::Encryption("aes error".to_string());
        assert_eq!(error.to_string(), "Encryption error: aes error");
    }

    #[test]
    fn test_error_display() {
        let error = ScribeError::DiskFull;
        let display_output = format!("{}", error);
        assert_eq!(display_output, "Disk full");

        let error = ScribeError::Mmap("test".to_string());
        let display_output = format!("{}", error);
        assert_eq!(display_output, "mmap error: test");
    }

    #[test]
    fn test_error_debug() {
        let error = ScribeError::BufferFull;
        let debug_output = format!("{:?}", error);
        assert_eq!(debug_output, "BufferFull");

        let error = ScribeError::Compression("test".to_string());
        let debug_output = format!("{:?}", error);
        assert!(debug_output.contains("Compression"));
        assert!(debug_output.contains("test"));
    }

    #[test]
    fn test_from_io_error() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let scribe_error: ScribeError = io_error.into();

        match scribe_error {
            ScribeError::Io(_) => {
                assert!(scribe_error.to_string().contains("IO error"));
                assert!(scribe_error.to_string().contains("file not found"));
            }
            _ => panic!("Expected ScribeError::Io variant"),
        }
    }

    #[test]
    fn test_from_io_error_permission_denied() {
        let io_error = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
        let scribe_error: ScribeError = ScribeError::from(io_error);

        assert!(matches!(scribe_error, ScribeError::Io(_)));
        assert!(scribe_error.to_string().contains("access denied"));
    }

    #[test]
    fn test_result_type_alias_ok() {
        let result: Result<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_result_type_alias_err() {
        let result: Result<i32> = Err(ScribeError::DiskFull);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Disk full");
    }

    #[test]
    fn test_result_type_alias_propagation() {
        fn returns_error() -> Result<()> {
            Err(ScribeError::NotInitialized)
        }

        fn calls_returns_error() -> Result<()> {
            returns_error()?;
            Ok(())
        }

        let result = calls_returns_error();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Not initialized");
    }

    #[test]
    fn test_io_error_automatic_conversion() {
        fn read_file() -> Result<String> {
            std::fs::read_to_string("/nonexistent/file")?;
            Ok("content".to_string())
        }

        let result = read_file();
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, ScribeError::Io(_)));
    }

    #[test]
    fn test_error_is_send_and_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<ScribeError>();
        assert_sync::<ScribeError>();
    }
}
