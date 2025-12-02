//! 阿里云 DashScope 文件上传工具
//!
//! 实现通过 DashScope API 获取上传凭证并上传文件到 OSS 的功能。
//! 上传后的文件有效期为 48 小时。

use anyhow::{Context, Result};
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;
use tracing::{debug, info};

use super::global_proxy::apply_proxy_to_client;

const DASHSCOPE_UPLOADS_URL: &str = "https://dashscope.aliyuncs.com/api/v1/uploads";
const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// 上传策略响应数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadPolicyData {
    pub upload_host: String,
    pub upload_dir: String,
    pub oss_access_key_id: String,
    pub signature: String,
    pub policy: String,
    pub x_oss_object_acl: String,
    pub x_oss_forbid_overwrite: String,
}

/// DashScope API 响应
#[derive(Debug, Deserialize)]
struct DashScopeResponse {
    data: UploadPolicyData,
}

/// 上传结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadResult {
    /// OSS 文件 URL (oss://...)
    pub oss_url: String,
    /// 文件名
    pub file_name: String,
    /// 过期时间描述
    pub expires_in: String,
}

/// 获取文件上传凭证
pub async fn get_upload_policy(api_key: &str, model_name: &str) -> Result<UploadPolicyData> {
    let client = {
        let builder = reqwest::Client::builder().timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS));
        let builder = apply_proxy_to_client(builder).await;
        builder.build().context("Failed to build HTTP client")?
    };

    let response = client
        .get(DASHSCOPE_UPLOADS_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .query(&[("action", "getPolicy"), ("model", model_name)])
        .send()
        .await
        .context("Failed to request upload policy")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Failed to get upload policy: status={}, body={}", status, body);
    }

    let resp: DashScopeResponse = response
        .json()
        .await
        .context("Failed to parse upload policy response")?;

    debug!("Got upload policy: upload_host={}, upload_dir={}", resp.data.upload_host, resp.data.upload_dir);
    Ok(resp.data)
}

/// 上传文件到 OSS
pub async fn upload_file_to_oss(policy: &UploadPolicyData, file_path: &Path) -> Result<String> {
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .context("Invalid file name")?
        .to_string();

    let key = format!("{}/{}", policy.upload_dir, file_name);

    let file_bytes = tokio::fs::read(file_path)
        .await
        .context("Failed to read file")?;

    let file_part = Part::bytes(file_bytes)
        .file_name(file_name.clone())
        .mime_str("application/octet-stream")?;

    let form = Form::new()
        .text("OSSAccessKeyId", policy.oss_access_key_id.clone())
        .text("Signature", policy.signature.clone())
        .text("policy", policy.policy.clone())
        .text("x-oss-object-acl", policy.x_oss_object_acl.clone())
        .text("x-oss-forbid-overwrite", policy.x_oss_forbid_overwrite.clone())
        .text("key", key.clone())
        .text("success_action_status", "200")
        .part("file", file_part);

    let client = {
        let builder = reqwest::Client::builder().timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS));
        let builder = apply_proxy_to_client(builder).await;
        builder.build().context("Failed to build HTTP client")?
    };

    let response = client
        .post(&policy.upload_host)
        .multipart(form)
        .send()
        .await
        .context("Failed to upload file to OSS")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Failed to upload file: status={}, body={}", status, body);
    }

    let oss_url = format!("oss://{}", key);
    info!("File uploaded successfully: {} -> {}", file_name, oss_url);
    Ok(oss_url)
}

/// 上传文件并获取 OSS URL
pub async fn upload_file_and_get_url(
    api_key: &str,
    model_name: &str,
    file_path: &Path,
) -> Result<UploadResult> {
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .context("Invalid file name")?
        .to_string();

    // 获取上传凭证
    let policy = get_upload_policy(api_key, model_name).await?;

    // 上传文件到 OSS
    let oss_url = upload_file_to_oss(&policy, file_path).await?;

    Ok(UploadResult {
        oss_url,
        file_name,
        expires_in: "48 hours".to_string(),
    })
}

/// 测试 DashScope 连接
pub async fn test_dashscope_connection(api_key: &str, model_name: &str) -> Result<bool> {
    match get_upload_policy(api_key, model_name).await {
        Ok(_) => {
            info!("DashScope connection test successful");
            Ok(true)
        }
        Err(e) => {
            debug!("DashScope connection test failed: {}", e);
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[tokio::test]
    #[ignore] // Need API key
    async fn test_get_upload_policy() {
        let api_key = "sk-8303958eff134cefbdb56460b5fa97c3";
        let res = upload_file_and_get_url(&api_key, "qwen-vl-plus", Path::new("test.jpg")).await;
        println!("res: {:?}", res);
    }
}

//测试命令是什么
// cargo test --test aliyun_oss::tests::test_get_upload_policy