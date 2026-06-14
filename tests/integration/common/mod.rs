use scribe::storage::recovery::Recovery;
use tempfile::TempDir;
use std::path::PathBuf;
use flate2::write::GzEncoder;
use flate2::Compression;
use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};

/// 创建测试用的 Recovery 实例
pub fn create_test_recovery(temp_dir: &TempDir) -> Recovery {
    Recovery::new(temp_dir.path().to_path_buf())
}

/// 压缩数据辅助函数
pub fn compress_data(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    use std::io::Write;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    encoder.finish()
}

/// 加密数据辅助函数（简化版，用于测试）
pub fn encrypt_data(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // 使用固定密钥用于测试
    let key = [0u8; 32];
    let cipher = ChaCha20Poly1305::new(&key.into());

    let nonce_bytes = [0u8; 12];
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher.encrypt(nonce, data)
        .map_err(|e| format!("Encryption failed: {:?}", e))?;

    Ok(ciphertext)
}

/// 解密数据辅助函数（简化版，用于测试）
#[allow(dead_code)]
pub fn decrypt_data(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let key = [0u8; 32];
    let cipher = ChaCha20Poly1305::new(&key.into());

    let nonce_bytes = [0u8; 12];
    let nonce = Nonce::from_slice(&nonce_bytes);

    let plaintext = cipher.decrypt(nonce, data)
        .map_err(|e| format!("Decryption failed: {:?}", e))?;

    Ok(plaintext)
}
