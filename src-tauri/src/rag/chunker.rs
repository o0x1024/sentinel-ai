use anyhow::{anyhow, Result};
use log::{debug, info, warn};
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

use crate::rag::config::{RagConfig, SupportedFileType};
use crate::rag::models::{DocumentChunk, ChunkMetadata, DocumentSource};

/// 文档分块器
pub struct DocumentChunker {
    config: RagConfig,
    text_cleaner: TextCleaner,
}

impl DocumentChunker {
    pub fn new(config: RagConfig) -> Self {
        Self {
            config,
            text_cleaner: TextCleaner::new(),
        }
    }

    /// 处理文档并生成分块
    pub async fn process_document(
        &self,
        file_path: &str,
    ) -> Result<(DocumentSource, Vec<DocumentChunk>)> {
        info!("开始处理文档: {}", file_path);
        
        // 创建文档源
        let path = Path::new(file_path);
        let file_name = path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        let file_type = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        let file_size = std::fs::metadata(file_path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        
        let file_hash = format!("{:x}", md5::compute(file_path.as_bytes()));
        
        let mut source = DocumentSource {
            id: Uuid::new_v4().to_string(),
            file_path: file_path.to_string(),
            file_name,
            file_type: file_type.clone(),
            file_size,
            file_hash,
            chunk_count: 0, // 将在分块后更新
            ingestion_status: crate::rag::models::IngestionStatusEnum::Processing,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            metadata: HashMap::new(),
        };
        
        // 检测文件类型
        let supported_file_type = self.detect_file_type(file_path)?;
        
        // 提取文本内容
        let content = self.extract_text_content(file_path, &supported_file_type).await?;
        
        // 清洗文本
        let cleaned_content = self.text_cleaner.clean_text(&content);
        
        // 分块处理
        let chunks = self.chunk_text(&cleaned_content, file_path, &source)?;
        
        // 更新分块数量
         source.chunk_count = chunks.len();
         source.ingestion_status = crate::rag::models::IngestionStatusEnum::Completed;
         source.updated_at = chrono::Utc::now();
        
        info!("文档处理完成，生成 {} 个分块", chunks.len());
        Ok((source, chunks))
    }

    /// 检测文件类型
    fn detect_file_type(&self, file_path: &str) -> Result<SupportedFileType> {
        let path = Path::new(file_path);
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| anyhow!("无法获取文件扩展名"))?
            .to_lowercase();

        match extension.as_str() {
            "txt" => Ok(SupportedFileType::Txt),
            "md" => Ok(SupportedFileType::Md),
            "pdf" => Ok(SupportedFileType::Pdf),
            "docx" => Ok(SupportedFileType::Docx),
            _ => Err(anyhow!("不支持的文件类型: {}", extension)),
        }
    }

    /// 提取文本内容
    async fn extract_text_content(
        &self,
        file_path: &str,
        file_type: &SupportedFileType,
    ) -> Result<String> {
        match file_type {
            SupportedFileType::Txt => self.extract_txt_content(file_path).await,
            SupportedFileType::Md => self.extract_md_content(file_path).await,
            SupportedFileType::Pdf => self.extract_pdf_content(file_path).await,
            SupportedFileType::Docx => self.extract_word_content(file_path).await,
        }
    }

    /// 提取TXT文件内容
    async fn extract_txt_content(&self, file_path: &str) -> Result<String> {
        tokio::fs::read_to_string(file_path)
            .await
            .map_err(|e| anyhow!("读取TXT文件失败: {}", e))
    }

    /// 提取Markdown文件内容
    async fn extract_md_content(&self, file_path: &str) -> Result<String> {
        let content = tokio::fs::read_to_string(file_path)
            .await
            .map_err(|e| anyhow!("读取MD文件失败: {}", e))?;
        
        // 使用pulldown-cmark解析Markdown并提取纯文本
        use pulldown_cmark::{Parser, Event, Tag, TagEnd};
        
        let parser = Parser::new(&content);
        let mut text_content = String::new();
        let mut in_code_block = false;
        
        for event in parser {
            match event {
                Event::Start(Tag::CodeBlock(_)) => in_code_block = true,
                Event::End(TagEnd::CodeBlock) => in_code_block = false,
                Event::Text(text) => {
                    if !in_code_block {
                        text_content.push_str(&text);
                        text_content.push(' ');
                    }
                }
                Event::SoftBreak | Event::HardBreak => {
                    text_content.push('\n');
                }
                _ => {}
            }
        }
        
        Ok(text_content)
    }

    /// 提取PDF文件内容
    async fn extract_pdf_content(&self, file_path: &str) -> Result<String> {
        // 暂时返回占位符，实际实现需要pdf-extract crate
        warn!("PDF解析功能暂未完全实现");
        Ok(format!("PDF文件内容占位符: {}", file_path))
    }

    /// 提取Word文件内容
    async fn extract_word_content(&self, file_path: &str) -> Result<String> {
        // 暂时返回占位符，实际实现需要docx-rs crate
        warn!("Word文档解析功能暂未完全实现");
        Ok(format!("Word文档内容占位符: {}", file_path))
    }

    /// 文本分块
    fn chunk_text(
        &self,
        content: &str,
        file_path: &str,
        source: &DocumentSource,
    ) -> Result<Vec<DocumentChunk>> {
        let mut chunks = Vec::new();
        let chunk_size = self.config.chunk_size_chars;
        let overlap = self.config.chunk_overlap_chars;
        
        // 将字符串转换为字符向量，以便安全地按字符数切片
        let chars: Vec<char> = content.chars().collect();
        
        if chars.len() <= chunk_size {
            // 内容较短，直接作为一个分块
            let chunk = self.create_chunk(content, 0, file_path, source)?;
            chunks.push(chunk);
            return Ok(chunks);
        }

        let mut start = 0;
        let mut chunk_index = 0;

        while start < chars.len() {
            let end = std::cmp::min(start + chunk_size, chars.len());
            
            // 从字符向量中提取子切片并转换回字符串
            let chunk_chars: String = chars[start..end].iter().collect();
            let chunk_content = chunk_chars.as_str();
            
            let chunk = self.create_chunk(chunk_content, chunk_index, file_path, source)?;
            chunks.push(chunk);
            
            chunk_index += 1;
            
            // 计算下一个分块的起始位置（考虑重叠）
            if end >= chars.len() {
                break;
            }
            start = end - overlap;
        }

        Ok(chunks)
    }

    /// 创建文档分块
    fn create_chunk(
        &self,
        content: &str,
        chunk_index: usize,
        file_path: &str,
        source: &DocumentSource,
    ) -> Result<DocumentChunk> {
        let chunk_id = Uuid::new_v4().to_string();
        let content_hash = format!("{:x}", md5::compute(content.as_bytes()));
        
        let path = Path::new(file_path);
        let file_name = path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        let file_type = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        let file_size = std::fs::metadata(file_path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        
        let metadata = ChunkMetadata {
            file_path: file_path.to_string(),
            file_name,
            file_type,
            file_size,
            chunk_start_char: chunk_index * self.config.chunk_size_chars,
            chunk_end_char: chunk_index * self.config.chunk_size_chars + content.len(),
            page_number: None,
            section_title: None,
            custom_fields: HashMap::new(),
        };

        Ok(DocumentChunk {
            id: chunk_id,
            source_id: source.id.clone(),
            content: content.to_string(),
            content_hash,
            chunk_index,
            metadata,
            embedding: None, // 嵌入向量将在后续步骤中生成
            created_at: chrono::Utc::now(),
        })
    }
}

/// 文本清洗器
pub struct TextCleaner {
    whitespace_regex: Regex,
    special_chars_regex: Regex,
}

impl TextCleaner {
    pub fn new() -> Self {
        Self {
            whitespace_regex: Regex::new(r"\s+").unwrap(),
            special_chars_regex: Regex::new(r"[^\w\s\u4e00-\u9fff.,!?;:()\x22\x27\-]").unwrap(),
        }
    }

    /// 清洗文本内容
    pub fn clean_text(&self, text: &str) -> String {
        let mut cleaned = text.to_string();
        
        // 移除多余的空白字符
        cleaned = self.whitespace_regex.replace_all(&cleaned, " ").to_string();
        
        // 移除特殊字符（保留中英文、数字和基本标点）
        cleaned = self.special_chars_regex.replace_all(&cleaned, "").to_string();
        
        // 去除首尾空白
        cleaned = cleaned.trim().to_string();
        
        debug!("文本清洗完成，原长度: {}, 清洗后长度: {}", text.len(), cleaned.len());
        
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rag::config::RagConfig;

    #[test]
    fn test_text_cleaner() {
        let cleaner = TextCleaner::new();
        let input = "  这是一个   测试文本！！！   包含多余空格和特殊字符@#$%  ";
        let expected = "这是一个 测试文本！！！ 包含多余空格和特殊字符";
        let result = cleaner.clean_text(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_detect_file_type() {
        let config = RagConfig::default();
        let chunker = DocumentChunker::new(config);
        
        assert!(matches!(chunker.detect_file_type("test.txt"), Ok(SupportedFileType::Txt)));
        assert!(matches!(chunker.detect_file_type("test.md"), Ok(SupportedFileType::Md)));
        assert!(matches!(chunker.detect_file_type("test.pdf"), Ok(SupportedFileType::Pdf)));
        assert!(matches!(chunker.detect_file_type("test.docx"), Ok(SupportedFileType::Docx)));
        assert!(chunker.detect_file_type("test.unknown").is_err());
    }
}