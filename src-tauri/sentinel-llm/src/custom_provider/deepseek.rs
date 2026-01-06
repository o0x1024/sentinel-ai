//! Custom DeepSeek Provider with reasoning_content support
//! 
//! This module provides a custom implementation for DeepSeek API that properly handles
//! the `reasoning_content` field required by deepseek-reasoner model when using tool calls.
//! 
//! Reference: https://api-docs.deepseek.com/zh-cn/guides/thinking_mode#tool-calls

use anyhow::{anyhow, Result};
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

use crate::StreamContent;
use rig::{
    message::{
        AssistantContent, Message as RigMessage, ToolCall as RigToolCall, ToolResultContent,
        UserContent,
    },
    tool::Tool,
};
use sentinel_tools::DynamicTool;

// ================================================================
// Message Definition with reasoning_content
// ================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum Message {
    System {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    User {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    Assistant {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        tool_calls: Vec<ToolCall>,
        /// Critical field for deepseek-reasoner model
        #[serde(skip_serializing_if = "Option::is_none")]
        reasoning_content: Option<String>,
    },
    #[serde(rename = "tool")]
    ToolResult {
        tool_call_id: String,
        content: String,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: Function,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Function {
    pub name: String,
    pub arguments: String,
}

impl From<RigToolCall> for ToolCall {
    fn from(call: RigToolCall) -> Self {
        Self {
            id: call.id,
            r#type: "function".to_string(),
            function: Function {
                name: call.function.name,
                arguments: serde_json::to_string(&call.function.arguments).unwrap_or_default(),
            },
        }
    }
}

// ================================================================
// Message Conversion
// ================================================================

/// Convert Rig Message to DeepSeek Message format
fn convert_message(message: RigMessage) -> Vec<Message> {
    match message {
        RigMessage::User { content } => {
            let mut user_messages = vec![];
            let mut tool_messages = vec![];

            for item in content.into_iter() {
                match item {
                    UserContent::Text(text) => user_messages.push(Message::User {
                        content: text.text,
                        name: None,
                    }),
                    UserContent::ToolResult(res) => {
                        let content = res.content.iter().map(|c| match c {
                            ToolResultContent::Text(t) => t.text.clone(),
                            ToolResultContent::Image(_) => "[Image]".to_string(),
                        }).collect::<Vec<String>>().join("");
                        tool_messages.push(Message::ToolResult {
                            tool_call_id: res.id,
                            content,
                        });
                    }
                    _ => {}
                }
            }
            
            // Critical: Tool messages must come BEFORE user messages to immediately follow 
            // the preceding Assistant message that likely contains the tool_calls.
            // Interleaving User messages between Assistant(tool_calls) and ToolResult 
            // causes "Messages with role 'tool' must be a response to a preceding message with 'tool_calls'" error.
            tool_messages.extend(user_messages);
            tool_messages
        }
        RigMessage::Assistant { content, .. } => {
            let mut text_content = String::new();
            let mut tool_calls = vec![];
            let mut reasoning_content: Option<String> = None;

            for item in content.into_iter() {
                match item {
                    AssistantContent::Text(text) => text_content.push_str(&text.text),
                    AssistantContent::ToolCall(call) => tool_calls.push(ToolCall::from(call)),
                    AssistantContent::Reasoning(r) => {
                        let reasoning_text = r.reasoning.join("");
                        if let Some(ref mut rc) = reasoning_content {
                            rc.push_str(&reasoning_text);
                        } else {
                            reasoning_content = Some(reasoning_text);
                        }
                    }
                    AssistantContent::Image(_) => {
                        // DeepSeek doesn't support image output, skip
                    }
                }
            }

            // Critical Fix: Ensure reasoning_content exists if there are tool calls
            // DeepSeek API requires this field even if empty
            if !tool_calls.is_empty() && reasoning_content.is_none() {
                reasoning_content = Some(String::new());
            }

            vec![Message::Assistant {
                content: text_content,
                name: None,
                tool_calls,
                reasoning_content,
            }]
        }
    }
}

// ================================================================
// Custom Stream Implementation
// ================================================================

/// Stream chat with DeepSeek API with proper reasoning_content support
pub async fn stream_deepseek<F>(
    http_client: Client,
    base_url: String,
    api_key: String,
    model: String,
    preamble: String,
    user_message: RigMessage,
    chat_history: Vec<RigMessage>,
    tools_map: HashMap<String, DynamicTool>,
    tools_json: Vec<serde_json::Value>,
    max_iterations: usize,
    mut on_content: F,
) -> Result<String>
where
    F: FnMut(StreamContent) -> bool,
{
    let mut current_messages = vec![Message::System {
        content: preamble,
        name: None,
    }];
    
    // Add history
    for msg in &chat_history {
        current_messages.extend(convert_message(msg.clone()));
    }
    
    // Add user message
    current_messages.extend(convert_message(user_message));
    
    // FIX: DeepSeek API requires ToolResult to strictly follow the Assistant message with tool_calls.
    // Problem 1: Orphaned ToolResults - if chat history was truncated (e.g. by sliding window),
    //            a ToolResult might exist without its parent Assistant message.
    // Problem 2: Interleaved messages - User or Assistant text messages between Assistant(calls) and ToolResult.
    
    // Pass 1: Collect all valid tool_call_ids from Assistant messages
    let mut valid_call_ids = std::collections::HashSet::new();
    let mut call_id_to_assistant_idx = HashMap::new();
    for (i, msg) in current_messages.iter().enumerate() {
        if let Message::Assistant { tool_calls, .. } = msg {
            for call in tool_calls {
                valid_call_ids.insert(call.id.clone());
                call_id_to_assistant_idx.insert(call.id.clone(), i);
            }
        }
    }

    // Pass 2: Remove orphaned ToolResults (those without a parent Assistant in the current messages)
    current_messages.retain(|msg| {
        if let Message::ToolResult { tool_call_id, .. } = msg {
            valid_call_ids.contains(tool_call_id)
        } else {
            true
        }
    });

    // Pass 3: Reorder ToolResults to immediately follow their parent Assistant message
    let mut i = 0;
    while i < current_messages.len() {
        if let Message::ToolResult { tool_call_id, .. } = &current_messages[i] {
            if let Some(&parent_idx) = call_id_to_assistant_idx.get(tool_call_id) {
                // Find the correct insertion position: after parent and after any existing ToolResults
                let mut target_idx = parent_idx + 1;
                while target_idx < i {
                    if let Message::ToolResult { .. } = &current_messages[target_idx] {
                        target_idx += 1;
                    } else {
                        // Found a non-ToolResult message, need to move current item here
                        let msg = current_messages.remove(i);
                        current_messages.insert(target_idx, msg);
                        // After moving, the item at position i is now a different message
                        // We need to re-check position i (don't increment)
                        // But to avoid infinite loop, we break and continue outer loop
                        break;
                    }
                }
                // If target_idx reached i, the message is already in correct position
                if target_idx >= i {
                    i += 1;
                }
            } else {
                // This shouldn't happen after Pass 2, but just in case
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    let mut final_content = String::new();
    
    for _iteration in 0..max_iterations {
        let mut body = json!({
            "model": model,
            "messages": current_messages,
            "stream": true,
        });
        
        if !tools_json.is_empty() {
            body["tools"] = json!(tools_json);
        }
        
        let response = http_client
            .post(format!("{}/chat/completions", base_url.trim_end_matches('/')))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("DeepSeek API Error: {}", error_text));
        }

        let mut stream = response.bytes_stream();
        let mut assistant_content = String::new();
        let mut assistant_reasoning = String::new();
        let mut tool_calls_map: HashMap<u64, (String, String, String)> = HashMap::new();
        let mut line_buffer = String::new();

        while let Some(chunk_res) = stream.next().await {
            let bytes = chunk_res?;
            let chunk_str = String::from_utf8_lossy(&bytes);
            line_buffer.push_str(&chunk_str);

            while let Some(newline_idx) = line_buffer.find('\n') {
                let line = line_buffer[..newline_idx].trim().to_string();
                line_buffer.drain(..=newline_idx);

                if line.is_empty() { continue; }
                if !line.starts_with("data: ") { continue; }
                
                let data = &line["data: ".len()..];
                if data == "[DONE]" { break; }
                
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(choices) = json["choices"].as_array() {
                        if let Some(choice) = choices.first() {
                            let delta = &choice["delta"];
                            
                            // Content
                            if let Some(c) = delta["content"].as_str() {
                                assistant_content.push_str(c);
                                final_content.push_str(c);
                                if !on_content(StreamContent::Text(c.to_string())) { 
                                    return Ok(final_content); 
                                }
                            }
                            
                            // Reasoning
                            if let Some(r) = delta["reasoning_content"].as_str() {
                                assistant_reasoning.push_str(r);
                                if !on_content(StreamContent::Reasoning(r.to_string())) { 
                                    return Ok(final_content); 
                                }
                            }
                            
                            // Tool Calls
                            if let Some(tcs) = delta["tool_calls"].as_array() {
                                for tc in tcs {
                                    let index = tc["index"].as_u64().unwrap_or(0);
                                    let entry = tool_calls_map.entry(index)
                                        .or_insert((String::new(), String::new(), String::new()));
                                    
                                    let mut name_changed = false;
                                    if let Some(id) = tc["id"].as_str() { 
                                        entry.0 = id.to_string(); 
                                    }
                                    
                                    if let Some(f) = tc.get("function") {
                                        if let Some(name) = f["name"].as_str() {
                                            if entry.1.is_empty() && !name.is_empty() {
                                                name_changed = true;
                                            }
                                            entry.1 = name.to_string();
                                        }
                                        if let Some(args) = f["arguments"].as_str() {
                                            entry.2.push_str(args);
                                            on_content(StreamContent::ToolCallDelta { 
                                                id: entry.0.clone(), 
                                                delta: args.to_string() 
                                            });
                                        }
                                    }

                                    // If we just got the ID or the name for the first time, emit ToolCallStart
                                    if !entry.0.is_empty() && (tc.get("id").is_some() || name_changed) {
                                        on_content(StreamContent::ToolCallStart { 
                                            id: entry.0.clone(), 
                                            name: entry.1.clone() 
                                        });
                                    }
                                }
                            }
                            
                            // Usage
                            if let Some(usage) = json.get("usage") {
                                let input = usage["prompt_tokens"].as_u64().unwrap_or(0) as u32;
                                let output = usage["completion_tokens"].as_u64().unwrap_or(0) as u32;
                                on_content(StreamContent::Usage { 
                                    input_tokens: input, 
                                    output_tokens: output 
                                });
                            }
                        }
                    }
                }
            }
        }
        
        // If no tool calls, we are done
        if tool_calls_map.is_empty() {
            on_content(StreamContent::Done);
            return Ok(final_content);
        }
        
        // Handle tool calls
        let mut tool_call_results = vec![];
        let mut tool_calls_vec = vec![];

        for (_, (id, name, args)) in tool_calls_map.iter() {
            on_content(StreamContent::ToolCallComplete { 
                id: id.clone(), 
                name: name.clone(), 
                arguments: args.clone() 
            });
            
            // Execute tool
            let result_text = if let Some(tool) = tools_map.get(name) {
                match serde_json::from_str::<serde_json::Value>(args) {
                    Ok(tool_args) => {
                        match tool.call(tool_args).await {
                            Ok(result) => serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
                            Err(e) => format!("Error executing tool: {}", e),
                        }
                    },
                    Err(e) => format!("Error parsing arguments: {}", e),
                }
            } else {
                format!("Tool not found: {}", name)
            };
            
            on_content(StreamContent::ToolResult { 
                id: id.clone(), 
                result: result_text.clone() 
            });
            tool_call_results.push((id.clone(), result_text));
            
            tool_calls_vec.push(ToolCall {
                id: id.clone(),
                r#type: "function".to_string(),
                function: Function { 
                    name: name.clone(), 
                    arguments: args.clone() 
                }
            });
        }
        
        // Append Assistant Message with reasoning_content
        current_messages.push(Message::Assistant {
            content: assistant_content,
            name: None,
            tool_calls: tool_calls_vec,
            reasoning_content: Some(assistant_reasoning),
        });
        
        // Append Tool Results
        for (id, res) in tool_call_results {
            current_messages.push(Message::ToolResult {
                tool_call_id: id,
                content: res,
            });
        }
    }
    
    Ok(final_content)
}

