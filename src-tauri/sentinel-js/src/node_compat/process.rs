//! process object stub (env, platform, cwd, argv, etc.)

use rquickjs::Ctx;
use crate::error::{JsError, Result};

pub fn install(ctx: &Ctx<'_>) -> Result<()> {
    let platform = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let code = format!(
        r#"
(function(globalThis) {{
    "use strict";

    var process = {{
        platform: "{platform}",
        arch: "{arch}",
        version: "v20.0.0",
        versions: {{ node: "20.0.0" }},
        env: {{}},
        argv: [],
        pid: 0,
        ppid: 0,
        stdout: {{ write: function(s) {{ console.log(s); }} }},
        stderr: {{ write: function(s) {{ console.error(s); }} }},
        cwd: function() {{ return "/"; }},
        exit: function(code) {{ throw new Error("process.exit(" + code + ") called"); }},
        nextTick: function(fn) {{ Promise.resolve().then(fn); }},
        hrtime: {{
            bigint: function() {{ return BigInt(Math.floor(performance.now() * 1e6)); }}
        }},
        memoryUsage: function() {{
            return {{ rss: 0, heapTotal: 0, heapUsed: 0, external: 0 }};
        }}
    }};

    globalThis.process = process;
    globalThis.__dirname = "/";
    globalThis.__filename = "plugin.js";
    globalThis.global = globalThis;
}})(globalThis);
"#
    );

    ctx.eval::<(), _>(code.as_str())
        .map_err(|e| JsError::Internal(format!("process install failed: {e}")))?;
    Ok(())
}
