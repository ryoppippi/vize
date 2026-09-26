use super::*;
#[test]
fn fixes_use_host_spans_and_unknown_report_fields_refuse() {
    let source = "<template>日本語😀<button :disabled=\"true\" /></template>";
    let document = PluginDocument::build(source, "Button.vue").unwrap();
    let binding = document
        .nodes
        .iter()
        .find(|node| node.kind == "ui.bind")
        .unwrap();
    let reports = serde_json::json!([{"rule": "boolean", "node": binding.id, "message": "Use literal", "fix": "disabled"}]).to_string();
    let found = diagnostics(&document, "team", &reports).unwrap();
    let [diagnostic] = found.as_slice() else {
        panic!("one diagnostic")
    };
    assert_eq!(diagnostic.fix.as_deref(), Some("disabled"));
    assert_eq!(
        source.get(diagnostic.start as usize..diagnostic.end as usize),
        Some(":disabled=\"true\"")
    );
    assert_eq!(diagnostic.line, 1);
    let invalid =
        serde_json::json!([{"rule": "boolean", "node": binding.id, "message": "x", "start": 0}])
            .to_string();
    assert!(diagnostics(&document, "team", &invalid).is_err());
}
#[test]
fn malformed_and_oversized_report_arrays_refuse() {
    let document = PluginDocument::build("<template>Hi</template>", "Hi.vue").unwrap();
    for reports in [
        "null",
        "{}",
        "[{\"rule\":\"x\",\"node\":0,\"message\":\"x\",\"fix\":{}}]",
    ] {
        assert!(diagnostics(&document, "team", reports).is_err());
    }
    let reports = serde_json::json!(vec![
        serde_json::json!({"rule":"x","node":0,"message":"x"});
        4097
    ])
    .to_string();
    assert!(diagnostics(&document, "team", &reports).is_err());
}

#[test]
fn poisoned_cached_anchors_refuse_before_they_can_cross_to_js() {
    let document = PluginDocument::build("<template>Hi</template>", "Hi.vue").unwrap();
    let mut found = diagnostics(
        &document,
        "team",
        r#"[{"rule":"x","node":0,"message":"x"}]"#,
    )
    .unwrap();
    assert!(valid_cached(&document, "team", &found));
    let Some(diagnostic) = found.first_mut() else {
        panic!("one diagnostic")
    };
    diagnostic.end = u32::MAX;
    assert!(!valid_cached(&document, "team", &found));
}
