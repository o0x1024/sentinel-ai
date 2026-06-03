//! TypeScript type stripping via oxc (parse → transform → codegen).
//!
//! Converts TypeScript source to valid JavaScript by removing all type-only
//! syntax through a proper AST pipeline.

use std::path::Path;

use oxc::allocator::Allocator;
use oxc::codegen::Codegen;
use oxc::parser::Parser;
use oxc::semantic::SemanticBuilder;
use oxc::span::SourceType;
use oxc::transformer::{TransformOptions, Transformer};

/// Strip TypeScript type syntax from `source`, producing valid JavaScript.
///
/// Uses oxc's pipeline: parser → semantic → TypeScript transformer → codegen.
/// This correctly handles all TS constructs (interfaces, type aliases,
/// declare blocks, generics, `as` casts, `satisfies`, enums, etc.).
pub fn strip_typescript(source: &str) -> Result<String, String> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(Path::new("plugin.ts"))
        .map_err(|e| format!("SourceType error: {e}"))?;

    let parser_ret = Parser::new(&allocator, source, source_type).parse();
    if parser_ret.panicked {
        return Err("Parser panicked".to_string());
    }
    if !parser_ret.errors.is_empty() {
        let msgs: Vec<String> = parser_ret.errors.iter().map(|e| e.to_string()).collect();
        return Err(format!("Parse errors: {}", msgs.join("; ")));
    }

    let mut program = parser_ret.program;

    let sem_ret = SemanticBuilder::new().build(&program);
    if !sem_ret.errors.is_empty() {
        let msgs: Vec<String> = sem_ret.errors.iter().map(|e| e.to_string()).collect();
        return Err(format!("Semantic errors: {}", msgs.join("; ")));
    }

    let transform_options = TransformOptions::default();
    let transformer_ret = Transformer::new(
        &allocator,
        Path::new("plugin.ts"),
        &transform_options,
    )
    .build_with_scoping(sem_ret.semantic.into_scoping(), &mut program);

    if !transformer_ret.errors.is_empty() {
        let msgs: Vec<String> = transformer_ret.errors.iter().map(|e| e.to_string()).collect();
        return Err(format!("Transform errors: {}", msgs.join("; ")));
    }

    let codegen_ret = Codegen::new().build(&program);
    Ok(codegen_ret.code)
}

/// Strip TypeScript and also remove `export` keywords so the result
/// can be evaluated in QuickJS script mode (non-ESM).
pub fn strip_typescript_for_script(source: &str) -> Result<String, String> {
    let js = strip_typescript(source)?;
    Ok(strip_export_keywords(&js))
}

/// Remove `export` / `export default` keywords from JS source.
/// Plugins bind to globalThis; ESM export syntax isn't needed.
fn strip_export_keywords(source: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim_start();

        // `export default ...` → keep only what follows
        if let Some(rest) = trimmed.strip_prefix("export default ") {
            let indent = &line[..line.len() - trimmed.len()];
            lines.push(format!("{indent}{rest}"));
            continue;
        }
        if trimmed == "export default" {
            continue;
        }

        // `export { ... }` or `export { ... } from ...` → drop the whole line
        if trimmed.starts_with("export {") || trimmed.starts_with("export*") {
            continue;
        }

        // `export function/class/const/let/var/async/enum` → strip `export `
        if let Some(rest) = trimmed.strip_prefix("export ") {
            let first_word = rest.split_ascii_whitespace().next().unwrap_or("");
            if matches!(
                first_word,
                "function" | "class" | "const" | "let" | "var" | "async" | "enum"
            ) {
                let indent = &line[..line.len() - trimmed.len()];
                lines.push(format!("{indent}{rest}"));
                continue;
            }
        }

        lines.push(line.to_string());
    }

    let mut result = lines.join("\n");
    if source.ends_with('\n') {
        result.push('\n');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_interface() {
        let input = "interface Foo { bar: string; }\nconst x = 1;";
        let output = strip_typescript(input).unwrap();
        assert!(!output.contains("interface"), "output: {output}");
        assert!(output.contains("const x = 1"), "output: {output}");
    }

    #[test]
    fn strip_type_alias() {
        let input = r#"type Name = string; const n = "hello";"#;
        let output = strip_typescript(input).unwrap();
        assert!(!output.contains("type Name"), "output: {output}");
        assert!(output.contains(r#"const n = "hello""#), "output: {output}");
    }

    #[test]
    fn strip_declare_const() {
        let input = "declare const Sentinel: { log: (msg: string) => void };\nconst x = 1;";
        let output = strip_typescript(input).unwrap();
        assert!(output.contains("const x = 1"), "output: {output}");
    }

    #[test]
    fn strip_function_param_types() {
        let input = "function add(a: number, b: number): number { return a + b; }";
        let output = strip_typescript(input).unwrap();
        assert!(output.contains("function add(a, b)"), "output: {output}");
        assert!(output.contains("return a + b"), "output: {output}");
    }

    #[test]
    fn strip_generic_params() {
        let input = "function identity<T>(x: T): T { return x; }";
        let output = strip_typescript(input).unwrap();
        assert!(output.contains("function identity(x)"), "output: {output}");
    }

    #[test]
    fn strip_as_cast() {
        let input = "const x = foo as string;";
        let output = strip_typescript(input).unwrap();
        assert!(output.contains("const x = foo"), "output: {output}");
        assert!(!output.contains("as string"), "output: {output}");
    }

    #[test]
    fn strip_export_interface() {
        let input = "export interface Foo { bar: string; }\nexport function run() {}";
        let output = strip_typescript_for_script(input).unwrap();
        assert!(output.contains("function run"), "output: {output}");
        assert!(!output.contains("interface Foo"), "output: {output}");
    }

    #[test]
    fn preserve_template_literals() {
        let input = r#"const msg = `Hello ${name}, you have ${count} items`;"#;
        let output = strip_typescript(input).unwrap();
        assert!(output.contains("Hello ${name}"), "output: {output}");
    }

    #[test]
    fn strip_complex_generics() {
        let input =
            "async function runWithConcurrency<T>(tasks: Array<() => Promise<T>>, concurrency: number): Promise<T[]> { return []; }";
        let output = strip_typescript(input).unwrap();
        assert!(
            output.contains("async function runWithConcurrency(tasks, concurrency)"),
            "output: {output}"
        );
        assert!(output.contains("return []"), "output: {output}");
    }

    #[test]
    fn strip_multiline_declare() {
        let input = r#"declare const Sentinel: {
    Monitor?: {
        reportProgress?: (payload: Record<string, unknown>) => Promise<boolean>;
    };
};

const x = 1;"#;
        let output = strip_typescript(input).unwrap();
        assert!(output.contains("const x = 1"), "output: {output}");
        assert!(!output.contains("Monitor"), "output: {output}");
    }

    #[test]
    fn strip_complex_type_alias() {
        let input = r#"type PluginGlobals = typeof globalThis & {
    get_input_schema?: typeof get_input_schema;
    analyze?: typeof analyze;
};
const pluginGlobals = globalThis;"#;
        let output = strip_typescript(input).unwrap();
        assert!(
            output.contains("const pluginGlobals = globalThis"),
            "output: {output}"
        );
        assert!(!output.contains("PluginGlobals"), "output: {output}");
    }
}
