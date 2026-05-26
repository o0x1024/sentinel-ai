//! Buffer class implementation (wraps Uint8Array).

use rquickjs::Ctx;
use crate::error::{JsError, Result};

pub fn install(ctx: &Ctx<'_>) -> Result<()> {
    let code = r#"
(function(globalThis) {
    "use strict";

    function Buffer(arg, encodingOrOffset, length) {
        if (typeof arg === "number") {
            return new Uint8Array(arg);
        }
        if (typeof arg === "string") {
            var encoding = encodingOrOffset || "utf8";
            if (encoding === "utf8" || encoding === "utf-8") {
                var encoder = new TextEncoder();
                return encoder.encode(arg);
            }
            if (encoding === "base64") {
                var decoded = atob(arg);
                var bytes = new Uint8Array(decoded.length);
                for (var i = 0; i < decoded.length; i++) bytes[i] = decoded.charCodeAt(i);
                return bytes;
            }
            if (encoding === "hex") {
                var hexBytes = new Uint8Array(arg.length / 2);
                for (var j = 0; j < arg.length; j += 2) {
                    hexBytes[j/2] = parseInt(arg.substring(j, j+2), 16);
                }
                return hexBytes;
            }
            return new TextEncoder().encode(arg);
        }
        if (Array.isArray(arg)) {
            return new Uint8Array(arg);
        }
        if (arg instanceof Uint8Array || arg instanceof ArrayBuffer) {
            return new Uint8Array(arg);
        }
        return new Uint8Array(0);
    }

    Buffer.from = function(data, encoding) {
        return Buffer(data, encoding);
    };

    Buffer.alloc = function(size, fill) {
        var buf = new Uint8Array(size);
        if (fill !== undefined) {
            var fillByte = typeof fill === "number" ? fill : 0;
            buf.fill(fillByte);
        }
        return buf;
    };

    Buffer.allocUnsafe = function(size) {
        return new Uint8Array(size);
    };

    Buffer.concat = function(list, totalLength) {
        if (!totalLength) {
            totalLength = 0;
            for (var i = 0; i < list.length; i++) totalLength += list[i].length;
        }
        var result = new Uint8Array(totalLength);
        var offset = 0;
        for (var j = 0; j < list.length; j++) {
            result.set(list[j], offset);
            offset += list[j].length;
        }
        return result;
    };

    Buffer.isBuffer = function(obj) {
        return obj instanceof Uint8Array;
    };

    Buffer.byteLength = function(str, encoding) {
        return Buffer.from(str, encoding).length;
    };

    // Extend Uint8Array prototype with Buffer-like methods
    if (!Uint8Array.prototype.toString_buf) {
        Uint8Array.prototype.toString_buf = Uint8Array.prototype.toString;
        Uint8Array.prototype.toString = function(encoding) {
            if (!encoding || encoding === "utf8" || encoding === "utf-8") {
                return new TextDecoder().decode(this);
            }
            if (encoding === "hex") {
                var hex = "";
                for (var i = 0; i < this.length; i++) {
                    var h = this[i].toString(16);
                    if (h.length === 1) h = "0" + h;
                    hex += h;
                }
                return hex;
            }
            if (encoding === "base64") {
                var str = "";
                for (var j = 0; j < this.length; j++) str += String.fromCharCode(this[j]);
                return btoa(str);
            }
            return new TextDecoder().decode(this);
        };
    }

    globalThis.Buffer = Buffer;
})(globalThis);
"#;

    ctx.eval::<(), _>(code)
        .map_err(|e| JsError::Internal(format!("Buffer install failed: {e}")))?;
    Ok(())
}
