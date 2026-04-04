use super::{paths::resolve_temp_dir_base, utils::convert_diagnostics};
use lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

fn unique_case_dir(name: &str) -> PathBuf {
    static NEXT_CASE_ID: AtomicUsize = AtomicUsize::new(0);

    let case_id = NEXT_CASE_ID.fetch_add(1, Ordering::Relaxed);
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("__agent_only")
        .join("tests")
        .join(format!(
            "corsa-temp-dir-{name}-{}-{case_id}",
            std::process::id()
        ))
}

#[test]
fn resolves_temp_dir_under_package_root_when_node_modules_exists() {
    let case_dir = unique_case_dir("package-root");
    let source_dir = case_dir.join("playground").join("src").join("shared");
    let node_modules_vue = case_dir.join("playground").join("node_modules").join("vue");

    let _ = fs::remove_dir_all(&case_dir);
    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&node_modules_vue).unwrap();

    let resolved = resolve_temp_dir_base(Some(&source_dir));

    assert_eq!(
        resolved,
        case_dir
            .join("playground")
            .join("__agent_only")
            .join("vize-corsa")
    );

    let _ = fs::remove_dir_all(&case_dir);
}

#[test]
fn falls_back_to_nearest_available_node_modules_root() {
    let case_dir = unique_case_dir("fallback");
    let workspace_root = case_dir.join("workspace");
    let source_dir = workspace_root.join("playground").join("src").join("shared");
    let node_modules_vue = workspace_root.join("node_modules").join("vue");

    let _ = fs::remove_dir_all(&case_dir);
    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&node_modules_vue).unwrap();

    let resolved = resolve_temp_dir_base(Some(&source_dir));

    assert_eq!(
        resolved,
        workspace_root.join("__agent_only").join("vize-corsa")
    );
    assert!(!resolved.starts_with(&source_dir));

    let _ = fs::remove_dir_all(&case_dir);
}

#[test]
fn converts_lsp_diagnostics_to_legacy_shape() {
    let diagnostics = vec![Diagnostic {
        range: Range::new(Position::new(1, 2), Position::new(3, 4)),
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String("TS2322".into())),
        code_description: None,
        source: Some("ts".into()),
        message: "broken".into(),
        related_information: None,
        tags: None,
        data: None,
    }];

    let converted = convert_diagnostics(&diagnostics);

    assert_eq!(converted.len(), 1);
    assert_eq!(converted[0].range.start.line, 1);
    assert_eq!(converted[0].range.start.character, 2);
    assert_eq!(converted[0].message, "broken");
    assert_eq!(
        converted[0].code,
        Some(serde_json::Value::String("TS2322".into()))
    );
}
