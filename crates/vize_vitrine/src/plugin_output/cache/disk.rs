//! Atomic operation-response storage; fresh processes repeat native validators.
#![expect(
    clippy::disallowed_types,
    clippy::disallowed_macros,
    reason = "serialized plugin boundary uses standard strings"
)]

use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const LIMIT: usize = 2 * 1024 * 1024 + 8192;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    schema: u32,
    key: String,
    response: String,
}

fn path(dir: &Path, key: &str) -> Option<PathBuf> {
    let (_, digest) = key.rsplit_once(':')?;
    if digest.is_empty() || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some(dir.join(format!("plugin-output-v1-{digest}.json")))
}

pub(super) fn exists(dir: &Path, key: &str) -> bool {
    path(dir, key).is_some_and(|path| path.is_file())
}

pub(super) fn read(dir: &Path, key: &str) -> Option<String> {
    let path = path(dir, key)?;
    let file = File::open(&path).ok()?;
    let mut bytes = Vec::new();
    file.take((LIMIT + 1) as u64).read_to_end(&mut bytes).ok()?;
    let entry = (bytes.len() <= LIMIT)
        .then(|| serde_json::from_slice::<Entry>(&bytes).ok())
        .flatten()
        .filter(|entry| {
            entry.schema == 1 && entry.key == key && entry.response.len() <= 1024 * 1024
        });
    if let Some(entry) = entry {
        return Some(entry.response);
    }
    let _ = fs::remove_file(path);
    None
}

pub(super) fn remove(dir: &Path, key: &str) {
    if let Some(path) = path(dir, key) {
        let _ = fs::remove_file(path);
    }
}

pub(super) fn write(dir: &Path, key: &str, response: &str) {
    if response.len() > 1024 * 1024 || fs::create_dir_all(dir).is_err() {
        return;
    }
    let Some(path) = path(dir, key) else { return };
    let Ok(bytes) = serde_json::to_vec(&Entry {
        schema: 1,
        key: key.to_owned(),
        response: response.to_owned(),
    }) else {
        return;
    };
    if bytes.len() > LIMIT {
        return;
    }
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let temporary = path.with_extension(format!(
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
    if file.write_all(&bytes).is_ok() && file.sync_all().is_ok() {
        drop(file);
        if fs::rename(&temporary, &path).is_ok() {
            return;
        }
    }
    let _ = fs::remove_file(temporary);
}
