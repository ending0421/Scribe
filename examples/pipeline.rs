//! Pipeline 使用示例

use scribe::pipeline::{Pipeline, LogBatch};
use scribe::stages::{CompressStage, EncryptStage};

fn main() {
    println!("Pipeline Example");

    // 创建数据
    let data = b"Hello, Scribe! This is a test message that will be compressed and encrypted.";
    let batch = LogBatch::new(data.to_vec());

    println!("Original size: {} bytes", batch.size());

    // 创建 Pipeline
    let pipeline = Pipeline::new().add_stage(Box::new(CompressStage::zstd(3)));
    // .add_stage(Box::new(EncryptStage::new(&[0u8; 32])));  // 需要密钥

    // 处理数据
    match pipeline.process(batch) {
        Ok(result) => {
            println!("Processed size: {} bytes", result.size());
            println!("Compression ratio: {:.2}x",
                     data.len() as f32 / result.size() as f32);
        }
        Err(e) => {
            eprintln!("Pipeline error: {}", e);
        }
    }
}
