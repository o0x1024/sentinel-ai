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
        
        // 先占位，稍后基于实际内容计算hash
        let mut source = DocumentSource {
            id: Uuid::new_v4().to_string(),
            file_path: file_path.to_string(),
            file_name,
            file_type: file_type.clone(),
            file_size,
            file_hash: String::new(),
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
        // 基于原始提取内容计算hash（避免空内容导致0大小与空hash）
        source.file_hash = format!("{:x}", md5::compute(content.as_bytes()));
        
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
                    text_content.push_str(&text);
                    if in_code_block {
                        text_content.push('\n'); // 代码块内容保持换行
                    } else {
                        text_content.push(' '); // 普通文本用空格分隔
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

    /// 文本分块（语义分块：优先按句子边界聚合，无法命中时回退硬切分，保证始终前进）
    fn chunk_text(
        &self,
        content: &str,
        file_path: &str,
        source: &DocumentSource,
    ) -> Result<Vec<DocumentChunk>> {
        let mut chunks = Vec::new();
        let max_chunk_chars = self.config.chunk_size_chars;
        let overlap_chars_cfg = self.config.chunk_overlap_chars;

        // 限制有效重叠，避免步长为0
        let effective_overlap = if max_chunk_chars > 0 {
            std::cmp::min(overlap_chars_cfg, max_chunk_chars.saturating_sub(1))
        } else { 0 };

        // 将字符串转换为字符向量，基于字符安全地计算偏移
        let chars: Vec<char> = content.chars().collect();
        let total_len = chars.len();

        if total_len == 0 || max_chunk_chars == 0 {
            return Ok(chunks);
        }

        // 预分句：返回基于字符索引的 [start, end) 区间
        let sentences = Self::split_into_sentences(&chars);

        // 若文档极短，直接返回一个分块
        if total_len <= max_chunk_chars {
            let chunk_content: String = chars[0..total_len].iter().collect();
            let chunk = self.create_chunk(&chunk_content, 0, 0, total_len, file_path, source)?;
            chunks.push(chunk);
            return Ok(chunks);
        }

        // 游标式推进，优先选句子结尾，找不到则硬切分
        let mut chunk_index = 0usize;
        let mut cursor = 0usize;

        while cursor < total_len {
            let target_end = std::cmp::min(cursor + max_chunk_chars, total_len);

            // 寻找不超过 target_end 的最近句子结束
            let mut end_char = cursor;
            for &(_, s_end) in &sentences {
                if s_end > cursor && s_end <= target_end {
                    end_char = s_end; // 取最后一个满足条件的句末
                }
                if s_end > target_end { break; }
            }

            if end_char <= cursor {
                // 未找到合适的句子边界，回退为硬切分
                end_char = target_end;
            }

            // 再次兜底，确保有进展
            if end_char <= cursor {
                end_char = std::cmp::min(cursor + max_chunk_chars, total_len);
                if end_char <= cursor { break; }
            }

            // 构建分块
            let chunk_str: String = chars[cursor..end_char].iter().collect();
            let chunk = self.create_chunk(&chunk_str, chunk_index, cursor, end_char, file_path, source)?;
            chunks.push(chunk);
            chunk_index += 1;

            if end_char >= total_len { break; }

            // 计算下一游标，按重叠回退，但必须保证前进
            let mut next_cursor = end_char.saturating_sub(effective_overlap);
            if next_cursor <= cursor {
                let step = max_chunk_chars.saturating_sub(effective_overlap).max(1);
                next_cursor = std::cmp::min(cursor + step, total_len);
            }
            if next_cursor <= cursor { break; }
            cursor = next_cursor;
        }

        Ok(chunks)
    }

    /// 将字符数组按句子边界切分，返回 (start_char, end_char) 半开区间
    fn split_into_sentences(chars: &[char]) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        let mut start = 0usize;
        let mut i = 0usize;
        while i < chars.len() {
            let c = chars[i];
            let is_end_punct = matches!(
                c,
                '.' | '!' | '?' | '。' | '！' | '？'
            );
            if is_end_punct {
                let mut j = i + 1;
                // 吞并尾随的引号或右括号等
                while j < chars.len() {
                    let c2 = chars[j];
                    if matches!(c2, '"' | '\'' | '”' | '’' | ')' | '）' | ']' | '】') {
                        i = j;
                        j += 1;
                    } else if c2 == ' ' { // 跳过紧随的空格
                        j += 1;
                    } else {
                        break;
                    }
                }
                let end = i + 1;
                if end > start {
                    out.push((start, end));
                }
                start = end;
            }
            i += 1;
        }
        if start < chars.len() {
            out.push((start, chars.len()));
        }
        out
    }

    /// 创建文档分块
    fn create_chunk(
        &self,
        content: &str,
        chunk_index: usize,
        start_char: usize,
        end_char: usize,
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
            // 记录真实起止偏移（基于字符）
            chunk_start_char: start_char,
            chunk_end_char: end_char,
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
            special_chars_regex: Regex::new(r"[^\w\s\u4e00-\u9fff.,;:()\x22\x27\-/=&%+#@\[\]{}!?！？。，；：（）【】《》\u201c\u201d\u2018\u2019—…]").unwrap(),
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
    use crate::rag::models::IngestionStatusEnum;

    #[test]
    fn test_chunk_text_no_punctuation_long_text() {
        let mut config = RagConfig::default();
        config.chunk_size_chars = 50;
        config.chunk_overlap_chars = 10;
        let chunker = DocumentChunker::new(config);

        // 构造无标点长文本（超过两个chunk长度）
        let base = "abcdefghijklmnopqrstuvwxyz"; // 26 chars
        let content = format!("{}{}{}{}{}", base, base, base, base, base); // 130 chars

        let source = DocumentSource {
            id: uuid::Uuid::new_v4().to_string(),
            file_path: "memory".to_string(),
            file_name: "mem.txt".to_string(),
            file_type: "txt".to_string(),
            file_size: content.len() as u64,
            file_hash: String::new(),
            chunk_count: 0,
            ingestion_status: IngestionStatusEnum::Processing,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        };

        let chunks = chunker.chunk_text(&content, "memory", &source).unwrap();
        assert!(!chunks.is_empty());
        // 校验单调递增且有前进
        let mut last_end = 0;
        for c in &chunks {
            assert!(c.metadata.chunk_end_char > c.metadata.chunk_start_char);
            assert!(c.metadata.chunk_end_char > last_end);
            last_end = c.metadata.chunk_end_char;
        }
        assert_eq!(last_end, content.chars().count());
    }
    fn test_text_cleaner() {
        let cleaner = TextCleaner::new();
        let input = "  这是一个   测试文本！！！   包含多余空格和特殊字符@#$%  ";
        let expected = "这是一个 测试文本！！！ 包含多余空格和特殊字符@#%";
        let result = cleaner.clean_text(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_text_cleaner_with_urls() {
        let cleaner = TextCleaner::new();
        let input = "访问 https://example.com/path?param=value&other=123 获取更多信息";
        let expected = "访问 https://example.com/path?param=value&other=123 获取更多信息";
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