//! Recovery utilities for corrupted log files.
//!
//! This module is kept for completeness but currently unused in the simplified FFI API.

#![allow(dead_code)]

use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::storage::frame::LogFrame;
use crate::Result;

const MAGIC_HEADER: u32 = 0xFEEDC0DE;
const MIN_FRAME_SIZE: usize = 25; // Magic(4) + Length(4) + Timestamp(8) + Level(1) + TagLen(2) + MsgLen(4) + CRC(4)

#[derive(Debug, Clone)]
pub struct RecoveryReport {
    pub files_processed: usize,
    pub frames_recovered: usize,
    pub frames_corrupted: usize,
    pub bytes_recovered: u64,
}

impl RecoveryReport {
    pub fn new() -> Self {
        Self {
            files_processed: 0,
            frames_recovered: 0,
            frames_corrupted: 0,
            bytes_recovered: 0,
        }
    }

    pub fn merge(&mut self, other: &RecoveryReport) {
        self.files_processed += other.files_processed;
        self.frames_recovered += other.frames_recovered;
        self.frames_corrupted += other.frames_corrupted;
        self.bytes_recovered += other.bytes_recovered;
    }
}

pub struct Recovery {
    log_dir: PathBuf,
}

impl Recovery {
    pub fn new(log_dir: PathBuf) -> Self {
        Self { log_dir }
    }

    /// 扫描 log_dir 中的所有 .mmap 文件
    pub fn scan_mmap_files(&self) -> Result<Vec<PathBuf>> {
        let mut mmap_files = Vec::new();

        if !self.log_dir.exists() {
            return Ok(mmap_files);
        }

        let entries = fs::read_dir(&self.log_dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if extension == "mmap" {
                        mmap_files.push(path);
                    }
                }
            }
        }

        // 按文件名排序以保证处理顺序
        mmap_files.sort();

        Ok(mmap_files)
    }

    /// 验证并恢复单个 mmap 文件中的有效日志帧
    pub fn validate_and_recover(&self, file: &Path) -> Result<Vec<LogFrame>> {
        let mut file_handle = File::open(file)?;
        let mut buffer = Vec::new();
        file_handle.read_to_end(&mut buffer)?;

        let mut frames = Vec::new();
        let mut offset = 0;

        while offset + MIN_FRAME_SIZE <= buffer.len() {
            // 尝试读取 Magic Header
            let magic = u32::from_le_bytes([
                buffer[offset],
                buffer[offset + 1],
                buffer[offset + 2],
                buffer[offset + 3],
            ]);

            if magic != MAGIC_HEADER {
                // 未找到有效的 Magic Header，尝试下一个字节
                offset += 1;
                continue;
            }

            // 读取 Frame Length
            let frame_length = u32::from_le_bytes([
                buffer[offset + 4],
                buffer[offset + 5],
                buffer[offset + 6],
                buffer[offset + 7],
            ]) as usize;

            // 验证 Frame Length 的合理性
            if !(MIN_FRAME_SIZE..=1024 * 1024).contains(&frame_length) {
                // 帧长度不合理，跳过此 Magic
                offset += 1;
                continue;
            }

            // 检查是否有足够的数据
            if offset + frame_length > buffer.len() {
                // 数据不完整，停止恢复
                break;
            }

            // 尝试反序列化整个帧
            match LogFrame::deserialize(&buffer[offset..offset + frame_length]) {
                Ok(frame) => {
                    frames.push(frame);
                    offset += frame_length;
                }
                Err(_) => {
                    // 反序列化失败（CRC 错误或格式错误），跳过此帧
                    offset += 1;
                }
            }
        }

        Ok(frames)
    }

    /// 恢复所有 mmap 文件
    pub fn recover_all(&self) -> Result<RecoveryReport> {
        let mut report = RecoveryReport::new();

        // 扫描所有 mmap 文件
        let mmap_files = self.scan_mmap_files()?;

        if mmap_files.is_empty() {
            return Ok(report);
        }

        // 创建持久化日志文件
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let recovered_log_path = self.log_dir.join(format!("recovered_{}.log.gz", timestamp));

        let recovered_file = File::create(&recovered_log_path)?;
        let mut encoder = GzEncoder::new(recovered_file, Compression::default());

        // 逐个处理 mmap 文件
        for mmap_file in &mmap_files {
            report.files_processed += 1;

            match self.validate_and_recover(mmap_file) {
                Ok(frames) => {
                    let frame_count = frames.len();

                    // 将恢复的帧写入压缩日志
                    for frame in frames {
                        let serialized = frame.serialize()?;
                        encoder.write_all(&serialized)?;
                        report.bytes_recovered += serialized.len() as u64;
                    }

                    report.frames_recovered += frame_count;

                    // 删除已处理的 mmap 文件
                    if let Err(e) = fs::remove_file(mmap_file) {
                        eprintln!("Failed to remove mmap file {:?}: {}", mmap_file, e);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to recover file {:?}: {}", mmap_file, e);
                    report.frames_corrupted += 1;
                }
            }
        }

        // 完成压缩
        encoder.finish()?;

        Ok(report)
    }

    /// 恢复单个文件的详细报告
    pub fn recover_file(&self, file: &Path) -> Result<RecoveryReport> {
        let mut report = RecoveryReport::new();
        report.files_processed = 1;

        let mut file_handle = File::open(file)?;
        let mut buffer = Vec::new();
        file_handle.read_to_end(&mut buffer)?;

        let mut offset = 0;
        let mut frames_found = 0;
        let mut frames_corrupted = 0;

        while offset + MIN_FRAME_SIZE <= buffer.len() {
            let magic = u32::from_le_bytes([
                buffer[offset],
                buffer[offset + 1],
                buffer[offset + 2],
                buffer[offset + 3],
            ]);

            if magic != MAGIC_HEADER {
                offset += 1;
                continue;
            }

            let frame_length = u32::from_le_bytes([
                buffer[offset + 4],
                buffer[offset + 5],
                buffer[offset + 6],
                buffer[offset + 7],
            ]) as usize;

            if !(MIN_FRAME_SIZE..=1024 * 1024).contains(&frame_length) {
                offset += 1;
                continue;
            }

            if offset + frame_length > buffer.len() {
                break;
            }

            match LogFrame::deserialize(&buffer[offset..offset + frame_length]) {
                Ok(_) => {
                    frames_found += 1;
                    report.bytes_recovered += frame_length as u64;
                    offset += frame_length;
                }
                Err(_) => {
                    frames_corrupted += 1;
                    offset += 1;
                }
            }
        }

        report.frames_recovered = frames_found;
        report.frames_corrupted = frames_corrupted;

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::frame::LogLevel;
    use tempfile::TempDir;

    fn create_test_frame(level: LogLevel, tag: &str, message: &str) -> LogFrame {
        LogFrame::new(level, tag.to_string(), message.to_string())
    }

    #[test]
    fn test_scan_mmap_files() {
        let temp_dir = TempDir::new().unwrap();
        let recovery = Recovery::new(temp_dir.path().to_path_buf());

        // 创建一些测试文件
        File::create(temp_dir.path().join("test1.mmap")).unwrap();
        File::create(temp_dir.path().join("test2.mmap")).unwrap();
        File::create(temp_dir.path().join("test.log")).unwrap();

        let mmap_files = recovery.scan_mmap_files().unwrap();
        assert_eq!(mmap_files.len(), 2);
    }

    #[test]
    fn test_normal_recovery() {
        let temp_dir = TempDir::new().unwrap();
        let recovery = Recovery::new(temp_dir.path().to_path_buf());

        // 创建包含有效日志帧的 mmap 文件
        let mmap_path = temp_dir.path().join("test.mmap");
        let mut file = File::create(&mmap_path).unwrap();

        let frame1 = create_test_frame(LogLevel::Info, "test", "message1");
        let frame2 = create_test_frame(LogLevel::Debug, "test", "message2");
        let frame3 = create_test_frame(LogLevel::Error, "test", "message3");

        file.write_all(&frame1.serialize().unwrap()).unwrap();
        file.write_all(&frame2.serialize().unwrap()).unwrap();
        file.write_all(&frame3.serialize().unwrap()).unwrap();
        drop(file);

        // 恢复日志帧
        let frames = recovery.validate_and_recover(&mmap_path).unwrap();
        assert_eq!(frames.len(), 3);
        assert_eq!(frames[0].message, "message1");
        assert_eq!(frames[1].message, "message2");
        assert_eq!(frames[2].message, "message3");
    }

    #[test]
    fn test_partial_corruption_recovery() {
        let temp_dir = TempDir::new().unwrap();
        let recovery = Recovery::new(temp_dir.path().to_path_buf());

        let mmap_path = temp_dir.path().join("test.mmap");
        let mut file = File::create(&mmap_path).unwrap();

        // 写入第一个有效帧
        let frame1 = create_test_frame(LogLevel::Info, "test", "valid_frame_1");
        let serialized1 = frame1.serialize().unwrap();
        file.write_all(&serialized1).unwrap();

        // 写入损坏的数据
        let corrupted_data = vec![0xFF; 50];
        file.write_all(&corrupted_data).unwrap();

        // 写入第二个有效帧
        let frame2 = create_test_frame(LogLevel::Debug, "test", "valid_frame_2");
        let serialized2 = frame2.serialize().unwrap();
        file.write_all(&serialized2).unwrap();

        drop(file);

        // 恢复应该找到两个有效帧
        let frames = recovery.validate_and_recover(&mmap_path).unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].message, "valid_frame_1");
        assert_eq!(frames[1].message, "valid_frame_2");
    }

    #[test]
    fn test_complete_corruption_recovery() {
        let temp_dir = TempDir::new().unwrap();
        let recovery = Recovery::new(temp_dir.path().to_path_buf());

        let mmap_path = temp_dir.path().join("test.mmap");
        let mut file = File::create(&mmap_path).unwrap();

        // 写入完全损坏的数据
        let corrupted_data = vec![0xAB; 1024];
        file.write_all(&corrupted_data).unwrap();
        drop(file);

        // 恢复应该返回空列表
        let frames = recovery.validate_and_recover(&mmap_path).unwrap();
        assert_eq!(frames.len(), 0);
    }

    #[test]
    fn test_incomplete_frame_recovery() {
        let temp_dir = TempDir::new().unwrap();
        let recovery = Recovery::new(temp_dir.path().to_path_buf());

        let mmap_path = temp_dir.path().join("test.mmap");
        let mut file = File::create(&mmap_path).unwrap();

        // 写入一个完整的帧
        let frame1 = create_test_frame(LogLevel::Info, "test", "complete_frame");
        file.write_all(&frame1.serialize().unwrap()).unwrap();

        // 写入一个不完整的帧（只写入一半）
        let frame2 = create_test_frame(LogLevel::Debug, "test", "incomplete_frame");
        let serialized2 = frame2.serialize().unwrap();
        file.write_all(&serialized2[..serialized2.len() / 2])
            .unwrap();

        drop(file);

        // 恢复应该只找到完整的帧
        let frames = recovery.validate_and_recover(&mmap_path).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].message, "complete_frame");
    }

    #[test]
    fn test_multi_file_recovery() {
        let temp_dir = TempDir::new().unwrap();
        let recovery = Recovery::new(temp_dir.path().to_path_buf());

        // 创建多个 mmap 文件
        for i in 0..3 {
            let mmap_path = temp_dir.path().join(format!("test{}.mmap", i));
            let mut file = File::create(&mmap_path).unwrap();

            for j in 0..5 {
                let frame =
                    create_test_frame(LogLevel::Info, "test", &format!("file{}_frame{}", i, j));
                file.write_all(&frame.serialize().unwrap()).unwrap();
            }
        }

        // 恢复所有文件
        let report = recovery.recover_all().unwrap();

        assert_eq!(report.files_processed, 3);
        assert_eq!(report.frames_recovered, 15); // 3 files * 5 frames
        assert_eq!(report.frames_corrupted, 0);
        assert!(report.bytes_recovered > 0);

        // 验证 mmap 文件已被删除
        let remaining_files = recovery.scan_mmap_files().unwrap();
        assert_eq!(remaining_files.len(), 0);

        // 验证创建了恢复日志文件
        let entries: Vec<_> = fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with("recovered_")
            })
            .collect();

        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_crc_corrupted_frame_recovery() {
        let temp_dir = TempDir::new().unwrap();
        let recovery = Recovery::new(temp_dir.path().to_path_buf());

        let mmap_path = temp_dir.path().join("test.mmap");
        let mut file = File::create(&mmap_path).unwrap();

        // 写入一个有效帧
        let frame1 = create_test_frame(LogLevel::Info, "test", "valid_frame");
        file.write_all(&frame1.serialize().unwrap()).unwrap();

        // 写入一个 CRC 损坏的帧
        let frame2 = create_test_frame(LogLevel::Error, "test", "corrupted_crc_frame");
        let mut serialized2 = frame2.serialize().unwrap();
        // 篡改 CRC（最后 4 字节）
        let len = serialized2.len();
        serialized2[len - 1] ^= 0xFF;
        file.write_all(&serialized2).unwrap();

        // 写入另一个有效帧
        let frame3 = create_test_frame(LogLevel::Debug, "test", "another_valid_frame");
        file.write_all(&frame3.serialize().unwrap()).unwrap();

        drop(file);

        // 恢复应该跳过 CRC 损坏的帧
        let frames = recovery.validate_and_recover(&mmap_path).unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].message, "valid_frame");
        assert_eq!(frames[1].message, "another_valid_frame");
    }

    #[test]
    fn test_recover_file_report() {
        let temp_dir = TempDir::new().unwrap();
        let recovery = Recovery::new(temp_dir.path().to_path_buf());

        let mmap_path = temp_dir.path().join("test.mmap");
        let mut file = File::create(&mmap_path).unwrap();

        // 写入 3 个有效帧
        for i in 0..3 {
            let frame = create_test_frame(LogLevel::Info, "test", &format!("message{}", i));
            file.write_all(&frame.serialize().unwrap()).unwrap();
        }

        // 写入损坏数据
        file.write_all(&[0xFF; 100]).unwrap();

        drop(file);

        // 生成恢复报告
        let report = recovery.recover_file(&mmap_path).unwrap();

        assert_eq!(report.files_processed, 1);
        assert_eq!(report.frames_recovered, 3);
        assert!(report.bytes_recovered > 0);
    }

    #[test]
    fn test_empty_directory_recovery() {
        let temp_dir = TempDir::new().unwrap();
        let recovery = Recovery::new(temp_dir.path().to_path_buf());

        let report = recovery.recover_all().unwrap();

        assert_eq!(report.files_processed, 0);
        assert_eq!(report.frames_recovered, 0);
        assert_eq!(report.frames_corrupted, 0);
        assert_eq!(report.bytes_recovered, 0);
    }

    #[test]
    fn test_recovery_report_merge() {
        let mut report1 = RecoveryReport::new();
        report1.files_processed = 2;
        report1.frames_recovered = 10;
        report1.frames_corrupted = 1;
        report1.bytes_recovered = 1000;

        let mut report2 = RecoveryReport::new();
        report2.files_processed = 3;
        report2.frames_recovered = 15;
        report2.frames_corrupted = 2;
        report2.bytes_recovered = 2000;

        report1.merge(&report2);

        assert_eq!(report1.files_processed, 5);
        assert_eq!(report1.frames_recovered, 25);
        assert_eq!(report1.frames_corrupted, 3);
        assert_eq!(report1.bytes_recovered, 3000);
    }

    #[test]
    fn test_large_frame_rejection() {
        let temp_dir = TempDir::new().unwrap();
        let recovery = Recovery::new(temp_dir.path().to_path_buf());

        let mmap_path = temp_dir.path().join("test.mmap");
        let mut file = File::create(&mmap_path).unwrap();

        // 写入一个有效帧
        let frame1 = create_test_frame(LogLevel::Info, "test", "valid_frame");
        file.write_all(&frame1.serialize().unwrap()).unwrap();

        // 写入一个声称长度过大的帧头
        let magic_bytes = MAGIC_HEADER.to_le_bytes();
        let invalid_length = (2 * 1024 * 1024u32).to_le_bytes(); // 2MB，超过限制
        file.write_all(&magic_bytes).unwrap();
        file.write_all(&invalid_length).unwrap();
        file.write_all(&[0u8; 100]).unwrap();

        // 写入另一个有效帧
        let frame2 = create_test_frame(LogLevel::Debug, "test", "another_valid");
        file.write_all(&frame2.serialize().unwrap()).unwrap();

        drop(file);

        // 恢复应该跳过无效长度的帧
        let frames = recovery.validate_and_recover(&mmap_path).unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].message, "valid_frame");
        assert_eq!(frames[1].message, "another_valid");
    }
}
