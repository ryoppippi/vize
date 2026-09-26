//! Bounded process reuse and atomic, validated provider output persistence.
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, VecDeque};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use super::ProviderOutput;

const CACHE_SCHEMA: u32 = 1;
const MAX_ENTRIES: usize = 32;
const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_FILE_BYTES: usize = 4 * 1024 * 1024 + 4096;

#[derive(Default)]
pub struct ProviderCache {
    entries: BTreeMap<String, String>,
    order: VecDeque<String>,
    bytes: usize,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CacheFile {
    schema: u32,
    key: String,
    values: Map<String, Value>,
    result_key: String,
}

impl ProviderCache {
    pub fn get(
        &mut self,
        key: &str,
        plugin: &str,
        provides: &[String],
        dir: Option<&Path>,
    ) -> Option<ProviderOutput> {
        let memory = self.entries.get(key).cloned();
        let json = memory.clone().or_else(|| {
            let at = path(dir?, key)?;
            let mut bytes = Vec::new();
            File::open(at)
                .ok()?
                .take((MAX_FILE_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
                .ok()?;
            (bytes.len() <= MAX_FILE_BYTES)
                .then(|| String::from_utf8(bytes).ok())
                .flatten()
        })?;
        let valid = || {
            let file: CacheFile = serde_json::from_str(&json).ok()?;
            if file.schema != CACHE_SCHEMA || file.key != key {
                return None;
            }
            let encoded = serde_json::to_string(&file.values).ok()?;
            let output = ProviderOutput::audit(plugin, provides, &encoded, &encoded).ok()?;
            (output.result_key == file.result_key).then_some(output)
        };
        let Some(output) = valid() else {
            self.remove(key);
            if let Some(at) = dir.and_then(|dir| path(dir, key)) {
                let _ = fs::remove_file(at);
            }
            return None;
        };
        if memory.is_none() {
            self.insert(key, json.clone());
        }
        if let Some(dir) = dir
            && !path(dir, key).is_some_and(|at| at.is_file())
        {
            write(dir, key, &json);
        }
        Some(output)
    }

    pub fn put(&mut self, key: &str, output: &ProviderOutput, dir: Option<&Path>) {
        let file = CacheFile {
            schema: CACHE_SCHEMA,
            key: key.to_owned(),
            values: output.values.clone(),
            result_key: output.result_key.clone(),
        };
        let Ok(json) = serde_json::to_string(&file) else {
            return;
        };
        if json.len() > MAX_FILE_BYTES {
            return;
        }
        if let Some(dir) = dir {
            write(dir, key, &json);
        }
        self.insert(key, json);
    }

    fn insert(&mut self, key: &str, json: String) {
        if json.len() > MAX_BYTES {
            return;
        }
        self.remove(key);
        while self.entries.len() >= MAX_ENTRIES || self.bytes + json.len() > MAX_BYTES {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if let Some(old) = self.entries.remove(&oldest) {
                self.bytes = self.bytes.saturating_sub(old.len());
            }
        }
        self.bytes += json.len();
        self.order.push_back(key.to_owned());
        self.entries.insert(key.to_owned(), json);
    }

    fn remove(&mut self, key: &str) {
        if let Some(old) = self.entries.remove(key) {
            self.bytes = self.bytes.saturating_sub(old.len());
        }
        self.order.retain(|stored| stored != key);
    }
}

fn path(dir: &Path, key: &str) -> Option<PathBuf> {
    let (_, digest) = key.rsplit_once(':')?;
    if digest.is_empty() || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some(dir.join(format!("plugin-facts-v{CACHE_SCHEMA}-{digest}.json")))
}

fn write(dir: &Path, key: &str, json: &str) {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let Some(at) = path(dir, key) else {
        return;
    };
    if fs::create_dir_all(dir).is_err() {
        return;
    }
    let temporary = at.with_extension(format!(
        "{}.{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let Ok(mut file) = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
    else {
        return;
    };
    if file.write_all(json.as_bytes()).is_ok() && file.sync_all().is_ok() {
        drop(file);
        if fs::rename(&temporary, at).is_ok() {
            return;
        }
    }
    let _ = fs::remove_file(temporary);
}

pub fn cache() -> &'static Mutex<ProviderCache> {
    static CACHE: OnceLock<Mutex<ProviderCache>> = OnceLock::new();
    CACHE.get_or_init(Mutex::default)
}

#[cfg(test)]
mod tests;
