//! Real disk round trips, corrupt entries and bounded process storage.
use super::super::ProviderOutput;
use super::{MAX_BYTES, MAX_ENTRIES, ProviderCache, path};

fn output() -> (Vec<String>, ProviderOutput) {
    let groups = vec!["tokens/colors".to_owned()];
    let json = r#"{"tokens/colors":[[0,"red"]]}"#;
    let output = ProviderOutput::audit("tokens", &groups, json, json).unwrap();
    (groups, output)
}

#[test]
fn persisted_outputs_round_trip_through_a_fresh_cache_and_validate_declarations() {
    let dir = tempfile::tempdir().unwrap();
    let (groups, output) = output();
    let key = "plugin-result:012345abcdef";
    ProviderCache::default().put(key, &output, Some(dir.path()));
    let mut fresh = ProviderCache::default();
    let hit = fresh.get(key, "tokens", &groups, Some(dir.path())).unwrap();
    assert_eq!(
        (hit.values, hit.result_key),
        (output.values, output.result_key)
    );
    assert!(
        fresh
            .get(
                key,
                "tokens",
                &["tokens/other".to_owned()],
                Some(dir.path())
            )
            .is_none()
    );
    assert!(!path(dir.path(), key).unwrap().exists());
}

#[test]
fn malformed_stale_mismatched_or_tampered_entries_are_cold() {
    let dir = tempfile::tempdir().unwrap();
    let (groups, _) = output();
    let key = "plugin-result:012345abcdef";
    for json in [
        "{broken",
        r#"{"schema":2,"key":"plugin-result:012345abcdef","values":{"tokens/colors":[]},"result_key":"stale"}"#,
        r#"{"schema":1,"key":"wrong","values":{"tokens/colors":[]},"result_key":"stale"}"#,
        r#"{"schema":1,"key":"plugin-result:012345abcdef","values":{"tokens/colors":[[0,"blue"]]},"result_key":"tampered"}"#,
    ] {
        let at = path(dir.path(), key).unwrap();
        std::fs::write(&at, json).unwrap();
        assert!(
            ProviderCache::default()
                .get(key, "tokens", &groups, Some(dir.path()))
                .is_none()
        );
        assert!(!at.exists());
    }
}

#[test]
fn process_storage_is_bounded_and_evicts_the_oldest_output() {
    let (groups, output) = output();
    let mut cache = ProviderCache::default();
    for index in 0..MAX_ENTRIES + 1 {
        cache.put(&format!("plugin-result:{index:x}"), &output, None);
    }
    assert_eq!(cache.entries.len(), MAX_ENTRIES);
    assert!(cache.bytes <= MAX_BYTES);
    assert!(
        cache
            .get("plugin-result:0", "tokens", &groups, None)
            .is_none()
    );
    assert!(
        cache
            .get("plugin-result:20", "tokens", &groups, None)
            .is_some()
    );
    for index in 100..110 {
        cache.insert(&format!("large:{index:x}"), "x".repeat(4 * 1024 * 1024));
    }
    assert_eq!(cache.entries.len(), 4);
    assert_eq!(cache.bytes, MAX_BYTES);
}

#[test]
fn disk_paths_never_include_plugin_paths_or_arbitrary_key_tails() {
    let dir = tempfile::tempdir().unwrap();
    assert!(path(dir.path(), "plugin-result:../../outside").is_none());
    assert!(path(dir.path(), "plugin-result:").is_none());
    assert_eq!(
        path(dir.path(), "user/../../name:0123abcdef")
            .unwrap()
            .parent(),
        Some(dir.path())
    );
}
