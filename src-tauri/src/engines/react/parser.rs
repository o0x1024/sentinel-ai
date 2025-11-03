//! Action 指令解析器
//!
//! 支持两种解析策略：
//! 1. 强 JSON：严格校验 JSON Schema
//! 2. 自然语言兜底：正则提取 Action/Action Input

use super::types::{ActionInstruction, ParsedAction, ReactToolCall, FinalAnswer};
use anyhow::{Context, Result};
use serde_json;

pub struct ActionParser;

impl ActionParser {
    /// 解析 LLM 输出为 ActionInstruction（优先 JSON）
    pub fn parse(output: &str) -> Result<ActionInstruction> {
        // 先尝试直接解析整个输出为 JSON
        if let Ok(instruction) = Self::parse_json(output) {
            return Ok(instruction);
        }

        // 尝试提取 ```json 代码块
        if let Some(json_block) = Self::extract_json_block(output) {
            if let Ok(instruction) = Self::parse_json(&json_block) {
                return Ok(instruction);
            }
        }

        // 兜底：自然语言解析
        Self::parse_natural_language(output)
    }

    /// 纯 JSON 解析
    fn parse_json(text: &str) -> Result<ActionInstruction> {
        serde_json::from_str(text)
            .context("Failed to parse as JSON ActionInstruction")
    }

    /// 提取 Markdown 代码块中的 JSON
    fn extract_json_block(text: &str) -> Option<String> {
        // 匹配 ```json ... ``` 或 ``` ... ```
        let re = regex::Regex::new(r"```(?:json)?\s*\n([\s\S]*?)\n```").ok()?;
        re.captures(text)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().to_string())
    }

    /// 自然语言解析（兜底策略）
    /// 支持格式：
    ///   Action: tool_name
    ///   Action Input: {"key": "value"}
    /// 或：
    ///   Final Answer: xxx
    /// 或：
    ///   普通对话文本（作为最终答案）
    fn parse_natural_language(text: &str) -> Result<ActionInstruction> {
        // 检查是否为最终答案
        if let Some(final_answer) = Self::extract_final_answer(text) {
            return Ok(ActionInstruction::FinalAnswer {
                final_answer: FinalAnswer {
                    answer: final_answer,
                    citations: Vec::new(),
                },
            });
        }

        // 提取 Action 和 Action Input
        match Self::extract_action_and_input(text) {
            Ok(parsed) => {
                Ok(ActionInstruction::ToolCall {
                    action: ReactToolCall::new(parsed.tool_name, parsed.args),
                    final_answer: false,
                })
            }
            Err(_) => {
                // 如果没有找到 Action，但文本看起来像是回答问题，当作最终答案
                if !text.trim().is_empty() && Self::looks_like_answer(text) {
                    Ok(ActionInstruction::FinalAnswer {
                        final_answer: FinalAnswer {
                            answer: text.trim().to_string(),
                            citations: Vec::new(),
                        },
                    })
                } else {
                    // 真的无法解析
                    Err(anyhow::anyhow!("No Action or Final Answer found in text"))
                }
            }
        }
    }

    /// 判断文本是否看起来像是一个答案（而不是需要工具调用的计划）
    fn looks_like_answer(text: &str) -> bool {
        let text_lower = text.to_lowercase();
        
        // 如果包含明确的思考标记但没有行动，可能是不完整的输出
        if text_lower.contains("thought:") && !text_lower.contains("action:") {
            // 检查是否有足够的内容作为答案
            return text.len() > 100;
        }
        
        // 如果文本较长且没有特定的 ReAct 格式标记，可能是直接回答
        if text.len() > 50 
            && !text_lower.contains("action:")
            && !text_lower.contains("action input:")
            && !text_lower.contains("thought:") {
            return true;
        }
        
        // 包含常见答案模式
        text_lower.contains("the answer is")
            || text_lower.contains("based on")
            || text_lower.contains("according to")
            || text_lower.contains("in summary")
            || text_lower.contains("in conclusion")
    }

    /// 提取 Final Answer
    fn extract_final_answer(text: &str) -> Option<String> {
        // 使用 (?s) 标志让 . 匹配换行符，并使用 .+? 非贪婪匹配
        // 匹配 "Final Answer:" 之后的所有内容
        let re = regex::Regex::new(r"(?is)Final\s*Answer\s*:\s*(.+)").ok()?;
        re.captures(text)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().trim().to_string())
    }

    /// 提取 Action 和 Action Input
    fn extract_action_and_input(text: &str) -> Result<ParsedAction> {
        // 提取 Action
        let action_re = regex::Regex::new(r"(?i)Action\s*:\s*([^\n]+)")
            .context("Failed to compile Action regex")?;
        let tool_name = action_re
            .captures(text)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().trim().to_string())
            .context("No Action found in text")?;

        // 提取 Action Input（尝试解析为 JSON，失败则当字符串）
        let input_re = regex::Regex::new(r"(?i)Action\s*Input\s*:\s*(.+)")
            .context("Failed to compile Action Input regex")?;
        let input_str = input_re
            .captures(text)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_else(|| "{}".to_string());

        let args = serde_json::from_str(&input_str)
            .unwrap_or_else(|_| serde_json::json!({ "query": input_str }));

        Ok(ParsedAction { tool_name, args })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_tool_call() {
        let json = r#"{"action": {"tool": "search_web", "args": {"query": "test"}, "call_id": "123", "is_parallel": false}, "final_answer": false}"#;
        let result = ActionParser::parse(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_json_block() {
        let text = r#"
```json
{"action": {"tool": "search_web", "args": {"query": "test"}, "call_id": "456", "is_parallel": false}, "final_answer": false}
```
"#;
        let result = ActionParser::parse(text);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_natural_language() {
        let text = r#"
Thought: I need to search for information.
Action: search_web
Action Input: {"query": "Rust programming"}
"#;
        let result = ActionParser::parse(text);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_final_answer() {
        let text = "Final Answer: The answer is 42.";
        let result = ActionParser::parse(text);
        assert!(result.is_ok());
        match result.unwrap() {
            ActionInstruction::FinalAnswer { final_answer } => {
                assert_eq!(final_answer.answer, "The answer is 42.");
            }
            _ => panic!("Expected FinalAnswer"),
        }
    }
}
