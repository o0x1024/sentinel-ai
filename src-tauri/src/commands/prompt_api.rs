use crate::models::prompt::{PromptCategory, PromptTemplate, TemplateType};
use crate::services::prompt_db::PromptRepository;
use crate::services::DatabaseService;
use crate::utils::prompt_resolver::{AgentPromptConfig, CanonicalStage, PromptResolver};
use anyhow::Result;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn list_prompt_templates_api(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<PromptTemplate>, String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.list_templates().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_prompt_template_api(
    db: State<'_, Arc<DatabaseService>>,
    id: i64,
) -> Result<Option<PromptTemplate>, String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.get_template(id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_prompt_template_api(
    db: State<'_, Arc<DatabaseService>>,
    template: PromptTemplate,
) -> Result<i64, String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.create_template(&template)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_prompt_template_api(
    db: State<'_, Arc<DatabaseService>>,
    id: i64,
    template: PromptTemplate,
) -> Result<(), String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);

    // If activating this template, deactivate other templates of the same type
    if template.is_active && template.template_type.is_some() {
        let template_type = template.template_type.as_ref().unwrap();

        // Get all templates of the same type
        let all_templates = repo
            .list_templates_filtered(template.category.clone(), Some(template_type.clone()), None)
            .await
            .map_err(|e| e.to_string())?;

        // Deactivate other active templates of the same type
        for other_template in all_templates {
            if other_template.id.is_some()
                && other_template.id.unwrap() != id
                && other_template.is_active
            {
                let mut deactivated = other_template.clone();
                deactivated.is_active = false;
                repo.update_template(other_template.id.unwrap(), &deactivated)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    repo.update_template(id, &template)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_prompt_template_api(
    db: State<'_, Arc<DatabaseService>>,
    id: i64,
) -> Result<(), String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.delete_template(id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_prompt_templates_filtered_api(
    db: State<'_, Arc<DatabaseService>>,
    category: Option<PromptCategory>,
    template_type: Option<TemplateType>,
    is_system: Option<bool>,
) -> Result<Vec<PromptTemplate>, String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.list_templates_filtered(category, template_type, is_system)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn duplicate_prompt_template_api(
    db: State<'_, Arc<DatabaseService>>,
    id: i64,
    new_name: Option<String>,
) -> Result<i64, String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.duplicate_template(id, new_name)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn evaluate_prompt_api(
    db: State<'_, Arc<DatabaseService>>,
    template_id: i64,
    context: serde_json::Value,
) -> Result<String, String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.evaluate_prompt(template_id, context)
        .await
        .map_err(|e| e.to_string())
}

// ===== Preview resolved prompt for AgentManager =====

fn map_stage_to_canonical(stage: &str) -> Option<CanonicalStage> {
    match stage {
        "system" => Some(CanonicalStage::System),
        "intent_classifier" => Some(CanonicalStage::IntentClassifier),
        _ => None,
    }
}

#[tauri::command]
pub async fn preview_resolved_prompt_api(
    db: State<'_, Arc<DatabaseService>>,
    engine: String,
    stage: String,
    agent_config: JsonValue,
) -> Result<String, String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);

    let resolver = PromptResolver::new(repo);
    let canonical =
        map_stage_to_canonical(&stage).ok_or_else(|| format!("Invalid stage: {}", stage))?;

    let params: HashMap<String, JsonValue> = agent_config
        .as_object()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .collect();

    let cfg = AgentPromptConfig::parse_agent_config(&params);

    resolver
        .resolve_prompt(&cfg, canonical, None)
        .await
        .map_err(|e| e.to_string())
}

/// Get plugin generation prompt template by type
#[tauri::command]
pub async fn get_plugin_generation_prompt_api(
    db: State<'_, Arc<DatabaseService>>,
    template_type: String, // "passive" or "agent"
) -> Result<String, String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);

    // 根据类型获取对应的模板
    let t_type = match template_type.as_str() {
        "passive" => TemplateType::PluginGeneration,
        "agent" => TemplateType::AgentPluginGeneration,
        _ => return Err(format!("Unknown template type: {}", template_type)),
    };

    // 获取该类型的模板（只获取激活的模板）
    let templates = repo
        .list_templates_filtered(Some(PromptCategory::Application), Some(t_type), None)
        .await
        .map_err(|e| e.to_string())?;

    // 只返回激活的模板，如果没有激活的则返回错误
    let template = templates
        .iter()
        .find(|t| t.is_active)
        .ok_or_else(|| format!("No active template found for type: {}", template_type))?;

    Ok(template.content.clone())
}

const DEFAULT_PASSIVE_PLUGIN_PROMPT: &str = r#"# Passive Scan Plugin Generation Task

You are a professional security researcher and TypeScript developer. Your task is to generate high-quality Passive Scan plugins for an AI-driven security testing system.

## Task Overview

Passive scan plugins should:
1. Be written in TypeScript.
2. Analyze HTTP traffic (requests and responses) without modifying it.
3. Detect specific vulnerabilities or collect information.
4. Export required scanning functions.
5. Use `Sentinel.emitFinding()` to report findings.

## Key Principles

**Passive Analysis**:
- NEVER send new requests or block traffic.
- Analyze existing `HttpTransaction` data strictly.
- Be stateless and performant (process thousands of requests/sec).

**Detection Strategy**:
- Check response headers for missing/insecure configurations.
- Inspect response bodies for sensitive information leaks (regex/pattern matching).
- Analyze request parameters for obvious security flaws.

**Code Quality**:
- Write concise, performant TypeScript.
- Handle missing optional fields safely (e.g., `transaction.response` might be null).
- Use descriptive variable names.

---

## Passive Plugin Interface (Required Structure)

The Passive Scan plugin you generate **MUST** include the following structure:

### 1. Main Scan Function

You **MUST** export `scan_transaction` to analyze traffic.

```typescript
/**
 * Scans an HTTP transaction (request + response)
 * @param {HttpTransaction} transaction
 */
export async function scan_transaction(transaction) {
    const req = transaction.request;
    const resp = transaction.response; // Note: Can be null/undefined if only request was captured
    
    // Logic goes here...
}
```

**Data Structures**:

*   **HttpTransaction**: `{ request: RequestContext, response: ResponseContext | null }`
*   **RequestContext**: `{ url, method, headers (Map-like object), body (Uint8Array), ... }`
*   **ResponseContext**: `{ status, headers (Map-like object), body (Uint8Array), ... }`

**Note on Body Handling**:
Bodies are `Uint8Array`. careful with large bodies. Use `TextDecoder` to convert to string if needed.

### 3. Reporting Findings

Use the global `Sentinel` object to report results.

```typescript
Sentinel.emitFinding({
    title: "Vulnerability Title",
    severity: "high",
    description: "Detailed description...",
    evidence: "The matched string or header value",
    confidence: "High", // "High", "Medium", "Low"
    location: "Header: X-Powered-By" // or URL, Body line, etc.
});
```

### Available APIs

*   `TextDecoder` / `TextEncoder`
*   `URL` / `URLSearchParams`
*   `Sentinel.log(level, message)`
*   `Sentinel.emitFinding(finding)`

---

## Output Format

Return ONLY the TypeScript plugin code wrapped in a markdown code block:

```typescript
export async function scan_transaction(transaction) {
  const resp = transaction.response;
  if (!resp) return;

  // Example: Check for a specific header
  const headerValue = resp.headers["x-vulnerable-header"];
  if (headerValue) {
      Sentinel.emitFinding({
          title: "Vulnerable Header Detected",
          severity: "medium",
          description: "The server exposes a vulnerable header.",
          evidence: `x-vulnerable-header: ${headerValue}`,
          confidence: "High",
          location: "Response Header"
      });
  }
}
  
globalThis.scan_transaction = scan_transaction;
```

**Requirements**:
1. **MUST export `scan_transaction`**.
2. **Handle `transaction.response` being potentially null**.
3. **Use `Sentinel.emitFinding`** for reporting.
4. **No network calls** (fetch, etc.) are allowed in passive plugins.
5. **MUST include the `globalThis` export at the end** - Without this, the plugin will fail with "Function not found" error.

Now generate the Passive Scan Plugin.
"#;

const DEFAULT_AGENT_PLUGIN_PROMPT: &str = r#"# Agent Tool Plugin Generation Task

You are a professional security researcher and TypeScript developer. Your task is to generate high-quality Agent tool plugins for an AI-driven security testing system.

## Task Overview

Agent tool plugins should:
1. Be written in TypeScript.
2. Implement specific security testing or analysis functionality.
3. Follow the Agent tool plugin interface.
4. Include appropriate error handling and validation.
5. Use the `ToolOutput` interface to return structured results.
6. **MUST export a `get_input_schema()` function to declare the input parameters' JSON Schema.**

## Key Principles

**Important**: Generate generic tool logic that works in different scenarios, not just for specific targets. Use requirements as a reference for common patterns, but make the tool widely applicable.

**Implementation Strategy**:
- Focus on reusable tool functionality (scanning, analysis, reporting, etc.).
- Use correct TypeScript types and interfaces.
- Validate inputs and handle edge cases.
- Return detailed, actionable results.
- Include confidence levels and evidence where applicable.

**Code Quality**:
- Write concise, well-commented TypeScript code.
- Use try-catch blocks for appropriate error handling.
- Include descriptive variable names.
- Add inline comments explaining tool logic.

**Security Best Practices**:
- Validate all inputs before processing.
- Handle sensitive data appropriately.
- Provide detailed error messages for debugging.
- Include appropriate logging to improve observability.

---

## Agent Tool Plugin Interface (Required Structure)

The Agent tool plugin you generate **MUST** include the following structure:

### 1. get_input_schema() Function (Required)

Export a `get_input_schema()` function that returns a JSON Schema object describing the input parameters. The engine will call this function after loading the plugin:

```typescript
/**
 * Export parameter schema function (Required)
 * The engine calls this function after loading to get parameter descriptions
 */
export function get_input_schema() {
    return {
        type: "object",
        required: ["target"],
        properties: {
            target: {
                type: "string",
                description: "Target URL or host address"
            },
            timeout: {
                type: "integer",
                default: 5000,
                description: "Timeout in milliseconds"
            },
            options: {
                type: "object",
                description: "Additional options"
            }
        }
    };
}

// Must bind to globalThis
globalThis.get_input_schema = get_input_schema;
```

### 2. TypeScript Interface and Implementation

```typescript
// Tool input interface - consistent with get_input_schema() return value
interface ToolInput {
    target: string;
    timeout?: number;
    options?: Record<string, any>;
}

// Tool output interface - standardized result structure
interface ToolOutput {
    success: boolean;           // Whether the tool execution was successful
    data?: any;                 // Tool-specific result data
    error?: string;             // Error message on failure
    // Optional fields:
    // confidence?: "high" | "medium" | "low";
    // evidence?: string[];
    // metadata?: Record<string, any>;
}

// Main tool function - implements tool logic
export async function analyze(input: ToolInput): Promise<ToolOutput> {
    try {
        // Validate input
        if (!input || !input.target) {
            return {
                success: false,
                error: "Invalid input: target is required"
            };
        }
        
        // Implement your tool logic here
        // For example:
        // const result = await performAnalysis(input);
        
        return {
            success: true,
            data: {
                // Your tool results
            }
        };
    } catch (error) {
        return {
            success: false,
            error: error instanceof Error ? error.message : String(error)
        };
    }
}

// **CRITICAL**: Export the function to globalThis for the plugin engine to call
// Without this export, the plugin will fail with "Function not found" error
globalThis.analyze = analyze;
```

### Complete Example: Port Scanner Tool

```typescript
/**
 * Port Scanner Tool
 * @plugin port_scanner
 * @name Port Scanner
 * @version 1.0.0
 * @author Sentinel AI
 * @category scanner
 * @default_severity medium
 * @tags network, port, scanner
 * @description Scan target host for open ports
 */

interface ToolInput {
    target: string;
    ports?: string;
    timeout?: number;
}

interface ToolOutput {
    success: boolean;
    data?: {
        open_ports: number[];
        closed_ports: number[];
        scan_duration: number;
    };
    error?: string;
}

/**
 * Export parameter schema function (Required)
 */
export function get_input_schema() {
    return {
        type: "object",
        required: ["target"],
        properties: {
            target: {
                type: "string",
                description: "Target host address (IP or domain)"
            },
            ports: {
                type: "string",
                default: "80,443,22,21",
                description: "List of ports to scan, comma-separated"
            },
            timeout: {
                type: "integer",
                default: 3000,
                description: "Timeout per port in milliseconds"
            }
        }
    };
}

export async function analyze(input: ToolInput): Promise<ToolOutput> {
    // Tool implementation...
}

// Must bind to globalThis
globalThis.get_input_schema = get_input_schema;
globalThis.analyze = analyze;
```

### Available APIs

Agent tool plugins can access the following built-in APIs:

**1. Fetch API** - Make HTTP requests:
```typescript
const response = await fetch('https://example.com/api', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ key: 'value' }),
    timeout: 5000,
});
const data = await response.json();
```

**2. TextDecoder/TextEncoder** - Decode/Encode text:
```typescript
const encoder = new TextEncoder();
const bytes = encoder.encode('Hello, World!');

const decoder = new TextDecoder();
const text = decoder.decode(bytes);
```

**3. URL/URLSearchParams** - Parse URLs:
```typescript
const url = new URL('https://example.com/api?key=value');
const params = new URLSearchParams(url.search);
const key = params.get('key');
```

**4. Logging** - Debug output:
```typescript
Deno.core.ops.op_plugin_log('info', 'Tool started...');
Deno.core.ops.op_plugin_log('error', 'Error occurred');
```

---

## Output Format

Return ONLY the TypeScript plugin code wrapped in a markdown code block:

```typescript
/**
 * Tool Plugin
 * @plugin tool_id
 * @name Tool Name
 * @version 1.0.0
 * @author Sentinel AI
 * @category category
 * @default_severity medium
 * @tags tag1, tag2
 * @description Tool description
 */

interface ToolInput {
    target: string;
    option1?: string;
}

interface ToolOutput {
    success: boolean;
    data?: any;
    error?: string;
}

/**
 * Export parameter schema function (Required)
 */
export function get_input_schema() {
    return {
        type: "object",
        required: ["target"],
        properties: {
            target: {
                type: "string",
                description: "Target address"
            },
            option1: {
                type: "string",
                default: "default_value",
                description: "Description of option 1"
            }
        }
    };
}

export async function analyze(input: ToolInput): Promise<ToolOutput> {
    try {
        // Validate input
        if (!input || !input.target) {
            return { success: false, error: "target is required" };
        }
        
        // Your tool logic
        return {
            success: true,
            data: {
                // Your results
            }
        };
    } catch (error) {
        return {
            success: false,
            error: error instanceof Error ? error.message : String(error)
        };
    }
}

// **CRITICAL**: Must export functions to globalThis
globalThis.get_input_schema = get_input_schema;
globalThis.analyze = analyze;
```

**Requirements**:
1. **MUST export a `get_input_schema()` function** - This is key for the AI and workflow system to understand tool parameters.
2. The `properties` in the returned schema must include a `description` field for each parameter.
3. Include detailed comments explaining the tool logic.
4. Use correct TypeScript types.
5. Handle edge cases and errors gracefully.
6. Return a structured `ToolOutput` with success/error status.
7. Include input parameter validation.
8. **MUST bind both `get_input_schema` and `analyze` to globalThis** - Without this, the plugin will fail with "Function not found" error.

Now generate the Agent Tool Plugin.
"#;

/// Get combined plugin generation prompt (generation + interface + output format)
#[tauri::command]
pub async fn get_combined_plugin_prompt_api(
    db: State<'_, Arc<DatabaseService>>,
    plugin_type: String, // "passive" or "agent"
    vuln_type: String,
    severity: String,
) -> Result<String, String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);

    println!("plugin_type: {}", plugin_type);
    println!("vuln_type: {}", vuln_type);
    println!("severity: {}", severity);

    // 根据插件类型选择对应的模板类型（合并后的完整模板）
    let template_type = match plugin_type.as_str() {
        "passive" => TemplateType::PluginGeneration,
        "agent" => TemplateType::AgentPluginGeneration,
        _ => TemplateType::PluginGeneration,
    };

    // 获取合并后的完整模板
    let templates = repo
        .list_templates_filtered(Some(PromptCategory::Application), Some(template_type), None)
        .await
        .map_err(|e| e.to_string())?;

    // 只获取激活的模板，如果没有激活的则使用回退方案
    let template = templates
        .iter()
        .find(|t| t.is_active)
        .map(|t| t.content.clone())
        .unwrap_or_else(|| {
            println!(
                "No active template found in DB for {}, using fallback.",
                plugin_type
            );
            match plugin_type.as_str() {
                "agent" => DEFAULT_AGENT_PLUGIN_PROMPT.to_string(),
                _ => DEFAULT_PASSIVE_PLUGIN_PROMPT.to_string(),
            }
        });

    // 进行变量替换
    let mut content = template;
    content = content.replace("{plugin_type}", &plugin_type);
    content = content.replace("{vuln_type}", &vuln_type);
    content = content.replace("{severity}", &severity);

    Ok(content)
}

/// Get default prompt content from prompt.md files in app data directory
#[tauri::command]
pub async fn get_default_prompt_content() -> Result<String, String> {
    use std::fs;
    use std::path::PathBuf;

    // Use default react directory
    let arch_dir = "react";

    // Get app data directory
    let app_data_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sentinel-ai")
        .join("prompts");

    // Construct path to prompt.md in app data directory
    let prompt_path = app_data_dir.join(arch_dir).join("prompt.md");

    // Read the file
    let content = fs::read_to_string(&prompt_path).map_err(|e| {
        format!(
            "Failed to read prompt file for {} at {:?}: {}",
            arch_dir, prompt_path, e
        )
    })?;

    // Return the full content
    Ok(content)
}

/// Initialize default prompt files by copying from source to app data directory
#[tauri::command]
pub async fn initialize_default_prompts() -> Result<String, String> {
    use std::fs;
    use std::path::PathBuf;

    // Get app data directory
    let app_data_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sentinel-ai")
        .join("prompts");

    // Create prompts directory if it doesn't exist
    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create prompts directory: {}", e))?;

    let architectures = vec![
        ("rewoo", "rewoo"),
        ("llm_compiler", "llm_compiler"),
        ("plan_and_execute", "plan_and_execute"),
        ("react", "react"),
        ("travel", "travel"),
    ];

    let mut copied_count = 0;
    let mut skipped_count = 0;

    for (arch_name, arch_dir) in architectures {
        // Source path (from compiled binary resources)
        let source_path: PathBuf = [
            env!("CARGO_MANIFEST_DIR"),
            "src",
            "engines",
            arch_dir,
            "prompt.md",
        ]
        .iter()
        .collect();

        // Destination path (in app data directory)
        let dest_dir = app_data_dir.join(arch_name);
        let dest_path = dest_dir.join("prompt.md");

        // Create architecture directory
        fs::create_dir_all(&dest_dir)
            .map_err(|e| format!("Failed to create directory for {}: {}", arch_name, e))?;

        // Only copy if destination doesn't exist (don't overwrite user modifications)
        if !dest_path.exists() {
            if source_path.exists() {
                fs::copy(&source_path, &dest_path)
                    .map_err(|e| format!("Failed to copy prompt file for {}: {}", arch_name, e))?;
                copied_count += 1;
            }
        } else {
            skipped_count += 1;
        }
    }

    Ok(format!(
        "Initialized prompts: {} copied, {} skipped (already exist)",
        copied_count, skipped_count
    ))
}
