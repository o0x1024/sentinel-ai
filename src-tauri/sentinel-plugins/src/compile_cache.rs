//! Plugin compilation cache
//!
//! Caches stripped JS source keyed by (plugin_id, code_hash).
//! Subsequent engine loads skip TypeScript stripping when the source is unchanged.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

fn cache() -> &'static Mutex<CompileCache> {
    static CACHE: OnceLock<Mutex<CompileCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(CompileCache::new()))
}

struct CacheEntry {
    js_source: String,
    is_module: bool,
    code_hash: u64,
    compiled_at: Instant,
    hit_count: u64,
}

struct CompileCache {
    entries: HashMap<String, CacheEntry>,
    max_entries: usize,
}

impl CompileCache {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            max_entries: 128,
        }
    }
}

/// Strip and cache plugin source.
/// Returns the stripped JS source and whether it's a module.
pub fn compile_cached(
    plugin_id: &str,
    source: &str,
    is_module: bool,
) -> Result<(String, bool), String> {
    let code_hash = hash_source(source);

    {
        let mut c = cache().lock().unwrap();
        if let Some(entry) = c.entries.get_mut(plugin_id) {
            if entry.code_hash == code_hash {
                entry.hit_count += 1;
                return Ok((entry.js_source.clone(), entry.is_module));
            }
        }
    }

    let js_source = crate::ts_strip::strip_typescript_for_script(source)?;

    let mut c = cache().lock().unwrap();

    if c.entries.len() >= c.max_entries {
        let oldest = c
            .entries
            .iter()
            .min_by_key(|(_, e)| e.compiled_at)
            .map(|(k, _)| k.clone());
        if let Some(key) = oldest {
            c.entries.remove(&key);
        }
    }

    c.entries.insert(
        plugin_id.to_string(),
        CacheEntry {
            js_source: js_source.clone(),
            is_module,
            code_hash,
            compiled_at: Instant::now(),
            hit_count: 0,
        },
    );

    Ok((js_source, is_module))
}

/// Invalidate cache for a specific plugin.
pub fn invalidate(plugin_id: &str) {
    let mut c = cache().lock().unwrap();
    c.entries.remove(plugin_id);
}

/// Clear all cached compilations.
pub fn clear_all() {
    let mut c = cache().lock().unwrap();
    c.entries.clear();
}

/// Get cache statistics.
pub fn stats() -> CompileCacheStats {
    let c = cache().lock().unwrap();
    let total_hits: u64 = c.entries.values().map(|e| e.hit_count).sum();
    CompileCacheStats {
        entries: c.entries.len(),
        total_hits,
    }
}

#[derive(Debug, Clone)]
pub struct CompileCacheStats {
    pub entries: usize,
    pub total_hits: u64,
}

/// Hash plugin source for cache lookup and reload skip checks.
pub fn hash_plugin_source(source: &str) -> u64 {
    hash_source(source)
}

fn hash_source(source: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for byte in source.bytes() {
        h ^= byte as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}
