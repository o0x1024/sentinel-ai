//! OCR tool using rig-core Tool trait and oar-ocr crate

use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use oar_ocr::prelude::*;
use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use std::io::Write;

/// OCR arguments
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct OcrArgs {
    /// Path to the image file
    pub image_path: String,
}

/// OCR result
#[derive(Debug, Clone, Serialize)]
pub struct OcrOutput {
    /// Detected text
    pub text: String,
}

/// OCR errors
#[derive(Debug, thiserror::Error)]
pub enum OcrError {
    #[error("Image not found: {0}")]
    ImageNotFound(String),
    #[error("Failed to load image: {0}")]
    LoadError(String),
    #[error("OCR engine error: {0}")]
    EngineError(String),
    #[error("Model error: {0}")]
    ModelError(String),
}

/// OCR tool
#[derive(Debug, Clone, Default)]
pub struct OcrTool;

impl OcrTool {
    pub const NAME: &'static str = "ocr";
    pub const DESCRIPTION: &'static str = "Extract text from an image file using OCR (Optical Character Recognition). Supports Chinese and English text.";

    fn get_model_path(model_name: &str) -> Result<PathBuf> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| anyhow!("Could not find data directory"))?
            .join("sentinel-ai")
            .join("models")
            .join("oar-ocr");

        if !data_dir.exists() {
            fs::create_dir_all(&data_dir)?;
        }

        let model_path = data_dir.join(model_name);
        Ok(model_path)
    }

    async fn download_model(model_name: &str, url: &str) -> Result<PathBuf, OcrError> {
        let path = Self::get_model_path(model_name)
            .map_err(|e| OcrError::ModelError(e.to_string()))?;
        
        // If file exists, check if it's valid
        if path.exists() {
            let metadata = fs::metadata(&path)
                .map_err(|e| OcrError::ModelError(format!("Failed to check model metadata: {}", e)))?;
            
            // Different minimum sizes for different file types
            // Dictionary files (.txt) are small (usually 50-100KB)
            // Model files (.onnx) should be at least 1MB
            let min_size = if model_name.ends_with(".txt") {
                10 * 1024  // 10KB minimum for dictionary files
            } else {
                1024 * 1024  // 1MB minimum for model files
            };
            
            if metadata.len() > min_size {
                return Ok(path);
            } else {
                tracing::warn!("File {} is too small ({} bytes, minimum: {}), re-downloading...", 
                    model_name, metadata.len(), min_size);
                let _ = fs::remove_file(&path);
            }
        }

        tracing::info!("Downloading OCR model: {} from {}", model_name, url);
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // 5 minutes timeout
            .build()
            .map_err(|e| OcrError::ModelError(format!("Failed to create HTTP client: {}", e)))?;

        let response = client.get(url)
            .header("User-Agent", "Sentinel-AI/1.0")
            .send()
            .await
            .map_err(|e| OcrError::ModelError(format!("Failed to start download: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(OcrError::ModelError(format!(
                "Failed to download model {}: HTTP status {}", 
                model_name, 
                response.status()
            )));
        }

        let mut file = fs::File::create(&path)
            .map_err(|e| OcrError::ModelError(format!("Failed to create model file: {}", e)))?;
        
        let mut stream = response.bytes_stream();
        while let Some(item) = stream.next().await {
            let chunk = item.map_err(|e| OcrError::ModelError(format!("Error during download: {}", e)))?;
            file.write_all(&chunk)
                .map_err(|e| OcrError::ModelError(format!("Failed to write to file: {}", e)))?;
        }

        // Final check on downloaded file size
        let metadata = fs::metadata(&path)
            .map_err(|e| OcrError::ModelError(format!("Failed to check downloaded model metadata: {}", e)))?;
        
        // Different minimum sizes for different file types
        let min_size = if model_name.ends_with(".txt") {
            10 * 1024  // 10KB minimum for dictionary files
        } else {
            1024 * 1024  // 1MB minimum for model files
        };
        
        if metadata.len() < min_size {
            let _ = fs::remove_file(&path);
            return Err(OcrError::ModelError(format!(
                "Downloaded file {} is invalid (size: {} bytes, minimum: {} bytes)", 
                model_name, 
                metadata.len(),
                min_size
            )));
        }

        tracing::info!("Successfully downloaded OCR model: {}", model_name);
        Ok(path)
    }

    async fn ensure_models(&self) -> Result<(PathBuf, PathBuf, PathBuf), OcrError> {
        // PaddleOCR v5 models from oar-ocr GitHub releases - supports Chinese and English
        // Using official oar-ocr release mirrors for reliable downloads
        let base_url = "https://github.com/GreatV/oar-ocr/releases/download/v0.3.0";
        
        let detection_url = format!("{}/pp-ocrv5_mobile_det.onnx", base_url);
        let recognition_url = format!("{}/pp-ocrv5_mobile_rec.onnx", base_url);
        let dict_url = format!("{}/ppocrv5_dict.txt", base_url);

        let det_path = Self::download_model("pp-ocrv5_mobile_det.onnx", &detection_url).await?;
        let rec_path = Self::download_model("pp-ocrv5_mobile_rec.onnx", &recognition_url).await?;
        let dict_path = Self::download_model("ppocrv5_dict.txt", &dict_url).await?;

        Ok((det_path, rec_path, dict_path))
    }

    fn run_ocr(&self, image_path: &str, detection_path: PathBuf, recognition_path: PathBuf, dict_path: PathBuf) -> Result<String, OcrError> {
        // Build OAR OCR engine
        let ocr = OAROCRBuilder::new(
            detection_path.to_str().ok_or_else(|| OcrError::ModelError("Invalid detection path".to_string()))?,
            recognition_path.to_str().ok_or_else(|| OcrError::ModelError("Invalid recognition path".to_string()))?,
            dict_path.to_str().ok_or_else(|| OcrError::ModelError("Invalid dict path".to_string()))?,
        )
        .build()
        .map_err(|e| OcrError::EngineError(e.to_string()))?;

        // Load image
        let image = load_image(std::path::Path::new(image_path))
            .map_err(|e| OcrError::LoadError(e.to_string()))?;

        // Run OCR prediction
        let results = ocr.predict(vec![image])
            .map_err(|e| OcrError::EngineError(e.to_string()))?;

        // Extract text from results
        let mut text_lines = Vec::new();
        for ocr_result in &results {
            for text_region in &ocr_result.text_regions {
                if let Some((text, _confidence)) = text_region.text_with_confidence() {
                    text_lines.push(text.to_string());
                }
            }
        }

        Ok(text_lines.join("\n"))
    }
}

impl Tool for OcrTool {
    const NAME: &'static str = Self::NAME;
    type Args = OcrArgs;
    type Output = OcrOutput;
    type Error = OcrError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(OcrArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let image_path = args.image_path;
        
        // Ensure models are downloaded (async)
        let (det_path, rec_path, dict_path) = self.ensure_models().await?;
        
        // Run in blocking context because ML inference is CPU intensive
        let text = tokio::task::spawn_blocking(move || {
            let tool = OcrTool::default();
            tool.run_ocr(&image_path, det_path, rec_path, dict_path)
        })
        .await
        .map_err(|e| OcrError::EngineError(format!("Task execution failed: {}", e)))??;

        Ok(OcrOutput { text })
    }
}
