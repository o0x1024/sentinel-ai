use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tokio::fs;

use base64::Engine as _;
use rig::tool::Tool;
use sentinel_tools::buildin_tools::ocr::{OcrArgs, OcrTool};
use sentinel_tools::buildin_tools::shell::get_shell_config;
use sentinel_tools::docker_sandbox::DockerSandbox;
use sentinel_tools::output_storage::CONTAINER_CONTEXT_DIR;

#[derive(Debug, Clone)]
pub struct ImageOcrResult {
    pub source_path: String,
    pub filename: Option<String>,
    pub text: String,
    pub from_cache: bool,
}

#[derive(Debug, Clone)]
pub struct ImageDockerPath {
    pub filename: Option<String>,
    pub container_path: String,
}

fn cache_dir() -> Result<PathBuf, String> {
    let dir = dirs::data_dir()
        .ok_or_else(|| "Failed to resolve data directory".to_string())?
        .join("sentinel-ai")
        .join("cache")
        .join("ocr");
    Ok(dir)
}

fn tmp_dir() -> Result<PathBuf, String> {
    Ok(cache_dir()?.join("tmp"))
}

async fn sha256_hex_of_file(path: &str) -> Result<String, String> {
    let bytes = fs::read(path)
        .await
        .map_err(|e| format!("Failed to read image file for hashing: {}", e))?;
    let digest = Sha256::digest(&bytes);
    let mut s = String::with_capacity(digest.len() * 2);
    for b in digest.iter() {
        s.push_str(&format!("{:02x}", b));
    }
    Ok(s)
}

fn sha256_hex_of_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut s = String::with_capacity(digest.len() * 2);
    for b in digest.iter() {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

async fn read_cache_text(hash_hex: &str) -> Option<String> {
    let dir = cache_dir().ok()?;
    let path = dir.join(format!("{}.txt", hash_hex));
    let text = fs::read_to_string(path).await.ok()?;
    let trimmed = text.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

async fn write_cache_text(hash_hex: &str, text: &str) {
    let Ok(dir) = cache_dir() else { return };
    let _ = fs::create_dir_all(&dir).await;
    let path = dir.join(format!("{}.txt", hash_hex));
    let _ = fs::write(path, text).await;
}

fn ext_from_media_type(media_type: Option<&str>) -> Option<&'static str> {
    let mt = media_type?.to_lowercase();
    match mt.as_str() {
        "jpeg" | "jpg" | "image/jpeg" => Some("jpg"),
        "png" | "image/png" => Some("png"),
        "gif" | "image/gif" => Some("gif"),
        "webp" | "image/webp" => Some("webp"),
        _ => None,
    }
}

fn ext_from_filename(filename: Option<&str>) -> Option<String> {
    let name = filename?;
    let ext = std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())?;
    if ext.is_empty() { None } else { Some(ext) }
}

fn normalize_local_path(p: &str) -> String {
    // Handle "file://..." paths if any
    p.strip_prefix("file://").unwrap_or(p).to_string()
}

async fn ensure_tmp_file(hash_hex: &str, ext: &str, bytes: &[u8]) -> Result<String, String> {
    let dir = tmp_dir()?;
    fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("Failed to create OCR tmp dir: {}", e))?;
    let path = dir.join(format!("{}.{}", hash_hex, ext));
    if fs::metadata(&path).await.is_err() {
        fs::write(&path, bytes)
            .await
            .map_err(|e| format!("Failed to write OCR tmp image: {}", e))?;
    }
    Ok(path.to_string_lossy().to_string())
}

fn get_image_base64(img: &serde_json::Value) -> Option<String> {
    img.get("data").and_then(|d| {
        if d.is_string() {
            d.as_str().map(|s| s.to_string())
        } else {
            d.get("data").and_then(|x| x.as_str()).map(|s| s.to_string())
        }
    })
}

async fn prepare_local_image_file_for_attachment(
    img: &serde_json::Value,
) -> Result<(String, String, String, Option<String>), String> {
    // Returns (hash_hex, host_path, ext, filename)
    let filename = get_string_field(img, "filename");
    let media_type = get_string_field(img, "media_type");
    let ext = ext_from_filename(filename.as_deref())
        .or_else(|| ext_from_media_type(media_type.as_deref()).map(|s| s.to_string()))
        .unwrap_or_else(|| "png".to_string());

    if let Some(sp) = get_string_field(img, "source_path") {
        let sp = normalize_local_path(&sp);
        if fs::metadata(&sp).await.is_ok() {
            let hash = sha256_hex_of_file(&sp).await?;
            return Ok((hash, sp, ext, filename));
        }
        tracing::warn!("Image source_path not readable, falling back to base64: {}", sp);
    }

    let b64 = get_image_base64(img).ok_or_else(|| "Missing image data".to_string())?;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(b64.as_bytes())
        .map_err(|e| format!("Failed to decode image base64: {}", e))?;
    let hash = sha256_hex_of_bytes(&decoded);
    let tmp_path = ensure_tmp_file(&hash, &ext, &decoded).await?;
    Ok((hash, tmp_path, ext, filename))
}

pub async fn stage_images_to_docker_context(
    attachments: &serde_json::Value,
) -> Result<Vec<ImageDockerPath>, String> {
    let Some(arr) = attachments.as_array() else {
        return Ok(vec![]);
    };

    let shell_cfg = get_shell_config().await;
    let docker_cfg = shell_cfg.docker_config.unwrap_or_default();
    let sandbox = DockerSandbox::new(docker_cfg);

    let mut out = Vec::new();
    for item in arr {
        let Some(img) = as_image_object(item) else { continue };
        let (hash, host_path, ext, filename) = match prepare_local_image_file_for_attachment(img).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Failed to prepare local image file for docker staging: {}", e);
                continue;
            }
        };

        let container_path = format!("{}/attachments/{}.{}", CONTAINER_CONTEXT_DIR, hash, ext);
        sandbox
            .copy_file_to_container(&host_path, &container_path)
            .await
            .map_err(|e| format!("Failed to copy image to docker container: {}", e))?;

        out.push(ImageDockerPath {
            filename,
            container_path,
        });
    }

    Ok(out)
}

pub async fn ocr_image_file(source_path: &str, filename: Option<String>) -> Result<ImageOcrResult, String> {
    let source_path = normalize_local_path(source_path);
    let hash = sha256_hex_of_file(&source_path).await?;
    if let Some(text) = read_cache_text(&hash).await {
        return Ok(ImageOcrResult {
            source_path,
            filename,
            text,
            from_cache: true,
        });
    }

    let tool = OcrTool;
    let output = tool
        .call(OcrArgs {
            image_path: source_path.clone(),
        })
        .await
        .map_err(|e| format!("OCR failed: {}", e))?;

    let text = output.text.trim().to_string();
    if !text.is_empty() {
        write_cache_text(&hash, &text).await;
    }

    Ok(ImageOcrResult {
        source_path,
        filename,
        text,
        from_cache: false,
    })
}

async fn ocr_image_bytes(
    bytes: &[u8],
    filename: Option<String>,
    media_type: Option<String>,
) -> Result<ImageOcrResult, String> {
    let hash = sha256_hex_of_bytes(bytes);
    if let Some(text) = read_cache_text(&hash).await {
        return Ok(ImageOcrResult {
            source_path: format!("memory:{}", &hash[..16]),
            filename,
            text,
            from_cache: true,
        });
    }

    let ext = ext_from_filename(filename.as_deref())
        .or_else(|| ext_from_media_type(media_type.as_deref()).map(|s| s.to_string()))
        .unwrap_or_else(|| "png".to_string());

    let tmp_path = ensure_tmp_file(&hash, &ext, bytes).await?;
    let tool = OcrTool;
    let output = tool
        .call(OcrArgs {
            image_path: tmp_path.clone(),
        })
        .await
        .map_err(|e| format!("OCR failed: {}", e))?;

    let text = output.text.trim().to_string();
    if !text.is_empty() {
        write_cache_text(&hash, &text).await;
    }

    Ok(ImageOcrResult {
        source_path: tmp_path,
        filename,
        text,
        from_cache: false,
    })
}

fn get_string_field(v: &serde_json::Value, key: &str) -> Option<String> {
    v.get(key).and_then(|x| x.as_str()).map(|s| s.to_string())
}

fn as_image_object(v: &serde_json::Value) -> Option<&serde_json::Value> {
    // Expected: { type: "image", ... }
    if v.get("type").and_then(|t| t.as_str()) == Some("image") {
        return Some(v);
    }
    // Legacy-ish: { image: { ... } }
    v.get("image")
}

pub async fn ocr_images_from_attachments(
    attachments: &serde_json::Value,
) -> Result<Vec<ImageOcrResult>, String> {
    let Some(arr) = attachments.as_array() else {
        return Ok(vec![]);
    };

    let mut results = Vec::new();
    for item in arr {
        let Some(img) = as_image_object(item) else { continue };
        let filename = get_string_field(img, "filename");
        let source_path = get_string_field(img, "source_path");

        if let Some(sp) = source_path {
            match ocr_image_file(&sp, filename.clone()).await {
                Ok(r) => {
                    if !r.text.trim().is_empty() {
                        results.push(r);
                    }
                    continue;
                }
                Err(e) => {
                    tracing::warn!("Image OCR failed for source_path '{}': {}", sp, e);
                    // Fall through to base64 fallback
                }
            }
        }

        // Fallback: decode base64 and OCR via temp file
        let media_type = get_string_field(img, "media_type");
        let base64_data = img
            .get("data")
            .and_then(|d| {
                if d.is_string() {
                    d.as_str().map(|s| s.to_string())
                } else {
                    d.get("data").and_then(|x| x.as_str()).map(|s| s.to_string())
                }
            });

        let Some(b64) = base64_data else { continue };
        let decoded = match base64::engine::general_purpose::STANDARD.decode(b64.as_bytes()) {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!("Failed to decode image base64 for OCR: {}", e);
                continue;
            }
        };

        match ocr_image_bytes(&decoded, filename.clone(), media_type).await {
            Ok(r) => {
                if !r.text.trim().is_empty() {
                    results.push(r);
                }
            }
            Err(e) => {
                tracing::warn!("Image OCR failed for base64 attachment: {}", e);
            }
        }
    }
    Ok(results)
}

pub fn format_ocr_context(results: &[ImageOcrResult], max_chars: usize) -> String {
    if results.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    for (idx, r) in results.iter().enumerate() {
        let title = r
            .filename
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("image");
        if idx > 0 {
            out.push_str("\n\n");
        }
        out.push_str(title);
        out.push_str(":\n");
        out.push_str(&r.text);
    }
    if out.len() > max_chars {
        out.truncate(max_chars);
        out.push_str("\n... [truncated]");
    }
    out
}

