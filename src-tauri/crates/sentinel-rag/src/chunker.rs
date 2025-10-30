use anyhow::{anyhow, Result};
use log::{debug, info, warn};
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

use crate::config::{RagConfig, SupportedFileType, ChunkingStrategy};
use crate::models::{DocumentChunk, ChunkMetadata, DocumentSource, IngestionStatusEnum};

pub struct DocumentChunker {
    config: RagConfig,
    text_cleaner: TextCleaner,
}

impl DocumentChunker {
    pub fn new(config: RagConfig) -> Self { Self { config, text_cleaner: TextCleaner::new() } }

    pub async fn process_document(&self, file_path: &str) -> Result<(DocumentSource, Vec<DocumentChunk>)> {
        info!("开始处理文档: {}", file_path);
        let path = Path::new(file_path);
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();
        let file_type = path.extension().and_then(|e| e.to_str()).unwrap_or("unknown").to_string();
        let file_size = std::fs::metadata(file_path).map(|m| m.len()).unwrap_or(0);
        let mut source = DocumentSource { id: Uuid::new_v4().to_string(), file_path: file_path.to_string(), file_name, file_type: file_type.clone(), file_size, file_hash: String::new(), chunk_count: 0, ingestion_status: IngestionStatusEnum::Processing, created_at: chrono::Utc::now(), updated_at: chrono::Utc::now(), metadata: HashMap::new() };
        let supported_file_type = self.detect_file_type(file_path)?;
        let content = self.extract_text_content(file_path, &supported_file_type).await?;
        let cleaned_content = self.text_cleaner.clean_text(&content);
        source.file_hash = format!("{:x}", md5::compute(content.as_bytes()));
        let chunks = self.chunk_text(&cleaned_content, file_path, &source)?;
        source.chunk_count = chunks.len();
        source.ingestion_status = IngestionStatusEnum::Completed;
        source.updated_at = chrono::Utc::now();
        Ok((source, chunks))
    }

    fn detect_file_type(&self, file_path: &str) -> Result<SupportedFileType> {
        let ext = Path::new(file_path).extension().and_then(|e| e.to_str()).ok_or_else(|| anyhow!("无法获取文件扩展名"))?.to_lowercase();
        match ext.as_str() { "txt" => Ok(SupportedFileType::Txt), "md" => Ok(SupportedFileType::Md), "pdf" => Ok(SupportedFileType::Pdf), "docx" => Ok(SupportedFileType::Docx), _ => Err(anyhow!("不支持的文件类型: {}", ext)) }
    }

    async fn extract_text_content(&self, file_path: &str, file_type: &SupportedFileType) -> Result<String> {
        match file_type { SupportedFileType::Txt => self.extract_txt_content(file_path).await, SupportedFileType::Md => self.extract_md_content(file_path).await, SupportedFileType::Pdf => self.extract_pdf_content(file_path).await, SupportedFileType::Docx => self.extract_word_content(file_path).await }
    }
    async fn extract_txt_content(&self, file_path: &str) -> Result<String> { tokio::fs::read_to_string(file_path).await.map_err(|e| anyhow!("读取TXT文件失败: {}", e)) }
    async fn extract_md_content(&self, file_path: &str) -> Result<String> {
        let content = tokio::fs::read_to_string(file_path).await.map_err(|e| anyhow!("读取MD文件失败: {}", e))?;
        use pulldown_cmark::{Parser, Event, Tag, TagEnd};
        let parser = Parser::new(&content);
        let mut text_content = String::new();
        let mut in_code_block = false;
        for event in parser { match event { Event::Start(Tag::CodeBlock(_)) => in_code_block = true, Event::End(TagEnd::CodeBlock) => in_code_block = false, Event::Text(text) => { text_content.push_str(&text); if in_code_block { text_content.push('\n'); } else { text_content.push(' '); }}, Event::SoftBreak | Event::HardBreak => text_content.push('\n'), _ => {} } }
        Ok(text_content)
    }
    async fn extract_pdf_content(&self, file_path: &str) -> Result<String> { warn!("PDF解析功能暂未完全实现"); Ok(format!("PDF文件内容占位符: {}", file_path)) }
    async fn extract_word_content(&self, file_path: &str) -> Result<String> { warn!("Word文档解析功能暂未完全实现"); Ok(format!("Word文档内容占位符: {}", file_path)) }

    fn chunk_text(&self, content: &str, file_path: &str, source: &DocumentSource) -> Result<Vec<DocumentChunk>> {
        match self.config.chunking_strategy { ChunkingStrategy::FixedSize => self.chunk_text_fixed_size(content, file_path, source), ChunkingStrategy::RecursiveCharacter => self.chunk_text_recursive(content, file_path, source), ChunkingStrategy::Semantic => self.chunk_text_semantic(content, file_path, source), ChunkingStrategy::StructureAware => self.chunk_text_structure_aware(content, file_path, source) }
    }

    fn chunk_text_fixed_size(&self, content: &str, file_path: &str, source: &DocumentSource) -> Result<Vec<DocumentChunk>> {
        let mut chunks = Vec::new();
        let max_chunk_chars = self.config.chunk_size_chars;
        let overlap_chars_cfg = self.config.chunk_overlap_chars;
        let effective_overlap = if max_chunk_chars > 0 { std::cmp::min(overlap_chars_cfg, max_chunk_chars.saturating_sub(1)) } else { 0 };
        let chars: Vec<char> = content.chars().collect();
        let total_len = chars.len();
        if total_len == 0 || max_chunk_chars == 0 { return Ok(chunks); }
        let sentences = Self::split_into_sentences(&chars);
        if total_len <= max_chunk_chars { let chunk_content: String = chars[0..total_len].iter().collect(); let chunk = self.create_chunk(&chunk_content, 0, 0, total_len, file_path, source)?; chunks.push(chunk); return Ok(chunks); }
        let mut chunk_index = 0usize; let mut cursor = 0usize;
        while cursor < total_len {
            let target_end = std::cmp::min(cursor + max_chunk_chars, total_len);
            let mut end_char = cursor;
            for &(_, s_end) in &sentences { if s_end > cursor && s_end <= target_end { end_char = s_end; } if s_end > target_end { break; } }
            if end_char <= cursor { end_char = target_end; }
            if end_char <= cursor { end_char = std::cmp::min(cursor + max_chunk_chars, total_len); if end_char <= cursor { break; } }
            let chunk_str: String = chars[cursor..end_char].iter().collect();
            let chunk = self.create_chunk(&chunk_str, chunk_index, cursor, end_char, file_path, source)?;
            chunks.push(chunk); chunk_index += 1;
            if end_char >= total_len { break; }
            let mut next_cursor = end_char.saturating_sub(effective_overlap);
            if next_cursor <= cursor { let step = max_chunk_chars.saturating_sub(effective_overlap).max(1); next_cursor = std::cmp::min(cursor + step, total_len); }
            if next_cursor <= cursor { break; }
            cursor = next_cursor;
        }
        Ok(chunks)
    }

    fn chunk_text_recursive(&self, content: &str, file_path: &str, source: &DocumentSource) -> Result<Vec<DocumentChunk>> {
        const SEPARATORS: &[&str] = &["\n\n","\n",". ","。","! ","！","? ","？"," ",""];
        let chunks = self.recursive_split(content, SEPARATORS, 0, self.config.chunk_size_chars, self.config.chunk_overlap_chars, file_path, source)?;
        Ok(chunks)
    }

    fn recursive_split(&self, text: &str, separators: &[&str], chunk_index_offset: usize, chunk_size: usize, overlap: usize, file_path: &str, source: &DocumentSource) -> Result<Vec<DocumentChunk>> {
        let mut chunks = Vec::new();
        if text.chars().count() <= chunk_size { if !text.trim().is_empty() { let chunk = self.create_chunk(text, chunk_index_offset, 0, text.chars().count(), file_path, source)?; chunks.push(chunk); } return Ok(chunks); }
        if let Some((&separator, remaining)) = separators.split_first() {
            if !separator.is_empty() && text.contains(separator) {
                let parts: Vec<&str> = text.split(separator).collect();
                let mut current_chunk = String::new();
                let mut chunk_index = chunk_index_offset;
                for (i, part) in parts.iter().enumerate() {
                    let part_with_sep = if i < parts.len() - 1 { format!("{}{}", part, separator) } else { part.to_string() };
                    if current_chunk.chars().count() + part_with_sep.chars().count() <= chunk_size {
                        current_chunk.push_str(&part_with_sep);
                    } else {
                        if !current_chunk.trim().is_empty() { let chunk = self.create_chunk(&current_chunk, chunk_index, 0, current_chunk.chars().count(), file_path, source)?; chunks.push(chunk); chunk_index += 1; }
                        if part_with_sep.chars().count() > chunk_size {
                            let mut sub_chunks = self.recursive_split(&part_with_sep, remaining, chunk_index, chunk_size, overlap, file_path, source)?;
                            chunk_index += sub_chunks.len(); chunks.append(&mut sub_chunks); current_chunk.clear();
                        } else { current_chunk = part_with_sep; }
                    }
                }
                if !current_chunk.trim().is_empty() { let chunk = self.create_chunk(&current_chunk, chunk_index, 0, current_chunk.chars().count(), file_path, source)?; chunks.push(chunk); }
                return Ok(chunks);
            }
        }
        let chars: Vec<char> = text.chars().collect();
        let mut start = 0; let mut chunk_index = chunk_index_offset;
        while start < chars.len() {
            let end = std::cmp::min(start + chunk_size, chars.len());
            let chunk_text: String = chars[start..end].iter().collect();
            if !chunk_text.trim().is_empty() { let chunk = self.create_chunk(&chunk_text, chunk_index, start, end, file_path, source)?; chunks.push(chunk); chunk_index += 1; }
            start = if end >= chars.len() { break; } else { end.saturating_sub(overlap) };
            if start >= end { break; }
        }
        Ok(chunks)
    }

    fn chunk_text_semantic(&self, content: &str, file_path: &str, source: &DocumentSource) -> Result<Vec<DocumentChunk>> {
        warn!("Semantic chunking not fully implemented, falling back to recursive chunking");
        self.chunk_text_recursive(content, file_path, source)
    }

    fn chunk_text_structure_aware(&self, content: &str, file_path: &str, source: &DocumentSource) -> Result<Vec<DocumentChunk>> {
        let file_type = self.detect_file_type(file_path)?;
        match file_type { SupportedFileType::Md => self.chunk_markdown_structure(content, file_path, source), _ => { debug!("Structure-aware chunking not implemented, fallback"); self.chunk_text_recursive(content, file_path, source) } }
    }

    fn chunk_markdown_structure(&self, content: &str, file_path: &str, source: &DocumentSource) -> Result<Vec<DocumentChunk>> {
        let mut chunks = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut current_section = String::new();
        let mut chunk_index = 0; let mut current_header = String::new();
        for line in lines {
            if line.starts_with('#') {
                if !current_section.trim().is_empty() {
                    let chunk_content = if !current_header.is_empty() { format!("{}\n\n{}", current_header, current_section) } else { current_section.clone() };
                    if chunk_content.chars().count() >= self.config.min_chunk_size_chars { let chunk = self.create_chunk(&chunk_content, chunk_index, 0, chunk_content.chars().count(), file_path, source)?; chunks.push(chunk); chunk_index += 1; }
                }
                current_header = line.to_string(); current_section.clear();
            } else {
                current_section.push_str(line); current_section.push('\n');
                if current_section.chars().count() > self.config.max_chunk_size_chars {
                    let chunk_content = if !current_header.is_empty() { format!("{}\n\n{}", current_header, current_section) } else { current_section.clone() };
                    let mut sub_chunks = self.chunk_text_recursive(&chunk_content, file_path, source)?;
                    for chunk in &mut sub_chunks { chunk.chunk_index = chunk_index; chunk_index += 1; }
                    chunks.append(&mut sub_chunks); current_section.clear();
                }
            }
        }
        if !current_section.trim().is_empty() {
            let chunk_content = if !current_header.is_empty() { format!("{}\n\n{}", current_header, current_section) } else { current_section };
            if chunk_content.chars().count() >= self.config.min_chunk_size_chars { let chunk = self.create_chunk(&chunk_content, chunk_index, 0, chunk_content.chars().count(), file_path, source)?; chunks.push(chunk); }
        }
        Ok(chunks)
    }

    fn split_into_sentences(chars: &[char]) -> Vec<(usize, usize)> {
        let mut out = Vec::new(); let mut start = 0usize; let mut i = 0usize;
        while i < chars.len() { let c = chars[i]; let is_end_punct = matches!(c, '.' | '!' | '?' | '。' | '！' | '？'); if is_end_punct { let mut j = i + 1; while j < chars.len() { let c2 = chars[j]; if matches!(c2, '"' | '\'' | '”' | '’' | ')' | '）' | ']' | '】') { i = j; j += 1; } else if c2 == ' ' { j += 1; } else { break; } } let end = i + 1; if end > start { out.push((start, end)); } start = end; } i += 1; } if start < chars.len() { out.push((start, chars.len())); } out
    }

    fn create_chunk(&self, content: &str, chunk_index: usize, start_char: usize, end_char: usize, file_path: &str, source: &DocumentSource) -> Result<DocumentChunk> {
        let chunk_id = Uuid::new_v4().to_string();
        let content_hash = format!("{:x}", md5::compute(content.as_bytes()));
        let path = Path::new(file_path);
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();
        let file_type = path.extension().and_then(|e| e.to_str()).unwrap_or("unknown").to_string();
        let file_size = std::fs::metadata(file_path).map(|m| m.len()).unwrap_or(0);
        let metadata = ChunkMetadata { file_path: file_path.to_string(), file_name, file_type, file_size, chunk_start_char: start_char, chunk_end_char: end_char, page_number: None, section_title: None, custom_fields: HashMap::new() };
        Ok(DocumentChunk { id: chunk_id, source_id: source.id.clone(), content: content.to_string(), content_hash, chunk_index, metadata, embedding: None, created_at: chrono::Utc::now() })
    }
}

pub struct TextCleaner { whitespace_regex: Regex, special_chars_regex: Regex }
impl TextCleaner { pub fn new() -> Self { Self { whitespace_regex: Regex::new(r"\s+").unwrap(), special_chars_regex: Regex::new(r"[^\w\s\u4e00-\u9fff.,;:()\x22\x27\-/=&%+#@\[\]{}!?！？。，；：（）【】《》\u201c\u201d\u2018\u2019—…]").unwrap() } } pub fn clean_text(&self, text: &str) -> String { let mut cleaned = text.to_string(); cleaned = self.whitespace_regex.replace_all(&cleaned, " ").to_string(); cleaned = self.special_chars_regex.replace_all(&cleaned, "").to_string(); cleaned = cleaned.trim().to_string(); debug!("文本清洗完成，原长度: {}, 清洗后长度: {}", text.len(), cleaned.len()); cleaned } }

