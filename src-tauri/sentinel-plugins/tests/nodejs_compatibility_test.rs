//! Node.js Compatibility Layer Tests
//!
//! Tests to verify that the Node.js compatibility layer works correctly

use sentinel_plugins::plugin_engine::PluginEngine;
use sentinel_plugins::types::PluginMetadata;

#[tokio::test]
async fn test_nodejs_require_fs() {
    let mut engine = PluginEngine::new().expect("Failed to create engine");

    let code = r#"
        const fs = require('fs');
        
        export async function analyze(input) {
            // Test that fs module is available
            if (!fs.promises) {
                throw new Error('fs.promises not available');
            }
            
            return { success: true, message: 'fs module loaded' };
        }
        
        globalThis.analyze = analyze;
    "#;

    let metadata = PluginMetadata {
        id: "test_nodejs_fs".to_string(),
        name: "Test Node.js fs".to_string(),
        version: "1.0.0".to_string(),
        author: Some("Test".to_string()),
        description: Some("Test Node.js fs module".to_string()),
        main_category: "agent".to_string(),
        category: "test".to_string(),
        tags: vec![],
        default_severity: sentinel_plugins::types::Severity::Info,
    };

    engine
        .load_plugin_with_metadata(code, metadata)
        .await
        .expect("Failed to load plugin");

    let input = serde_json::json!({ "test": "data" });
    let (findings, result) = engine
        .execute_agent(&input)
        .await
        .expect("Failed to execute plugin");

    assert_eq!(findings.len(), 0);
    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result["success"], true);
}

#[tokio::test]
async fn test_nodejs_require_path() {
    let mut engine = PluginEngine::new().expect("Failed to create engine");

    let code = r#"
        const path = require('path');
        
        export async function analyze(input) {
            const joined = path.join('/foo', 'bar', 'baz.txt');
            const basename = path.basename(joined);
            const dirname = path.dirname(joined);
            const extname = path.extname(joined);
            
            return {
                success: true,
                joined,
                basename,
                dirname,
                extname
            };
        }
        
        globalThis.analyze = analyze;
    "#;

    let metadata = PluginMetadata {
        id: "test_nodejs_path".to_string(),
        name: "Test Node.js path".to_string(),
        version: "1.0.0".to_string(),
        author: Some("Test".to_string()),
        description: Some("Test Node.js path module".to_string()),
        main_category: "agent".to_string(),
        category: "test".to_string(),
        tags: vec![],
        default_severity: sentinel_plugins::types::Severity::Info,
    };

    engine
        .load_plugin_with_metadata(code, metadata)
        .await
        .expect("Failed to load plugin");

    let input = serde_json::json!({});
    let (_findings, result) = engine
        .execute_agent(&input)
        .await
        .expect("Failed to execute plugin");

    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result["joined"], "/foo/bar/baz.txt");
    assert_eq!(result["basename"], "baz.txt");
    assert_eq!(result["dirname"], "/foo/bar");
    assert_eq!(result["extname"], ".txt");
}

#[tokio::test]
async fn test_nodejs_buffer() {
    let mut engine = PluginEngine::new().expect("Failed to create engine");

    let code = r#"
        export async function analyze(input) {
            // Test Buffer.from
            const buf1 = Buffer.from('hello');
            const buf2 = Buffer.from([0x68, 0x65, 0x6c, 0x6c, 0x6f]);
            const buf3 = Buffer.from('aGVsbG8=', 'base64');
            
            // Test Buffer.alloc
            const buf4 = Buffer.alloc(5, 0x61);
            
            // Test Buffer.concat
            const buf5 = Buffer.concat([buf1, buf2]);
            
            // Test toString
            const str1 = buf1.toString();
            const str2 = buf1.toString('hex');
            const str3 = buf1.toString('base64');
            
            return {
                success: true,
                str1,
                str2,
                str3,
                buf4_str: buf4.toString(),
                buf5_length: buf5.length
            };
        }
        
        globalThis.analyze = analyze;
    "#;

    let metadata = PluginMetadata {
        id: "test_nodejs_buffer".to_string(),
        name: "Test Node.js Buffer".to_string(),
        version: "1.0.0".to_string(),
        author: Some("Test".to_string()),
        description: Some("Test Node.js Buffer".to_string()),
        main_category: "agent".to_string(),
        category: "test".to_string(),
        tags: vec![],
        default_severity: sentinel_plugins::types::Severity::Info,
    };

    engine
        .load_plugin_with_metadata(code, metadata)
        .await
        .expect("Failed to load plugin");

    let input = serde_json::json!({});
    let (_findings, result) = engine
        .execute_agent(&input)
        .await
        .expect("Failed to execute plugin");

    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result["str1"], "hello");
    assert_eq!(result["str2"], "68656c6c6f");
    assert_eq!(result["buf4_str"], "aaaaa");
    assert_eq!(result["buf5_length"], 10);
}

#[tokio::test]
async fn test_nodejs_process() {
    let mut engine = PluginEngine::new().expect("Failed to create engine");

    let code = r#"
        export async function analyze(input) {
            return {
                success: true,
                platform: process.platform,
                arch: process.arch,
                pid: process.pid,
                version: process.version
            };
        }
        
        globalThis.analyze = analyze;
    "#;

    let metadata = PluginMetadata {
        id: "test_nodejs_process".to_string(),
        name: "Test Node.js process".to_string(),
        version: "1.0.0".to_string(),
        author: Some("Test".to_string()),
        description: Some("Test Node.js process object".to_string()),
        main_category: "agent".to_string(),
        category: "test".to_string(),
        tags: vec![],
        default_severity: sentinel_plugins::types::Severity::Info,
    };

    engine
        .load_plugin_with_metadata(code, metadata)
        .await
        .expect("Failed to load plugin");

    let input = serde_json::json!({});
    let (_findings, result) = engine
        .execute_agent(&input)
        .await
        .expect("Failed to execute plugin");

    assert!(result.is_some());
    let result = result.unwrap();
    assert!(result["platform"].is_string());
    assert!(result["arch"].is_string());
}

#[tokio::test]
async fn test_nodejs_crypto() {
    let mut engine = PluginEngine::new().expect("Failed to create engine");

    let code = r#"
        const crypto = require('crypto');
        
        export async function analyze(input) {
            // Test randomBytes
            const random = crypto.randomBytes(16);
            
            // Test hash (sha256)
            const hash = crypto.createHash('sha256');
            hash.update('hello world');
            const digest = await hash.digest('hex');
            
            // Test randomUUID
            const uuid = crypto.randomUUID();
            
            return {
                success: true,
                random_length: random.length,
                digest,
                uuid_length: uuid.length
            };
        }
        
        globalThis.analyze = analyze;
    "#;

    let metadata = PluginMetadata {
        id: "test_nodejs_crypto".to_string(),
        name: "Test Node.js crypto".to_string(),
        version: "1.0.0".to_string(),
        author: Some("Test".to_string()),
        description: Some("Test Node.js crypto module".to_string()),
        main_category: "agent".to_string(),
        category: "test".to_string(),
        tags: vec![],
        default_severity: sentinel_plugins::types::Severity::Info,
    };

    engine
        .load_plugin_with_metadata(code, metadata)
        .await
        .expect("Failed to load plugin");

    let input = serde_json::json!({});
    let (_findings, result) = engine
        .execute_agent(&input)
        .await
        .expect("Failed to execute plugin");

    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result["random_length"], 16);
    assert_eq!(result["digest"], "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
    assert_eq!(result["uuid_length"], 36);
}
