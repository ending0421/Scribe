use crate::pipeline::{PipelineStage, LogBatch, Fallback};
use crate::Result;
use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};

pub struct EncryptStage {
    cipher: ChaCha20Poly1305,
    nonce_counter: std::sync::atomic::AtomicU64,
}

impl EncryptStage {
    pub fn new(key: &[u8; 32]) -> Self {
        let cipher = ChaCha20Poly1305::new(key.into());
        Self {
            cipher,
            nonce_counter: std::sync::atomic::AtomicU64::new(0),
        }
    }

    fn next_nonce(&self) -> [u8; 12] {
        let counter = self
            .nonce_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut nonce = [0u8; 12];
        nonce[0..8].copy_from_slice(&counter.to_le_bytes());
        nonce
    }
}

impl PipelineStage for EncryptStage {
    fn name(&self) -> &str {
        "encrypt"
    }

    fn process(&self, data: LogBatch) -> Result<LogBatch> {
        let nonce_bytes = self.next_nonce();
        let nonce = Nonce::from_slice(&nonce_bytes);

        match self.cipher.encrypt(nonce, data.data.as_ref()) {
            Ok(ciphertext) => {
                // 将 nonce 和密文一起返回
                let mut result = nonce_bytes.to_vec();
                result.extend_from_slice(&ciphertext);
                Ok(LogBatch::new(result))
            }
            Err(e) => Err(crate::ScribeError::Encryption(e.to_string())),
        }
    }

    fn on_error(&self, data: LogBatch, _error: crate::ScribeError) -> Fallback {
        // 加密失败时跳过，使用原始数据（但应该告警）
        eprintln!("WARNING: Encryption failed, using unencrypted data");
        Fallback::Skip
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = [0u8; 32];
        let stage = EncryptStage::new(&key);

        let data = b"sensitive data".to_vec();
        let batch = LogBatch::new(data.clone());

        let encrypted = stage.process(batch).unwrap();

        // 验证数据已加密（不同于原始数据）
        assert_ne!(&encrypted.data[12..], &data[..]);

        // 解密验证
        let nonce_bytes: [u8; 12] = encrypted.data[0..12].try_into().unwrap();
        let nonce = Nonce::from_slice(&nonce_bytes);
        let cipher = ChaCha20Poly1305::new(&key.into());
        let decrypted = cipher.decrypt(nonce, &encrypted.data[12..]).unwrap();

        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_wrong_key_decrypt_fails() {
        let key1 = [1u8; 32];
        let key2 = [2u8; 32];
        let stage = EncryptStage::new(&key1);

        let data = b"secret message".to_vec();
        let batch = LogBatch::new(data.clone());

        let encrypted = stage.process(batch).unwrap();

        // 尝试用错误的密钥解密
        let nonce_bytes: [u8; 12] = encrypted.data[0..12].try_into().unwrap();
        let nonce = Nonce::from_slice(&nonce_bytes);
        let wrong_cipher = ChaCha20Poly1305::new(&key2.into());
        let result = wrong_cipher.decrypt(nonce, &encrypted.data[12..]);

        assert!(result.is_err());
    }

    #[test]
    fn test_nonce_increments() {
        let key = [0u8; 32];
        let stage = EncryptStage::new(&key);

        let batch1 = LogBatch::new(b"message 1".to_vec());
        let batch2 = LogBatch::new(b"message 2".to_vec());

        let encrypted1 = stage.process(batch1).unwrap();
        let encrypted2 = stage.process(batch2).unwrap();

        // 提取 nonce
        let nonce1: [u8; 12] = encrypted1.data[0..12].try_into().unwrap();
        let nonce2: [u8; 12] = encrypted2.data[0..12].try_into().unwrap();

        // nonce 应该不同
        assert_ne!(nonce1, nonce2);

        // nonce 应该是递增的
        let counter1 = u64::from_le_bytes(nonce1[0..8].try_into().unwrap());
        let counter2 = u64::from_le_bytes(nonce2[0..8].try_into().unwrap());
        assert_eq!(counter2, counter1 + 1);
    }

    #[test]
    fn test_empty_data_encryption() {
        let key = [0u8; 32];
        let stage = EncryptStage::new(&key);

        let data = vec![];
        let batch = LogBatch::new(data.clone());

        let encrypted = stage.process(batch).unwrap();

        // 应该包含 nonce + tag
        assert!(encrypted.data.len() >= 12);

        // 解密验证
        let nonce_bytes: [u8; 12] = encrypted.data[0..12].try_into().unwrap();
        let nonce = Nonce::from_slice(&nonce_bytes);
        let cipher = ChaCha20Poly1305::new(&key.into());
        let decrypted = cipher.decrypt(nonce, &encrypted.data[12..]).unwrap();

        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_large_data_encryption() {
        let key = [0u8; 32];
        let stage = EncryptStage::new(&key);

        // 1MB 数据
        let data = vec![42u8; 1024 * 1024];
        let batch = LogBatch::new(data.clone());

        let encrypted = stage.process(batch).unwrap();

        // 解密验证
        let nonce_bytes: [u8; 12] = encrypted.data[0..12].try_into().unwrap();
        let nonce = Nonce::from_slice(&nonce_bytes);
        let cipher = ChaCha20Poly1305::new(&key.into());
        let decrypted = cipher.decrypt(nonce, &encrypted.data[12..]).unwrap();

        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_on_error() {
        let key = [0u8; 32];
        let stage = EncryptStage::new(&key);
        let data = vec![1, 2, 3];
        let batch = LogBatch::new(data);
        let error = crate::ScribeError::Encryption("test error".to_string());

        let fallback = stage.on_error(batch, error);
        assert!(matches!(fallback, Fallback::Skip));
    }

    #[test]
    fn test_stage_name() {
        let key = [0u8; 32];
        let stage = EncryptStage::new(&key);
        assert_eq!(stage.name(), "encrypt");
    }

    #[test]
    fn test_deterministic_nonce_generation() {
        let key = [0u8; 32];
        let stage = EncryptStage::new(&key);

        // 生成第一个 nonce
        let nonce1 = stage.next_nonce();
        let counter1 = u64::from_le_bytes(nonce1[0..8].try_into().unwrap());

        // 剩余字节应该是 0
        assert_eq!(&nonce1[8..], &[0u8; 4]);
        assert_eq!(counter1, 0);

        // 第二个 nonce
        let nonce2 = stage.next_nonce();
        let counter2 = u64::from_le_bytes(nonce2[0..8].try_into().unwrap());
        assert_eq!(counter2, 1);
    }
}
