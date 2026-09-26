use super::*;

fn compiled() -> CompileResult {
    CompileResult {
        code: "const answer=42;".to_owned(),
        preamble: String::new(),
        ast: serde_json::json!({}),
        map: None,
        helpers: vec![],
        templates: None,
    }
}

#[test]
fn fresh_disk_entries_repeat_output_validation() {
    let directory = tempfile::tempdir().unwrap();
    let compiled = compiled();
    let spec = PluginSpec {
        name: "fresh-disk",
        version: "1",
        fingerprint: "code",
        family: "output",
        inputs: Some(&[]),
    };
    let key = content_key(&compiled, &spec, "config", true).unwrap();
    disk::write(
        directory.path(),
        &key,
        r#"[{"placement":"replace","comment":"forged"}]"#,
    );
    assert!(get(&key, Some(directory.path()), &compiled, "output").is_none());
    assert!(!disk::exists(directory.path(), &key));
    disk::write(
        directory.path(),
        &key,
        r#"[{"placement":"prepend","comment":"license"}]"#,
    );
    let hit = get(&key, Some(directory.path()), &compiled, "output").unwrap();
    assert_eq!(hit.code, "/* license */\nconst answer=42;");
}

#[test]
fn corrupt_mismatched_and_oversized_entries_are_misses() {
    let directory = tempfile::tempdir().unwrap();
    let key = "key:abc123";
    let path = directory.path().join("plugin-output-v1-abc123.json");
    for content in [
        "{torn".to_owned(),
        serde_json::json!({"schema":2,"key":key,"response":"[]"}).to_string(),
        serde_json::json!({"schema":1,"key":"different","response":"[]"}).to_string(),
        "x".repeat(3 * 1024 * 1024),
    ] {
        std::fs::write(&path, content).unwrap();
        assert!(disk::read(directory.path(), key).is_none());
        assert!(!path.exists());
    }
}

#[test]
fn memory_hit_materializes_a_new_disk_directory_without_js() {
    let directory = tempfile::tempdir().unwrap();
    let compiled = compiled();
    let spec = PluginSpec {
        name: "disk-materialization",
        version: "1",
        fingerprint: "code",
        family: "output",
        inputs: Some(&[]),
    };
    let key = content_key(&compiled, &spec, "config", true).unwrap();
    put(key.clone(), compiled.clone(), "[]".to_owned(), None);
    assert!(get(&key, Some(directory.path()), &compiled, "output").is_some());
    assert_eq!(disk::read(directory.path(), &key), Some("[]".to_owned()));
}
