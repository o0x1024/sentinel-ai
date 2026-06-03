//! crypto.getRandomValues, crypto.randomUUID, crypto.subtle.digest

use rquickjs::{Ctx, Function, Value};
use crate::error::{JsError, Result};

pub fn install(ctx: &Ctx<'_>) -> Result<()> {
    // Install the host __crypto_digest function first
    ctx.globals().set("__crypto_digest", Function::new(ctx.clone(), crypto_digest_host)
        .map_err(|e| JsError::Internal(e.to_string()))?)
        .map_err(|e| JsError::Internal(e.to_string()))?;

    // Install crypto object with getRandomValues, randomUUID, and subtle.digest
    let crypto_code = r#"
(function(globalThis) {
    "use strict";
    
    var crypto = {};
    
    crypto.getRandomValues = function(array) {
        for (var i = 0; i < array.length; i++) {
            array[i] = Math.floor(Math.random() * 256);
        }
        return array;
    };
    
    crypto.randomUUID = function() {
        var bytes = new Uint8Array(16);
        crypto.getRandomValues(bytes);
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        var hex = "";
        for (var i = 0; i < 16; i++) {
            var h = bytes[i].toString(16);
            if (h.length === 1) h = "0" + h;
            hex += h;
        }
        return hex.substring(0,8) + "-" + hex.substring(8,12) + "-" +
               hex.substring(12,16) + "-" + hex.substring(16,20) + "-" +
               hex.substring(20,32);
    };
    
    crypto.subtle = {
        digest: function(algorithm, data) {
            var algo = (typeof algorithm === "string") ? algorithm : algorithm.name;
            algo = algo.replace("-", "").toLowerCase();
            var bytes;
            if (data instanceof Uint8Array) {
                bytes = data;
            } else if (data instanceof ArrayBuffer) {
                bytes = new Uint8Array(data);
            } else {
                bytes = new Uint8Array(0);
            }
            var result = globalThis.__crypto_digest(algo, bytes);
            return Promise.resolve(result);
        }
    };
    
    globalThis.crypto = crypto;
})(globalThis);
"#;

    ctx.eval::<(), _>(crypto_code)
        .map_err(|e| JsError::Internal(format!("crypto install failed: {e}")))?;

    Ok(())
}

/// Host implementation of crypto digest.
fn crypto_digest_host<'js>(ctx: Ctx<'js>, algo: String, data: Value<'js>) -> rquickjs::Result<Value<'js>> {
    use sha2::{Sha256, Sha384, Sha512, Digest as _};
    use sha1::Sha1;
    use md5::Md5;

    let bytes = extract_bytes_from_value(&ctx, &data);

    let hash_bytes: Vec<u8> = match algo.as_str() {
        "sha256" | "sha2256" => Sha256::digest(&bytes).to_vec(),
        "sha384" => Sha384::digest(&bytes).to_vec(),
        "sha512" => Sha512::digest(&bytes).to_vec(),
        "sha1" => Sha1::digest(&bytes).to_vec(),
        "md5" => Md5::digest(&bytes).to_vec(),
        _ => return Err(rquickjs::Error::new_from_js("string", "Unsupported algorithm")),
    };

    let ab = rquickjs::ArrayBuffer::new(ctx, hash_bytes)
        .map_err(|_e| rquickjs::Error::new_from_js("value", "Failed to create ArrayBuffer"))?;
    Ok(ab.into_value())
}

fn extract_bytes_from_value(_ctx: &Ctx<'_>, value: &Value<'_>) -> Vec<u8> {
    // Try as ArrayBuffer
    if let Some(obj) = value.as_object() {
        if let Some(ab) = rquickjs::ArrayBuffer::from_object(obj.clone()) {
            if let Some(bytes) = ab.as_bytes() {
                return bytes.to_vec();
            }
        }
    }

    // Try as array of numbers
    if let Some(arr) = value.as_array() {
        let mut bytes = Vec::new();
        for i in 0..arr.len() {
            if let Ok(v) = arr.get::<f64>(i) {
                bytes.push(v as u8);
            }
        }
        return bytes;
    }

    Vec::new()
}
