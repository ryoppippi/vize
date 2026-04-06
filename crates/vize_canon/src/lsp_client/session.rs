#![allow(clippy::disallowed_types)]

use super::CorsaLspClient;
use corsa::{
    api::{
        ApiMode, ApiSpawnConfig, CapabilitiesResponse, DocumentIdentifier, OverlayChanges,
        OverlayUpdate, ProjectSession,
    },
    fast::CompactString,
};
use corsa_runtime::block_on;
use std::{path::Path, sync::Arc};
use vize_carton::{cstr, String};

pub(super) fn spawn_project_session(
    executable: &str,
    cwd: &Path,
    project_root: &Path,
) -> Result<(ProjectSession, Arc<CapabilitiesResponse>), String> {
    let config_path = project_root.join("tsconfig.json");
    let config_path_wire = config_path.to_string_lossy();
    let session = block_on(ProjectSession::spawn(
        ApiSpawnConfig::new(executable)
            .with_mode(ApiMode::SyncMsgpackStdio)
            .with_cwd(cwd),
        config_path_wire.as_ref(),
        None,
    ))
    .map_err(|error| cstr!("Failed to start Corsa API session: {error}"))?;
    let capabilities = block_on(session.describe_capabilities())
        .unwrap_or_else(|_| Arc::new(CapabilitiesResponse::default()));
    Ok((session, capabilities))
}

pub(super) fn uri_document_identifier(uri: &str) -> DocumentIdentifier {
    DocumentIdentifier::Uri {
        uri: CompactString::from(uri),
    }
}

impl CorsaLspClient {
    pub(super) fn supports_overlay_api(&self) -> bool {
        self.capabilities.overlay.update_snapshot_overlay_changes
    }

    pub(super) fn supports_project_diagnostics_api(&self) -> bool {
        self.capabilities.diagnostics.project
    }

    pub(super) fn supports_file_diagnostics_api(&self) -> bool {
        self.capabilities.diagnostics.file
    }

    pub(super) fn supports_hover_api(&self) -> bool {
        self.capabilities.editor.hover
    }

    pub(super) fn supports_definition_api(&self) -> bool {
        self.capabilities.editor.definition
    }

    pub(super) fn supports_references_api(&self) -> bool {
        self.capabilities.editor.references
    }

    pub(super) fn supports_rename_api(&self) -> bool {
        self.capabilities.editor.rename
    }

    pub(super) fn supports_completion_api(&self) -> bool {
        self.capabilities.editor.completion
    }

    pub(super) fn can_use_api_for_uri(&self, uri: &str) -> bool {
        !self.document_texts.contains_key(uri) || self.supports_overlay_api()
    }

    pub(super) fn sync_overlay_document(&mut self, uri: &str, content: &str) -> Result<(), String> {
        self.document_texts.insert(uri.into(), content.into());
        if !self.supports_overlay_api() {
            return Ok(());
        }

        let version = next_overlay_version(&mut self.overlay_versions, uri);
        block_on(self.session.refresh_with_overlay_changes(
            None,
            Some(OverlayChanges {
                upsert: vec![OverlayUpdate {
                    document: uri_document_identifier(uri),
                    text: content.into(),
                    version: Some(version),
                    language_id: Some("typescript".into()),
                }],
                delete: Vec::new(),
            }),
        ))
        .map_err(|error| cstr!("Failed to sync Corsa overlay: {error}"))
    }

    pub(super) fn delete_overlay_document(&mut self, uri: &str) -> Result<(), String> {
        self.document_texts.remove(uri);
        self.overlay_versions.remove(uri);
        if !self.supports_overlay_api() {
            return Ok(());
        }

        block_on(self.session.refresh_with_overlay_changes(
            None,
            Some(OverlayChanges {
                upsert: Vec::new(),
                delete: vec![uri_document_identifier(uri)],
            }),
        ))
        .map_err(|error| cstr!("Failed to remove Corsa overlay: {error}"))
    }

    pub(super) fn utf16_offset_for(&self, uri: &str, line: u32, character: u32) -> Option<u32> {
        self.document_texts
            .get(uri)
            .map(|content| line_character_to_utf16_offset(content.as_str(), line, character))
            .or_else(|| {
                load_file_text(uri)
                    .as_deref()
                    .map(|content| line_character_to_utf16_offset(content, line, character))
            })
    }
}

fn next_overlay_version(versions: &mut vize_carton::FxHashMap<String, i32>, uri: &str) -> i32 {
    let next = versions.get(uri).copied().unwrap_or(0).saturating_add(1);
    versions.insert(uri.into(), next);
    next
}

fn load_file_text(uri: &str) -> Option<String> {
    let path = uri.strip_prefix("file://")?;
    std::fs::read_to_string(path).ok().map(Into::into)
}

pub(super) fn line_character_to_utf16_offset(text: &str, line: u32, character: u32) -> u32 {
    let mut offset = 0u32;
    let mut lines = text.split_inclusive('\n');

    for _ in 0..line {
        let Some(segment) = lines.next() else {
            return text.encode_utf16().count() as u32;
        };
        offset += segment.encode_utf16().count() as u32;
    }

    let Some(segment) = lines.next() else {
        return text.encode_utf16().count() as u32;
    };
    let line_without_break = segment.strip_suffix('\n').unwrap_or(segment);
    let line_len = line_without_break.encode_utf16().count() as u32;
    offset + character.min(line_len)
}

#[cfg(test)]
mod tests {
    use super::{line_character_to_utf16_offset, uri_document_identifier};
    use corsa::api::DocumentIdentifier;

    #[test]
    fn utf16_offsets_clamp_to_line_boundaries() {
        assert_eq!(line_character_to_utf16_offset("alpha\nbeta", 0, 99), 5);
        assert_eq!(line_character_to_utf16_offset("a😀b", 0, 3), 3);
        assert_eq!(line_character_to_utf16_offset("a\nb", 9, 0), 3);
    }

    #[test]
    fn api_queries_use_uri_document_identifiers() {
        assert!(matches!(
            uri_document_identifier("file:///workspace/App.vue.ts"),
            DocumentIdentifier::Uri { .. }
        ));
    }
}
