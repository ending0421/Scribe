use scribe::{LogFrame, LogLevel};
use std::fs::{File, remove_file};
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

mod common;
use common::create_test_recovery;

#[test]
fn test_crash_recovery_normal_case() {
    let temp_dir = TempDir::new().unwrap();
    let recovery = create_test_recovery(&temp_dir);

    // 创建包含有效日志帧的 mmap 文件，模拟正常日志
    let mmap_path = temp_dir.path().join("crash_test.mmap");
    let mut file = File::create(&mmap_path).unwrap();

    let frames = vec![
        LogFrame::new(LogLevel::Info, "app".to_string(), "Application started".to_string()),
        LogFrame::new(LogLevel::Debug, "network".to_string(), "Connecting to server".to_string()),
        LogFrame::new(LogLevel::Error, "auth".to_string(), "Authentication failed".to_string()),
    ];

    for frame in &frames {
        file.write_all(&frame.serialize().unwrap()).unwrap();
    }
    drop(file);

    // 恢复日志
    let recovered = recovery.validate_and_recover(&mmap_path).unwrap();

    assert_eq!(recovered.len(), 3);
    assert_eq!(recovered[0].level, LogLevel::Info);
    assert_eq!(recovered[0].tag, "app");
    assert_eq!(recovered[0].message, "Application started");

    assert_eq!(recovered[1].level, LogLevel::Debug);
    assert_eq!(recovered[2].level, LogLevel::Error);
}

#[test]
fn test_crash_recovery_partial_corruption() {
    let temp_dir = TempDir::new().unwrap();
    let recovery = create_test_recovery(&temp_dir);

    let mmap_path = temp_dir.path().join("partial_corruption.mmap");
    let mut file = File::create(&mmap_path).unwrap();

    // 写入有效帧1
    let frame1 = LogFrame::new(LogLevel::Info, "valid1".to_string(), "First valid frame".to_string());
    file.write_all(&frame1.serialize().unwrap()).unwrap();

    // 写入损坏数据（模拟崩溃时的不完整写入）
    let corrupted_data = vec![0xFF, 0xAB, 0xCD, 0xEF, 0x00, 0x11, 0x22, 0x33];
    file.write_all(&corrupted_data).unwrap();

    // 写入有效帧2
    let frame2 = LogFrame::new(LogLevel::Warn, "valid2".to_string(), "Second valid frame".to_string());
    file.write_all(&frame2.serialize().unwrap()).unwrap();

    // 再次写入损坏数据
    file.write_all(&vec![0xDE; 100]).unwrap();

    // 写入有效帧3
    let frame3 = LogFrame::new(LogLevel::Error, "valid3".to_string(), "Third valid frame".to_string());
    file.write_all(&frame3.serialize().unwrap()).unwrap();

    drop(file);

    // 恢复应该跳过损坏部分，找到所有有效帧
    let recovered = recovery.validate_and_recover(&mmap_path).unwrap();

    assert_eq!(recovered.len(), 3);
    assert_eq!(recovered[0].message, "First valid frame");
    assert_eq!(recovered[1].message, "Second valid frame");
    assert_eq!(recovered[2].message, "Third valid frame");
}

#[test]
fn test_crash_recovery_crc_corruption() {
    let temp_dir = TempDir::new().unwrap();
    let recovery = create_test_recovery(&temp_dir);

    let mmap_path = temp_dir.path().join("crc_corruption.mmap");
    let mut file = File::create(&mmap_path).unwrap();

    // 写入有效帧
    let frame1 = LogFrame::new(LogLevel::Info, "good".to_string(), "Valid frame".to_string());
    file.write_all(&frame1.serialize().unwrap()).unwrap();

    // 写入 CRC 损坏的帧
    let frame2 = LogFrame::new(LogLevel::Error, "bad".to_string(), "Corrupted CRC frame".to_string());
    let mut serialized = frame2.serialize().unwrap();
    let len = serialized.len();
    serialized[len - 1] ^= 0xFF; // 篡改 CRC
    serialized[len - 2] ^= 0xAA;
    file.write_all(&serialized).unwrap();

    // 写入另一个有效帧
    let frame3 = LogFrame::new(LogLevel::Debug, "good2".to_string(), "Another valid frame".to_string());
    file.write_all(&frame3.serialize().unwrap()).unwrap();

    drop(file);

    // 恢复应该跳过 CRC 损坏的帧
    let recovered = recovery.validate_and_recover(&mmap_path).unwrap();

    assert_eq!(recovered.len(), 2);
    assert_eq!(recovered[0].tag, "good");
    assert_eq!(recovered[1].tag, "good2");
}

#[test]
fn test_crash_recovery_incomplete_frame() {
    let temp_dir = TempDir::new().unwrap();
    let recovery = create_test_recovery(&temp_dir);

    let mmap_path = temp_dir.path().join("incomplete.mmap");
    let mut file = File::create(&mmap_path).unwrap();

    // 写入完整帧
    let frame1 = LogFrame::new(LogLevel::Info, "complete".to_string(), "Complete frame".to_string());
    file.write_all(&frame1.serialize().unwrap()).unwrap();

    // 写入不完整的帧（只写一半，模拟崩溃时的中断）
    let frame2 = LogFrame::new(LogLevel::Error, "incomplete".to_string(), "This frame is cut off".to_string());
    let serialized = frame2.serialize().unwrap();
    let half_len = serialized.len() / 2;
    file.write_all(&serialized[..half_len]).unwrap();

    drop(file);

    // 恢复应该只找到完整的帧
    let recovered = recovery.validate_and_recover(&mmap_path).unwrap();

    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].tag, "complete");
}

#[test]
fn test_crash_recovery_multi_file() {
    let temp_dir = TempDir::new().unwrap();
    let recovery = create_test_recovery(&temp_dir);

    // 创建多个 mmap 文件，模拟多次运行后的残留文件
    for i in 0..5 {
        let mmap_path = temp_dir.path().join(format!("buffer_{}.mmap", i));
        let mut file = File::create(&mmap_path).unwrap();

        for j in 0..3 {
            let frame = LogFrame::new(
                LogLevel::Info,
                format!("file{}", i),
                format!("Message {} from file {}", j, i),
            );
            file.write_all(&frame.serialize().unwrap()).unwrap();
        }
    }

    // 恢复所有文件
    let report = recovery.recover_all().unwrap();

    assert_eq!(report.files_processed, 5);
    assert_eq!(report.frames_recovered, 15); // 5 files * 3 frames each
    assert_eq!(report.frames_corrupted, 0);
    assert!(report.bytes_recovered > 0);

    // 验证 mmap 文件已被删除
    let remaining = recovery.scan_mmap_files().unwrap();
    assert_eq!(remaining.len(), 0);

    // 验证创建了恢复日志
    let entries: Vec<_> = std::fs::read_dir(temp_dir.path())
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
fn test_crash_recovery_mixed_corruption() {
    let temp_dir = TempDir::new().unwrap();
    let recovery = create_test_recovery(&temp_dir);

    let mmap_path = temp_dir.path().join("mixed.mmap");
    let mut file = File::create(&mmap_path).unwrap();

    // 1. 有效帧
    let frame1 = LogFrame::new(LogLevel::Info, "valid1".to_string(), "First".to_string());
    file.write_all(&frame1.serialize().unwrap()).unwrap();

    // 2. CRC 损坏
    let frame2 = LogFrame::new(LogLevel::Debug, "crc_bad".to_string(), "Bad CRC".to_string());
    let mut ser2 = frame2.serialize().unwrap();
    ser2[ser2.len() - 1] ^= 0xFF;
    file.write_all(&ser2).unwrap();

    // 3. 有效帧
    let frame3 = LogFrame::new(LogLevel::Warn, "valid2".to_string(), "Second".to_string());
    file.write_all(&frame3.serialize().unwrap()).unwrap();

    // 4. 随机损坏数据
    file.write_all(&vec![0xAB; 50]).unwrap();

    // 5. 有效帧
    let frame5 = LogFrame::new(LogLevel::Error, "valid3".to_string(), "Third".to_string());
    file.write_all(&frame5.serialize().unwrap()).unwrap();

    // 6. 不完整帧
    let frame6 = LogFrame::new(LogLevel::Info, "incomplete".to_string(), "Cut".to_string());
    let ser6 = frame6.serialize().unwrap();
    file.write_all(&ser6[..ser6.len() / 3]).unwrap();

    drop(file);

    // 恢复应该找到 3 个有效帧
    let recovered = recovery.validate_and_recover(&mmap_path).unwrap();

    assert_eq!(recovered.len(), 3);
    assert_eq!(recovered[0].tag, "valid1");
    assert_eq!(recovered[1].tag, "valid2");
    assert_eq!(recovered[2].tag, "valid3");
}

#[test]
fn test_crash_recovery_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let recovery = create_test_recovery(&temp_dir);

    let mmap_path = temp_dir.path().join("empty.mmap");
    File::create(&mmap_path).unwrap();

    let recovered = recovery.validate_and_recover(&mmap_path).unwrap();
    assert_eq!(recovered.len(), 0);
}

#[test]
fn test_crash_recovery_completely_corrupted() {
    let temp_dir = TempDir::new().unwrap();
    let recovery = create_test_recovery(&temp_dir);

    let mmap_path = temp_dir.path().join("all_corrupt.mmap");
    let mut file = File::create(&mmap_path).unwrap();

    // 写入完全随机的数据
    let random_data: Vec<u8> = (0..2048).map(|i| (i * 13 % 256) as u8).collect();
    file.write_all(&random_data).unwrap();
    drop(file);

    let recovered = recovery.validate_and_recover(&mmap_path).unwrap();
    assert_eq!(recovered.len(), 0);
}

#[test]
fn test_crash_recovery_report_accuracy() {
    let temp_dir = TempDir::new().unwrap();
    let recovery = create_test_recovery(&temp_dir);

    let mmap_path = temp_dir.path().join("report_test.mmap");
    let mut file = File::create(&mmap_path).unwrap();

    let mut expected_bytes = 0u64;

    // 写入 10 个有效帧
    for i in 0..10 {
        let frame = LogFrame::new(
            LogLevel::Info,
            "test".to_string(),
            format!("Message number {}", i),
        );
        let serialized = frame.serialize().unwrap();
        expected_bytes += serialized.len() as u64;
        file.write_all(&serialized).unwrap();
    }

    // 写入损坏数据
    file.write_all(&vec![0xFF; 200]).unwrap();

    drop(file);

    // 生成报告
    let report = recovery.recover_file(&mmap_path).unwrap();

    assert_eq!(report.files_processed, 1);
    assert_eq!(report.frames_recovered, 10);
    assert_eq!(report.bytes_recovered, expected_bytes);
}

#[test]
fn test_crash_recovery_large_frame_rejection() {
    let temp_dir = TempDir::new().unwrap();
    let recovery = create_test_recovery(&temp_dir);

    let mmap_path = temp_dir.path().join("large_frame.mmap");
    let mut file = File::create(&mmap_path).unwrap();

    // 写入有效帧
    let frame1 = LogFrame::new(LogLevel::Info, "valid".to_string(), "Valid".to_string());
    file.write_all(&frame1.serialize().unwrap()).unwrap();

    // 写入声称长度过大的伪帧头
    const MAGIC_HEADER: u32 = 0xFEEDC0DE;
    let magic_bytes = MAGIC_HEADER.to_le_bytes();
    let invalid_length = (10 * 1024 * 1024u32).to_le_bytes(); // 10MB，超过 1MB 限制
    file.write_all(&magic_bytes).unwrap();
    file.write_all(&invalid_length).unwrap();
    file.write_all(&vec![0u8; 100]).unwrap();

    // 写入另一个有效帧
    let frame2 = LogFrame::new(LogLevel::Error, "valid2".to_string(), "Valid2".to_string());
    file.write_all(&frame2.serialize().unwrap()).unwrap();

    drop(file);

    // 恢复应该跳过声称过大的帧
    let recovered = recovery.validate_and_recover(&mmap_path).unwrap();

    assert_eq!(recovered.len(), 2);
    assert_eq!(recovered[0].tag, "valid");
    assert_eq!(recovered[1].tag, "valid2");
}

#[test]
fn test_crash_recovery_no_files() {
    let temp_dir = TempDir::new().unwrap();
    let recovery = create_test_recovery(&temp_dir);

    let report = recovery.recover_all().unwrap();

    assert_eq!(report.files_processed, 0);
    assert_eq!(report.frames_recovered, 0);
    assert_eq!(report.frames_corrupted, 0);
    assert_eq!(report.bytes_recovered, 0);
}

#[test]
fn test_crash_recovery_with_non_mmap_files() {
    let temp_dir = TempDir::new().unwrap();
    let recovery = create_test_recovery(&temp_dir);

    // 创建一些非 .mmap 文件
    File::create(temp_dir.path().join("test.log")).unwrap();
    File::create(temp_dir.path().join("data.txt")).unwrap();

    // 创建一个 .mmap 文件
    let mmap_path = temp_dir.path().join("valid.mmap");
    let mut file = File::create(&mmap_path).unwrap();
    let frame = LogFrame::new(LogLevel::Info, "test".to_string(), "Message".to_string());
    file.write_all(&frame.serialize().unwrap()).unwrap();
    drop(file);

    // 扫描应该只找到 .mmap 文件
    let mmap_files = recovery.scan_mmap_files().unwrap();
    assert_eq!(mmap_files.len(), 1);

    // 恢复应该只处理 .mmap 文件
    let report = recovery.recover_all().unwrap();
    assert_eq!(report.files_processed, 1);
    assert_eq!(report.frames_recovered, 1);
}
