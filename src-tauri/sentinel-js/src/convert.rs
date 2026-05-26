//! Conversion between serde_json::Value and QuickJS values.

use rquickjs::{Ctx, Value, Object, Array};

use crate::error::{JsError, Result};

/// Convert a serde_json::Value to a QuickJS JS value.
pub fn json_to_js<'js>(ctx: &Ctx<'js>, value: &serde_json::Value) -> Result<Value<'js>> {
    match value {
        serde_json::Value::Null => Ok(Value::new_null(ctx.clone())),
        serde_json::Value::Bool(b) => Ok(Value::new_bool(ctx.clone(), *b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::new_int(ctx.clone(), i as i32))
            } else if let Some(f) = n.as_f64() {
                Ok(Value::new_float(ctx.clone(), f))
            } else {
                Ok(Value::new_float(ctx.clone(), 0.0))
            }
        }
        serde_json::Value::String(s) => {
            let js_str = rquickjs::String::from_str(ctx.clone(), s)
                .map_err(|e| JsError::Conversion(e.to_string()))?;
            Ok(js_str.into_value())
        }
        serde_json::Value::Array(arr) => {
            let js_arr = Array::new(ctx.clone())
                .map_err(|e| JsError::Conversion(e.to_string()))?;
            for (i, item) in arr.iter().enumerate() {
                let js_item = json_to_js(ctx, item)?;
                js_arr.set(i, js_item)
                    .map_err(|e| JsError::Conversion(e.to_string()))?;
            }
            Ok(js_arr.into_value())
        }
        serde_json::Value::Object(obj) => {
            let js_obj = Object::new(ctx.clone())
                .map_err(|e| JsError::Conversion(e.to_string()))?;
            for (key, val) in obj {
                let js_val = json_to_js(ctx, val)?;
                js_obj.set(key.as_str(), js_val)
                    .map_err(|e| JsError::Conversion(e.to_string()))?;
            }
            Ok(js_obj.into_value())
        }
    }
}

/// Convert a QuickJS value to serde_json::Value.
pub fn js_to_json<'js>(ctx: &Ctx<'js>, value: &Value<'js>) -> Result<serde_json::Value> {
    if value.is_null() || value.is_undefined() {
        return Ok(serde_json::Value::Null);
    }

    if let Some(b) = value.as_bool() {
        return Ok(serde_json::Value::Bool(b));
    }

    if let Some(i) = value.as_int() {
        return Ok(serde_json::json!(i));
    }

    if let Some(f) = value.as_float() {
        if f.is_finite() {
            return Ok(serde_json::json!(f));
        }
        return Ok(serde_json::Value::Null);
    }

    if let Some(s) = value.as_string() {
        let rust_str = s.to_string()
            .map_err(|e| JsError::Conversion(e.to_string()))?;
        return Ok(serde_json::Value::String(rust_str));
    }

    if let Some(arr) = value.as_array() {
        let mut result = Vec::new();
        for i in 0..arr.len() {
            let item: Value = arr.get(i)
                .map_err(|e| JsError::Conversion(e.to_string()))?;
            result.push(js_to_json(ctx, &item)?);
        }
        return Ok(serde_json::Value::Array(result));
    }

    if let Some(obj) = value.as_object() {
        let mut map = serde_json::Map::new();
        for key in obj.keys::<String>() {
            if let Ok(key) = key {
                let val: Value = obj.get(&key)
                    .map_err(|e| JsError::Conversion(e.to_string()))?;
                map.insert(key, js_to_json(ctx, &val)?);
            }
        }
        return Ok(serde_json::Value::Object(map));
    }

    // Functions and other types → null
    Ok(serde_json::Value::Null)
}
