use super::LspDiagnostic;
use lsp_types::{Diagnostic, Position, TextDocumentIdentifier, TextDocumentPositionParams, Uri};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use std::str::FromStr;
use vize_carton::{cstr, String};

pub(super) fn parse_uri(uri: &str) -> Result<Uri, String> {
    Uri::from_str(uri).map_err(|e| cstr!("Invalid URI `{uri}`: {e}"))
}

pub(super) fn text_document_position(
    uri: &str,
    line: u32,
    character: u32,
) -> Result<TextDocumentPositionParams, String> {
    Ok(TextDocumentPositionParams::new(
        TextDocumentIdentifier::new(parse_uri(uri)?),
        Position::new(line, character),
    ))
}

pub(super) fn json_from_value<T>(value: Value) -> Result<T, String>
where
    T: DeserializeOwned,
{
    serde_json::from_value(value).map_err(|e| cstr!("Failed to decode JSON params: {e}"))
}

pub(super) fn value_to_json<T>(value: T) -> Result<Value, String>
where
    T: Serialize,
{
    serde_json::to_value(value).map_err(|e| cstr!("Failed to encode JSON response: {e}"))
}

pub(super) fn convert_diagnostics(diagnostics: &[Diagnostic]) -> Vec<LspDiagnostic> {
    diagnostics
        .iter()
        .filter_map(|diagnostic| {
            serde_json::to_value(diagnostic)
                .ok()
                .and_then(|value| serde_json::from_value(value).ok())
        })
        .collect()
}
