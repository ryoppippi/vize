//! File rename support for workspace import updates.
#![allow(clippy::disallowed_types, clippy::disallowed_methods)]

mod manual;

use tower_lsp::lsp_types::{
    DocumentChangeOperation, DocumentChanges, OneOf, OptionalVersionedTextDocumentIdentifier,
    RenameFilesParams, TextDocumentEdit, TextEdit, Url, WorkspaceEdit,
};

use crate::server::ServerState;

/// File rename service for import-path updates and open document state moves.
pub struct FileRenameService;

impl FileRenameService {
    /// Compute workspace edits before a file rename so clients like Neovim can apply them.
    pub async fn will_rename_files(
        state: &ServerState,
        params: &RenameFilesParams,
    ) -> Option<WorkspaceEdit> {
        let tsgo_edit = Self::tsgo_workspace_edit(state, params).await;
        let manual_edit =
            manual::collect_import_rename_edits(state, &params.files, tsgo_edit.is_some());

        merge_workspace_edits(tsgo_edit, manual_edit)
    }

    /// Update in-memory state after files were renamed on disk.
    pub async fn did_rename_files(
        state: &ServerState,
        params: &RenameFilesParams,
    ) -> Vec<(Url, Url)> {
        let renamed = manual::rename_open_documents(state, &params.files);

        #[cfg(feature = "native")]
        {
            state.invalidate_batch_cache();

            if !renamed.is_empty() && state.has_tsgo_bridge() {
                if let Some(bridge) = state.get_tsgo_bridge().await {
                    for (old_uri, _) in &renamed {
                        for request_path in [
                            crate::ide::tsgo_support::template_request_path(old_uri),
                            crate::ide::tsgo_support::script_request_path(old_uri, false),
                            crate::ide::tsgo_support::script_request_path(old_uri, true),
                        ] {
                            let request_uri =
                                crate::ide::tsgo_support::request_file_uri(&request_path);
                            let _ = bridge.close_virtual_document(&request_uri).await;
                        }
                    }
                }
            }
        }

        renamed
    }

    #[cfg(feature = "native")]
    async fn tsgo_workspace_edit(
        state: &ServerState,
        params: &RenameFilesParams,
    ) -> Option<WorkspaceEdit> {
        if !state.has_tsgo_bridge() {
            return None;
        }

        let bridge = state.get_tsgo_bridge().await?;
        let renames = params
            .files
            .iter()
            .map(|file| (file.old_uri.as_str(), file.new_uri.as_str()))
            .collect::<Vec<_>>();

        let edit = bridge.will_rename_files(&renames).await.ok()??;
        serde_json::from_value(edit).ok()
    }

    #[cfg(not(feature = "native"))]
    async fn tsgo_workspace_edit(
        _state: &ServerState,
        _params: &RenameFilesParams,
    ) -> Option<WorkspaceEdit> {
        None
    }
}

fn merge_workspace_edits(
    base: Option<WorkspaceEdit>,
    overlay: Option<WorkspaceEdit>,
) -> Option<WorkspaceEdit> {
    match (base, overlay) {
        (None, None) => None,
        (Some(edit), None) | (None, Some(edit)) => Some(edit),
        (Some(mut base), Some(mut overlay)) => {
            merge_change_sets(&mut base, &mut overlay);

            if let Some(overlay_annotations) = overlay.change_annotations.take() {
                base.change_annotations
                    .get_or_insert_with(std::collections::HashMap::new)
                    .extend(overlay_annotations);
            }

            Some(base)
        }
    }
}

fn merge_change_sets(base: &mut WorkspaceEdit, overlay: &mut WorkspaceEdit) {
    if overlay.document_changes.is_some() && base.document_changes.is_none() {
        if let Some(base_changes) = base.changes.take() {
            overlay.document_changes = merge_document_changes(
                overlay.document_changes.take(),
                Some(base_changes),
            );
        }

        base.document_changes = overlay.document_changes.take();
    } else if let Some(overlay_document_changes) = overlay.document_changes.take() {
        base.document_changes = merge_document_changes(
            base.document_changes.take().or_else(|| {
                base.changes.take().map(|changes| DocumentChanges::Edits(changes_to_document_edits(changes)))
            }),
            Some(document_changes_to_changes(overlay_document_changes)),
        );
    }

    if let Some(overlay_changes) = overlay.changes.take() {
        if base.document_changes.is_some() {
            base.document_changes =
                merge_document_changes(base.document_changes.take(), Some(overlay_changes));
            base.changes = None;
        } else {
            let base_changes = base
                .changes
                .get_or_insert_with(std::collections::HashMap::new);

            for (uri, edits) in overlay_changes {
                base_changes.entry(uri).or_default().extend(edits);
            }
        }
    }
}

fn merge_document_changes(
    current: Option<DocumentChanges>,
    additional_changes: Option<std::collections::HashMap<Url, Vec<TextEdit>>>,
) -> Option<DocumentChanges> {
    let Some(additional_changes) = additional_changes else {
        return current;
    };

    let additional_edits = changes_to_document_edits(additional_changes);
    if additional_edits.is_empty() {
        return current;
    }

    match current {
        Some(DocumentChanges::Edits(mut edits)) => {
            edits.extend(additional_edits);
            Some(DocumentChanges::Edits(edits))
        }
        Some(DocumentChanges::Operations(mut operations)) => {
            let insert_at = operations
                .iter()
                .position(|operation| matches!(operation, DocumentChangeOperation::Op(_)))
                .unwrap_or(operations.len());

            let additional_operations = additional_edits
                .into_iter()
                .map(DocumentChangeOperation::Edit);

            operations.splice(insert_at..insert_at, additional_operations);
            Some(DocumentChanges::Operations(operations))
        }
        None => Some(DocumentChanges::Edits(additional_edits)),
    }
}

fn changes_to_document_edits(
    changes: std::collections::HashMap<Url, Vec<TextEdit>>,
) -> Vec<TextDocumentEdit> {
    changes
        .into_iter()
        .map(|(uri, edits)| TextDocumentEdit {
            text_document: OptionalVersionedTextDocumentIdentifier { uri, version: None },
            edits: edits.into_iter().map(OneOf::Left).collect(),
        })
        .collect()
}

fn document_changes_to_changes(
    document_changes: DocumentChanges,
) -> std::collections::HashMap<Url, Vec<TextEdit>> {
    let mut changes = std::collections::HashMap::new();

    match document_changes {
        DocumentChanges::Edits(edits) => {
            for edit in edits {
                let uri = edit.text_document.uri;
                let entry = changes.entry(uri).or_insert_with(Vec::new);

                for edit in edit.edits {
                    match edit {
                        OneOf::Left(edit) => entry.push(edit),
                        OneOf::Right(annotated) => entry.push(annotated.text_edit),
                    }
                }
            }
        }
        DocumentChanges::Operations(operations) => {
            for operation in operations {
                if let DocumentChangeOperation::Edit(edit) = operation {
                    let uri = edit.text_document.uri;
                    let entry = changes.entry(uri).or_insert_with(Vec::new);

                    for edit in edit.edits {
                        match edit {
                            OneOf::Left(edit) => entry.push(edit),
                            OneOf::Right(annotated) => entry.push(annotated.text_edit),
                        }
                    }
                }
            }
        }
    }

    changes
}

#[cfg(test)]
mod tests {
    use tower_lsp::lsp_types::{
        DocumentChanges, OneOf, OptionalVersionedTextDocumentIdentifier, Position, Range,
        TextDocumentEdit, TextEdit,
    };

    use super::merge_workspace_edits;

    fn text_edit(new_text: &str) -> TextEdit {
        TextEdit {
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 0,
                },
            },
            new_text: new_text.to_string(),
        }
    }

    #[test]
    fn merges_changes_into_document_changes() {
        let base_uri = Url::parse("file:///base.vue").unwrap();
        let overlay_uri = Url::parse("file:///overlay.vue").unwrap();
        let merged = merge_workspace_edits(
            Some(WorkspaceEdit {
                changes: None,
                document_changes: Some(DocumentChanges::Edits(vec![TextDocumentEdit {
                    text_document: OptionalVersionedTextDocumentIdentifier {
                        uri: base_uri.clone(),
                        version: None,
                    },
                    edits: vec![OneOf::Left(text_edit("from-tsgo"))],
                }])),
                change_annotations: None,
            }),
            Some(WorkspaceEdit {
                changes: Some(std::collections::HashMap::from([(
                    overlay_uri.clone(),
                    vec![text_edit("from-manual")],
                )])),
                document_changes: None,
                change_annotations: None,
            }),
        )
        .unwrap();

        assert!(merged.changes.is_none());

        let DocumentChanges::Edits(edits) = merged.document_changes.unwrap() else {
            panic!("expected document edits");
        };

        assert_eq!(edits.len(), 2);
        assert!(edits.iter().any(|edit| edit.text_document.uri == base_uri));
        assert!(edits.iter().any(|edit| edit.text_document.uri == overlay_uri));
    }

    #[test]
    fn prefers_overlay_document_changes_over_base_changes() {
        let base_uri = Url::parse("file:///base.vue").unwrap();
        let overlay_uri = Url::parse("file:///overlay.vue").unwrap();
        let merged = merge_workspace_edits(
            Some(WorkspaceEdit {
                changes: Some(std::collections::HashMap::from([(
                    base_uri.clone(),
                    vec![text_edit("from-base")],
                )])),
                document_changes: None,
                change_annotations: None,
            }),
            Some(WorkspaceEdit {
                changes: None,
                document_changes: Some(DocumentChanges::Edits(vec![TextDocumentEdit {
                    text_document: OptionalVersionedTextDocumentIdentifier {
                        uri: overlay_uri.clone(),
                        version: None,
                    },
                    edits: vec![OneOf::Left(text_edit("from-overlay"))],
                }])),
                change_annotations: None,
            }),
        )
        .unwrap();

        assert!(merged.changes.is_none());

        let DocumentChanges::Edits(edits) = merged.document_changes.unwrap() else {
            panic!("expected document edits");
        };

        assert_eq!(edits.len(), 2);
        assert!(edits.iter().any(|edit| edit.text_document.uri == base_uri));
        assert!(edits.iter().any(|edit| edit.text_document.uri == overlay_uri));
    }
}
