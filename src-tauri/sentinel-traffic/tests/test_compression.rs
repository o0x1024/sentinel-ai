//! 测试响应体压缩和解压功能

use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;

#[test]
fn test_gzip_compression_detection() {
    // 准备测试数据
    let original_text = "Hello, World! This is a test of gzip compression.".repeat(10);
    
    // 使用 gzip 压缩
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(original_text.as_bytes()).unwrap();
    let compressed = encoder.finish().unwrap();
    
    // 验证压缩有效（压缩后应该更小）
    println!("Original size: {} bytes", original_text.len());
    println!("Compressed size: {} bytes", compressed.len());
    assert!(compressed.len() < original_text.len());
    
    // 模拟解压过程（在实际应用中由 proxy.rs 完成）
    use flate2::read::GzDecoder;
    use std::io::Read;
    
    let mut decoder = GzDecoder::new(&compressed[..]);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).unwrap();
    
    // 验证解压后数据正确
    assert_eq!(decompressed, original_text.as_bytes());
    println!("✓ Gzip compression and decompression works correctly");
}

#[test]
fn test_brotli_compression() {
    use brotli::enc::BrotliEncoderParams;
    
    let original_text = "Brotli compression test data.".repeat(10);
    let original_bytes = original_text.as_bytes();
    
    // Brotli 压缩
    let mut compressed = Vec::new();
    let params = BrotliEncoderParams::default();
    brotli::BrotliCompress(
        &mut &original_bytes[..],
        &mut compressed,
        &params,
    ).unwrap();
    
    println!("Original size: {} bytes", original_bytes.len());
    println!("Brotli compressed size: {} bytes", compressed.len());
    
    // Brotli 解压
    use brotli::Decompressor;
    use std::io::Read;
    
    let mut decompressor = Decompressor::new(&compressed[..], 4096);
    let mut decompressed = Vec::new();
    decompressor.read_to_end(&mut decompressed).unwrap();
    
    assert_eq!(decompressed, original_bytes);
    println!("✓ Brotli compression and decompression works correctly");
}

#[test]
fn test_large_gzip_response() {
    // 模拟大型 JSON 响应
    let large_json = r#"{"data": ["#.to_string() + 
        &(0..1000).map(|i| format!(r#"{{"id": {}, "value": "test"}}"#, i))
        .collect::<Vec<_>>()
        .join(",") + 
        "]}";
    
    // 压缩
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(large_json.as_bytes()).unwrap();
    let compressed = encoder.finish().unwrap();
    
    println!("Large JSON size: {} bytes", large_json.len());
    println!("Compressed size: {} bytes", compressed.len());
    println!("Compression ratio: {:.2}%", (compressed.len() as f64 / large_json.len() as f64) * 100.0);
    
    // 解压
    use flate2::read::GzDecoder;
    use std::io::Read;
    
    let mut decoder = GzDecoder::new(&compressed[..]);
    let mut decompressed = String::new();
    decoder.read_to_string(&mut decompressed).unwrap();
    
    assert_eq!(decompressed, large_json);
    println!("✓ Large gzip response handled correctly");
}
