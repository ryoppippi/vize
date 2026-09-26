//! Checked formatter/output hooks over a real native compile result.
//!
//! Plugins change whitespace or add unlinked comments. The host preserves
//! JavaScript semantics and rebases every existing authored map anchor.
#![expect(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::disallowed_macros,
    reason = "serialized plugin boundary uses standard strings"
)]

mod cache;
mod map;
mod rewrite;
#[cfg(test)]
mod tests;

use crate::CompileResult;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Instant;

pub use cache::{CacheInput, content_key_for_build};

/// A plugin identity and the inputs its sandbox declares.
pub struct PluginSpec<'a> {
    pub name: &'a str,
    pub version: &'a str,
    pub fingerprint: &'a str,
    pub family: &'a str,
    pub inputs: Option<&'a [CacheInput]>,
}

/// Actual host and callback cost; cache hits make no JavaScript call.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cost {
    pub name: String,
    pub version: String,
    pub family: String,
    pub content_key: String,
    pub batch_bytes: u32,
    pub operations: u32,
    pub cached: bool,
    pub elapsed_ns: f64,
    pub js_ns: f64,
}

/// Run one batched hook, validating the callback before retaining its result.
pub fn run(
    compiled: &CompileResult,
    spec: &PluginSpec<'_>,
    config: &str,
    use_cache: bool,
    audit: bool,
    cache_dir: Option<&Path>,
    mut callback: impl FnMut(String) -> Result<String, String>,
) -> Result<(CompileResult, Cost), String> {
    let started = Instant::now();
    validate(spec, use_cache)?;
    // Cached operations always satisfy the deterministic-output contract.
    let audit = use_cache || audit;
    let key = cache::content_key(compiled, spec, config, audit)?;
    let mut cost = Cost {
        name: spec.name.to_owned(),
        version: spec.version.to_owned(),
        family: spec.family.to_owned(),
        content_key: key.clone(),
        batch_bytes: 0,
        operations: 0,
        cached: false,
        elapsed_ns: 0.0,
        js_ns: 0.0,
    };
    if use_cache && let Some(result) = cache::get(&key, cache_dir, compiled, spec.family) {
        cost.cached = true;
        cost.elapsed_ns = started.elapsed().as_nanos() as f64;
        return Ok((result, cost));
    }
    let batch = serde_json::json!({
        "schema": 1, "plugin": spec.name, "family": spec.family,
        "offsetEncoding": "utf8", "compiled": compiled,
    })
    .to_string();
    cost.batch_bytes = batch.len() as u32;
    let called = Instant::now();
    let response = callback(batch.clone())?;
    cost.js_ns += called.elapsed().as_nanos() as f64;
    let (result, count) = rewrite::apply(compiled, spec.family, &response)?;
    cost.operations = count;
    if audit {
        let called = Instant::now();
        let response = callback(batch)?;
        cost.js_ns += called.elapsed().as_nanos() as f64;
        let (repeated, _) = rewrite::apply(compiled, spec.family, &response)?;
        if serde_json::to_value(&result).ok() != serde_json::to_value(&repeated).ok() {
            return Err(format!(
                "plugin `{}` returned nondeterministic output",
                spec.name
            ));
        }
    }
    if use_cache {
        cache::put(key, result.clone(), response, cache_dir);
    }
    cost.elapsed_ns = started.elapsed().as_nanos() as f64;
    Ok((result, cost))
}

fn validate(spec: &PluginSpec<'_>, use_cache: bool) -> Result<(), String> {
    if [spec.name, spec.version, spec.fingerprint]
        .iter()
        .any(|value| value.trim().is_empty())
    {
        return Err("output plugin name, version and fingerprint must be nonempty".to_owned());
    }
    if !matches!(spec.family, "formatter" | "output") {
        return Err(format!("unknown output hook family `{}`", spec.family));
    }
    if use_cache && spec.inputs.is_none() {
        return Err(
            "cached output plugins must declare cacheInputs, including an empty list".to_owned(),
        );
    }
    let mut names: Vec<_> = spec
        .inputs
        .into_iter()
        .flatten()
        .map(|input| input.name.as_str())
        .collect();
    names.sort_unstable();
    if names.iter().any(|name| name.is_empty())
        || names.windows(2).any(|pair| matches!(pair, [a,b] if a == b))
    {
        return Err("output plugin cache input names must be nonempty and unique".to_owned());
    }
    Ok(())
}
