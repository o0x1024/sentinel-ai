//! require() implementation supporting built-in Node modules.

use rquickjs::{Ctx, Function, Object, Value};
use crate::error::{JsError, Result};

pub fn install(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    globals.set("require", Function::new(ctx.clone(), require_impl)
        .map_err(|e| JsError::Internal(e.to_string()))?)
        .map_err(|e| JsError::Internal(e.to_string()))?;

    // Also set module and exports for CJS-style plugins
    let module = Object::new(ctx.clone())
        .map_err(|e| JsError::Internal(e.to_string()))?;
    let exports = Object::new(ctx.clone())
        .map_err(|e| JsError::Internal(e.to_string()))?;
    module.set("exports", exports.clone())
        .map_err(|e| JsError::Internal(e.to_string()))?;
    globals.set("module", module)
        .map_err(|e| JsError::Internal(e.to_string()))?;
    globals.set("exports", exports)
        .map_err(|e| JsError::Internal(e.to_string()))?;

    Ok(())
}

fn require_impl(ctx: Ctx<'_>, module_name: String) -> rquickjs::Result<Value<'_>> {
    match module_name.as_str() {
        "path" | "node:path" => build_path_module(&ctx),
        "url" | "node:url" => build_url_module(&ctx),
        "crypto" | "node:crypto" => build_crypto_module(&ctx),
        "buffer" | "node:buffer" => build_buffer_module(&ctx),
        "util" | "node:util" => build_util_module(&ctx),
        "os" | "node:os" => build_os_module(&ctx),
        "fs" | "node:fs" => build_fs_module(&ctx),
        "querystring" | "node:querystring" => build_querystring_module(&ctx),
        _ => Err(rquickjs::Error::new_from_js("string", "Module not found")),
    }
}

fn build_path_module<'js>(ctx: &Ctx<'js>) -> rquickjs::Result<Value<'js>> {
    let code = include_str!("path_module.js");
    ctx.eval(code)
}

fn build_url_module<'js>(ctx: &Ctx<'js>) -> rquickjs::Result<Value<'js>> {
    let code = "(function() { return { URL: globalThis.URL, URLSearchParams: globalThis.URLSearchParams }; })()";
    ctx.eval(code)
}

fn build_crypto_module<'js>(ctx: &Ctx<'js>) -> rquickjs::Result<Value<'js>> {
    let code = include_str!("crypto_module.js");
    ctx.eval(code)
}

fn build_buffer_module<'js>(ctx: &Ctx<'js>) -> rquickjs::Result<Value<'js>> {
    let code = "(function() { return { Buffer: globalThis.Buffer }; })()";
    ctx.eval(code)
}

fn build_util_module<'js>(ctx: &Ctx<'js>) -> rquickjs::Result<Value<'js>> {
    let code = r#"(function() {
        return {
            inspect: function(obj) { try { return JSON.stringify(obj, null, 2); } catch(e) { return String(obj); } },
            format: function() {
                var args = Array.prototype.slice.call(arguments);
                if (args.length === 0) return "";
                var fmt = String(args[0]);
                var idx = 1;
                return fmt.replace(/%[sdjifoO%]/g, function(m) {
                    if (m === "%%") return "%";
                    if (idx >= args.length) return m;
                    return String(args[idx++]);
                });
            },
            promisify: function(fn) { return fn; },
            types: { isUint8Array: function(v) { return v instanceof Uint8Array; } }
        };
    })()"#;
    ctx.eval(code)
}

fn build_os_module<'js>(ctx: &Ctx<'js>) -> rquickjs::Result<Value<'js>> {
    let platform = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let code = format!(
        r#"(function() {{ return {{
            platform: function() {{ return "{platform}"; }},
            arch: function() {{ return "{arch}"; }},
            homedir: function() {{ return "/"; }},
            tmpdir: function() {{ return "/tmp"; }},
            EOL: "\n",
            cpus: function() {{ return []; }}
        }}; }})()"#
    );
    ctx.eval(code)
}

fn build_fs_module<'js>(ctx: &Ctx<'js>) -> rquickjs::Result<Value<'js>> {
    // Minimal fs stub — actual file access controlled by sandbox
    let code = r#"(function() {
        return {
            readFileSync: function(path, opts) {
                if (typeof globalThis.__host_read_file === "function") {
                    return globalThis.__host_read_file(path, opts);
                }
                throw new Error("fs.readFileSync: file system access not available");
            },
            writeFileSync: function(path, data) {
                if (typeof globalThis.__host_write_file === "function") {
                    return globalThis.__host_write_file(path, data);
                }
                throw new Error("fs.writeFileSync: file system access not available");
            },
            existsSync: function(path) {
                if (typeof globalThis.__host_file_exists === "function") {
                    return globalThis.__host_file_exists(path);
                }
                return false;
            },
            promises: {
                readFile: function(path, opts) {
                    return Promise.resolve(require("fs").readFileSync(path, opts));
                },
                writeFile: function(path, data) {
                    return Promise.resolve(require("fs").writeFileSync(path, data));
                }
            }
        };
    })()"#;
    ctx.eval(code)
}

fn build_querystring_module<'js>(ctx: &Ctx<'js>) -> rquickjs::Result<Value<'js>> {
    let code = r#"(function() {
        return {
            stringify: function(obj) {
                return Object.keys(obj).map(function(k) {
                    return encodeURIComponent(k) + "=" + encodeURIComponent(obj[k]);
                }).join("&");
            },
            parse: function(str) {
                var result = {};
                str.split("&").forEach(function(pair) {
                    var parts = pair.split("=");
                    if (parts[0]) result[decodeURIComponent(parts[0])] = decodeURIComponent(parts[1] || "");
                });
                return result;
            }
        };
    })()"#;
    ctx.eval(code)
}
