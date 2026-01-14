//! Document attachment processing commands

use crate::models::attachment::{DocumentAttachment, DocumentProcessingMode};
use sentinel_tools::shell::get_shell_config;
use sentinel_tools::DockerSandbox;
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

/// Docker status for file analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerAnalysisStatus {
    pub docker_available: bool,
    pub image_exists: bool,
    pub container_ready: bool,
    pub ready_for_file_analysis: bool,
    pub supported_file_types: Vec<String>,
    pub error_message: Option<String>,
}

/// Processed document result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedDocumentResult {
    pub id: String,
    pub original_filename: String,
    pub file_size: u64,
    pub mime_type: String,
    pub processing_mode: String,
    pub status: String,
    pub extracted_text: Option<String>,
    pub container_path: Option<String>,
    pub extraction_method: Option<String>,
    pub error_message: Option<String>,
}

/// File stat result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStatResult {
    pub size: u64,
    pub is_file: bool,
    pub is_dir: bool,
}

/// Get file stat (size, type)
#[tauri::command]
pub async fn get_file_stat(path: String) -> Result<FileStatResult, String> {
    let metadata = std::fs::metadata(&path)
        .map_err(|e| format!("Failed to get file metadata: {}", e))?;
    
    Ok(FileStatResult {
        size: metadata.len(),
        is_file: metadata.is_file(),
        is_dir: metadata.is_dir(),
    })
}

/// Check Docker availability for file analysis
#[tauri::command]
pub async fn check_docker_for_file_analysis() -> Result<DockerAnalysisStatus, String> {
    let docker_available = DockerSandbox::is_docker_available().await;
    
    if !docker_available {
        return Ok(DockerAnalysisStatus {
            docker_available: false,
            image_exists: false,
            container_ready: false,
            ready_for_file_analysis: false,
            supported_file_types: DocumentAttachment::SUPPORTED_EXTENSIONS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            error_message: Some("Docker is not available".to_string()),
        });
    }

    let image_exists = DockerSandbox::image_exists("sentinel-sandbox:latest").await;
    
    if !image_exists {
        return Ok(DockerAnalysisStatus {
            docker_available: true,
            image_exists: false,
            container_ready: false,
            ready_for_file_analysis: false,
            supported_file_types: DocumentAttachment::SUPPORTED_EXTENSIONS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            error_message: Some("Sandbox image not found. Please build it first.".to_string()),
        });
    }

    // Check if container is ready
    let container_ready = DockerSandbox::get_persistent_shell_container_info()
        .await
        .map(|info| info.is_some())
        .unwrap_or(false);

    Ok(DockerAnalysisStatus {
        docker_available: true,
        image_exists: true,
        container_ready,
        ready_for_file_analysis: true,
        supported_file_types: DocumentAttachment::SUPPORTED_EXTENSIONS
            .iter()
            .map(|s| s.to_string())
            .collect(),
        error_message: None,
    })
}

/// Process document attachment based on mode
#[tauri::command]
pub async fn process_document_attachment(
    file_path: String,
    mode: String, // "content" or "security"
) -> Result<ProcessedDocumentResult, String> {
    let path = Path::new(&file_path);
    
    // Validate file exists
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    // Get file info
    let metadata = std::fs::metadata(&file_path)
        .map_err(|e| format!("Failed to get file metadata: {}", e))?;
    let file_size = metadata.len();
    
    let original_filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // 所有文件类型都支持，未知类型作为文本处理
    let mime_type = DocumentAttachment::mime_type_from_extension(&extension).to_string();
    let id = Uuid::new_v4().to_string();

    let processing_mode = match mode.as_str() {
        "content" => DocumentProcessingMode::Content,
        "security" => DocumentProcessingMode::Security,
        _ => return Err(format!("Invalid processing mode: {}", mode)),
    };

    match processing_mode {
        DocumentProcessingMode::Content => {
            process_content_mode(&id, &file_path, &original_filename, file_size, &mime_type, &extension).await
        }
        DocumentProcessingMode::Security => {
            process_security_mode(&id, &file_path, &original_filename, file_size, &mime_type).await
        }
    }
}

/// Content mode: extract text from document
async fn process_content_mode(
    id: &str,
    file_path: &str,
    original_filename: &str,
    file_size: u64,
    mime_type: &str,
    extension: &str,
) -> Result<ProcessedDocumentResult, String> {
    // Try Docker extraction first (more secure)
    if DockerSandbox::is_docker_available().await 
        && DockerSandbox::image_exists("sentinel-sandbox:latest").await 
    {
        match extract_text_in_docker(file_path, extension).await {
            Ok(text) => {
                return Ok(ProcessedDocumentResult {
                    id: id.to_string(),
                    original_filename: original_filename.to_string(),
                    file_size,
                    mime_type: mime_type.to_string(),
                    processing_mode: "content".to_string(),
                    status: "ready".to_string(),
                    extracted_text: Some(text),
                    container_path: None,
                    extraction_method: Some("docker".to_string()),
                    error_message: None,
                });
            }
            Err(e) => {
                tracing::warn!("Docker extraction failed, falling back to local: {}", e);
            }
        }
    }

    // Fall back to local extraction
    match extract_text_locally(file_path, extension).await {
        Ok(text) => {
            Ok(ProcessedDocumentResult {
                id: id.to_string(),
                original_filename: original_filename.to_string(),
                file_size,
                mime_type: mime_type.to_string(),
                processing_mode: "content".to_string(),
                status: "ready".to_string(),
                extracted_text: Some(text),
                container_path: None,
                extraction_method: Some("local".to_string()),
                error_message: None,
            })
        }
        Err(e) => {
            Ok(ProcessedDocumentResult {
                id: id.to_string(),
                original_filename: original_filename.to_string(),
                file_size,
                mime_type: mime_type.to_string(),
                processing_mode: "content".to_string(),
                status: "failed".to_string(),
                extracted_text: None,
                container_path: None,
                extraction_method: None,
                error_message: Some(e),
            })
        }
    }
}

/// Security mode: transfer file to Docker for analysis
async fn process_security_mode(
    id: &str,
    file_path: &str,
    original_filename: &str,
    file_size: u64,
    mime_type: &str,
) -> Result<ProcessedDocumentResult, String> {
    // Security mode requires Docker
    if !DockerSandbox::is_docker_available().await {
        return Err("Security analysis requires Docker".to_string());
    }

    if !DockerSandbox::image_exists("sentinel-sandbox:latest").await {
        return Err("Security analysis requires sandbox image".to_string());
    }

    // Transfer file to container
    match transfer_file_to_container(file_path, id).await {
        Ok(container_path) => {
            Ok(ProcessedDocumentResult {
                id: id.to_string(),
                original_filename: original_filename.to_string(),
                file_size,
                mime_type: mime_type.to_string(),
                processing_mode: "security".to_string(),
                status: "ready".to_string(),
                extracted_text: None,
                container_path: Some(container_path),
                extraction_method: None,
                error_message: None,
            })
        }
        Err(e) => {
            Ok(ProcessedDocumentResult {
                id: id.to_string(),
                original_filename: original_filename.to_string(),
                file_size,
                mime_type: mime_type.to_string(),
                processing_mode: "security".to_string(),
                status: "failed".to_string(),
                extracted_text: None,
                container_path: None,
                extraction_method: None,
                error_message: Some(e),
            })
        }
    }
}

/// Extract text from document in Docker container
async fn extract_text_in_docker(file_path: &str, extension: &str) -> Result<String, String> {
    let shell_config = get_shell_config().await;
    let docker_config = shell_config.docker_config
        .ok_or_else(|| "Docker config not available".to_string())?;
    
    let sandbox = DockerSandbox::new(docker_config);
    
    // Generate unique filename
    let id = Uuid::new_v4().to_string();
    let original_ext = Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or(extension);
    let container_path = format!("/workspace/uploads/temp_{}_{}.{}", id, "doc", original_ext);
    
    // Copy file to container
    sandbox.copy_file_to_container(file_path, &container_path)
        .await
        .map_err(|e| format!("Failed to copy file to container: {}", e))?;
    
    // Build extraction command based on file type
    let extract_cmd = build_extraction_command(&container_path, extension);
    
    // Execute extraction
    let (stdout, stderr, exit_code) = sandbox.execute(&extract_cmd, 60)
        .await
        .map_err(|e| format!("Failed to execute extraction: {}", e))?;
    
    // Cleanup temp file
    let _ = sandbox.delete_file_in_container(&container_path).await;
    
    if exit_code != 0 {
        return Err(format!("Extraction failed: {}", stderr));
    }
    
    Ok(stdout.trim().to_string())
}

/// Build extraction command based on file type
fn build_extraction_command(file_path: &str, extension: &str) -> String {
    match extension.to_lowercase().as_str() {
        // 纯文本文件：直接读取
        "txt" | "md" | "csv" | "json" | "xml" | "yaml" | "yml" | "toml" | "ini" | "conf" | "cfg" | "log" => {
            format!("cat '{}'", file_path)
        }
        // 代码文件：直接读取
        "js" | "jsx" | "ts" | "tsx" | "py" | "java" | "c" | "cpp" | "h" | "hpp" | "cc" | "cxx" 
        | "rs" | "go" | "rb" | "php" | "sh" | "bash" | "zsh" | "sql" | "html" | "htm" | "css" => {
            format!("cat '{}'", file_path)
        }
        // PDF
        "pdf" => {
            format!("pdftotext '{}' - 2>/dev/null || echo '[PDF extraction failed]'", file_path)
        }
        // Word DOCX
        "docx" => {
            format!(
                r#"python3 -c "
from docx import Document
doc = Document('{}')
for para in doc.paragraphs:
    print(para.text)
" 2>/dev/null || echo '[DOCX extraction failed - python-docx may not be installed]'"#,
                file_path
            )
        }
        // Word DOC (旧格式)
        "doc" => {
            format!("antiword '{}' 2>/dev/null || echo '[DOC extraction failed]'", file_path)
        }
        // Excel
        "xlsx" | "xls" => {
            format!(
                r#"python3 -c "
from openpyxl import load_workbook
wb = load_workbook('{}', read_only=True, data_only=True)
for sheet in wb.sheetnames:
    print(f'=== Sheet: {{sheet}} ===')
    ws = wb[sheet]
    for row in ws.iter_rows(values_only=True):
        print('\t'.join(str(c) if c is not None else '' for c in row))
" 2>/dev/null || echo '[Excel extraction failed]'"#,
                file_path
            )
        }
        // PowerPoint
        "pptx" | "ppt" => {
            format!(
                r#"python3 -c "
from pptx import Presentation
prs = Presentation('{}')
for i, slide in enumerate(prs.slides, 1):
    print(f'=== Slide {{i}} ===')
    for shape in slide.shapes:
        if hasattr(shape, 'text'):
            print(shape.text)
" 2>/dev/null || echo '[PowerPoint extraction failed]'"#,
                file_path
            )
        }
        // Email
        "eml" => {
            format!(
                r#"python3 -c "
import email
from email import policy
with open('{}', 'rb') as f:
    msg = email.message_from_binary_file(f, policy=policy.default)
print('From:', msg.get('From', ''))
print('To:', msg.get('To', ''))
print('Subject:', msg.get('Subject', ''))
print('Date:', msg.get('Date', ''))
print('---')
if msg.is_multipart():
    for part in msg.walk():
        if part.get_content_type() == 'text/plain':
            print(part.get_content())
else:
    print(msg.get_content())
" 2>/dev/null || echo '[Email extraction failed]'"#,
                file_path
            )
        }
        // RTF
        "rtf" => {
            format!("unrtf --text '{}' 2>/dev/null | tail -n +4 || echo '[RTF extraction failed]'", file_path)
        }
        // 未知类型：尝试作为文本读取，失败则用 strings 提取
        _ => {
            format!(
                "cat '{}' 2>/dev/null || (file '{}' && echo '---' && strings '{}' | head -500)",
                file_path, file_path, file_path
            )
        }
    }
}

/// Extract text locally (fallback when Docker unavailable)
async fn extract_text_locally(file_path: &str, extension: &str) -> Result<String, String> {
    match extension.to_lowercase().as_str() {
        // 纯文本文件：直接读取
        "txt" | "md" | "csv" | "json" | "xml" | "yaml" | "yml" | "toml" | "ini" | "conf" | "cfg" | "log" => {
            tokio::fs::read_to_string(file_path)
                .await
                .map_err(|e| format!("Failed to read file: {}", e))
        }
        // 代码文件：直接读取
        "js" | "jsx" | "ts" | "tsx" | "py" | "java" | "c" | "cpp" | "h" | "hpp" | "cc" | "cxx" 
        | "rs" | "go" | "rb" | "php" | "sh" | "bash" | "zsh" | "sql" | "html" | "htm" | "css" => {
            tokio::fs::read_to_string(file_path)
                .await
                .map_err(|e| format!("Failed to read file: {}", e))
        }
        // Office 文档和 PDF 需要 Docker
        "pdf" => {
            Err("PDF extraction requires Docker environment for security".to_string())
        }
        "docx" | "doc" => {
            Err("Word document extraction requires Docker environment for security".to_string())
        }
        "xlsx" | "xls" => {
            Err("Excel extraction requires Docker environment for security".to_string())
        }
        "pptx" | "ppt" => {
            Err("PowerPoint extraction requires Docker environment for security".to_string())
        }
        "eml" | "msg" => {
            Err("Email extraction requires Docker environment for security".to_string())
        }
        "rtf" => {
            Err("RTF extraction requires Docker environment for security".to_string())
        }
        // 压缩文件需要 Docker
        "zip" | "rar" | "7z" | "tar" | "gz" => {
            Err("Archive extraction requires Docker environment for security".to_string())
        }
        // 未知类型：尝试作为文本读取
        _ => {
            tokio::fs::read_to_string(file_path)
                .await
                .map_err(|e| format!("Failed to read file as text: {}. This file type may require Docker.", e))
        }
    }
}

/// Transfer file to Docker container for security analysis
async fn transfer_file_to_container(file_path: &str, id: &str) -> Result<String, String> {
    let shell_config = get_shell_config().await;
    let docker_config = shell_config.docker_config
        .ok_or_else(|| "Docker config not available".to_string())?;
    
    let sandbox = DockerSandbox::new(docker_config);
    
    // Get original extension
    let extension = Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("bin");
    
    // Get sanitized original filename
    let original_name = Path::new(file_path)
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .take(32)
        .collect::<String>();
    
    let container_path = format!("/workspace/uploads/{}_{}.{}", id, original_name, extension);
    
    sandbox.copy_file_to_container(file_path, &container_path)
        .await
        .map_err(|e| format!("Failed to transfer file: {}", e))?;
    
    Ok(container_path)
}

/// Delete uploaded file from container
#[tauri::command]
pub async fn delete_document_from_container(container_path: String) -> Result<(), String> {
    let shell_config = get_shell_config().await;
    let docker_config = shell_config.docker_config
        .ok_or_else(|| "Docker config not available".to_string())?;
    
    let sandbox = DockerSandbox::new(docker_config);
    
    sandbox.delete_file_in_container(&container_path)
        .await
        .map_err(|e| format!("Failed to delete file: {}", e))?;
    
    Ok(())
}

/// List files in container uploads directory
#[tauri::command]
pub async fn list_container_uploads() -> Result<Vec<String>, String> {
    let shell_config = get_shell_config().await;
    let docker_config = shell_config.docker_config
        .ok_or_else(|| "Docker config not available".to_string())?;
    
    let sandbox = DockerSandbox::new(docker_config);
    
    let (stdout, _, exit_code) = sandbox.execute("ls -1 /workspace/uploads 2>/dev/null || echo ''", 10)
        .await
        .map_err(|e| format!("Failed to list files: {}", e))?;
    
    if exit_code != 0 {
        return Ok(vec![]);
    }
    
    let files: Vec<String> = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| format!("/workspace/uploads/{}", l))
        .collect();
    
    Ok(files)
}
