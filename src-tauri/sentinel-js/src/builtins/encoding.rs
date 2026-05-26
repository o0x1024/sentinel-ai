//! TextEncoder / TextDecoder implementation (UTF-8).

use rquickjs::{Ctx, Value};
use crate::error::{JsError, Result};

pub fn install(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    // TextEncoder
    let encoder_code = r#"
        (function() {
            function TextEncoder() {}
            TextEncoder.prototype.encoding = "utf-8";
            TextEncoder.prototype.encode = function(str) {
                str = String(str || "");
                var bytes = [];
                for (var i = 0; i < str.length; i++) {
                    var code = str.charCodeAt(i);
                    if (code < 0x80) {
                        bytes.push(code);
                    } else if (code < 0x800) {
                        bytes.push(0xc0 | (code >> 6), 0x80 | (code & 0x3f));
                    } else if (code >= 0xd800 && code <= 0xdbff && i + 1 < str.length) {
                        var lo = str.charCodeAt(i + 1);
                        if (lo >= 0xdc00 && lo <= 0xdfff) {
                            code = ((code - 0xd800) << 10) + (lo - 0xdc00) + 0x10000;
                            i++;
                            bytes.push(
                                0xf0 | (code >> 18),
                                0x80 | ((code >> 12) & 0x3f),
                                0x80 | ((code >> 6) & 0x3f),
                                0x80 | (code & 0x3f)
                            );
                            continue;
                        }
                        bytes.push(0xef, 0xbf, 0xbd);
                    } else if (code >= 0xdc00 && code <= 0xdfff) {
                        bytes.push(0xef, 0xbf, 0xbd);
                    } else {
                        bytes.push(
                            0xe0 | (code >> 12),
                            0x80 | ((code >> 6) & 0x3f),
                            0x80 | (code & 0x3f)
                        );
                    }
                }
                return new Uint8Array(bytes);
            };
            return TextEncoder;
        })()
    "#;

    let encoder_class: Value = ctx.eval(encoder_code)
        .map_err(|e| JsError::Internal(e.to_string()))?;
    globals.set("TextEncoder", encoder_class)
        .map_err(|e| JsError::Internal(e.to_string()))?;

    // TextDecoder
    let decoder_code = r#"
        (function() {
            function TextDecoder(encoding) {
                this.encoding = encoding || "utf-8";
            }
            TextDecoder.prototype.decode = function(buf) {
                if (!buf) return "";
                var bytes = (buf instanceof Uint8Array) ? buf : new Uint8Array(buf);
                var result = "";
                for (var i = 0; i < bytes.length; ) {
                    var b = bytes[i];
                    var code;
                    if (b < 0x80) { code = b; i++; }
                    else if ((b & 0xe0) === 0xc0) {
                        code = ((b & 0x1f) << 6) | (bytes[i+1] & 0x3f); i += 2;
                    } else if ((b & 0xf0) === 0xe0) {
                        code = ((b & 0x0f) << 12) | ((bytes[i+1] & 0x3f) << 6) | (bytes[i+2] & 0x3f); i += 3;
                    } else {
                        code = ((b & 0x07) << 18) | ((bytes[i+1] & 0x3f) << 12) | ((bytes[i+2] & 0x3f) << 6) | (bytes[i+3] & 0x3f); i += 4;
                    }
                    if (code > 0xffff) {
                        code -= 0x10000;
                        result += String.fromCharCode(0xd800 + (code >> 10), 0xdc00 + (code & 0x3ff));
                    } else {
                        result += String.fromCharCode(code);
                    }
                }
                return result;
            };
            return TextDecoder;
        })()
    "#;

    let decoder_class: Value = ctx.eval(decoder_code)
        .map_err(|e| JsError::Internal(e.to_string()))?;
    globals.set("TextDecoder", decoder_class)
        .map_err(|e| JsError::Internal(e.to_string()))?;

    Ok(())
}
