use super::{
    utils::{json_from_value, text_document_position, value_to_json},
    CorsaLspClient,
};
use corsa_runtime::block_on;
use lsp_types::{
    request::{
        Completion, GotoDefinition, HoverRequest, PrepareRenameRequest, References, Rename,
        WillRenameFiles,
    },
    CompletionContext, CompletionParams, FileRename, HoverParams, PartialResultParams,
    ReferenceContext, ReferenceParams, RenameFilesParams, RenameParams, WorkDoneProgressParams,
};
use serde_json::{json, Value};
use vize_carton::{cstr, String};

impl CorsaLspClient {
    /// Get hover information at a position.
    pub fn hover(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Option<crate::LspHover>, String> {
        let value = match self.hover_raw(uri, line, character)? {
            Some(value) => value,
            None => return Ok(None),
        };

        serde_json::from_value(value)
            .map(Some)
            .map_err(|err| cstr!("Failed to parse hover response: {err}"))
    }

    pub(crate) fn hover_raw(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Option<Value>, String> {
        let params = HoverParams {
            text_document_position_params: text_document_position(uri, line, character)?,
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        let response = block_on(self.client.request::<HoverRequest>(params))
            .map_err(|e| cstr!("Failed to request hover: {e}"))?;
        self.drain_pending_messages();
        response.map(value_to_json).transpose()
    }

    pub(crate) fn definition_raw(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Option<Value>, String> {
        let response = block_on(
            self.client
                .request::<GotoDefinition>(json_from_value(json!({
                    "textDocument": {
                        "uri": uri
                    },
                    "position": {
                        "line": line,
                        "character": character
                    }
                }))?),
        )
        .map_err(|e| cstr!("Failed to request definition: {e}"))?;
        self.drain_pending_messages();
        response.map(value_to_json).transpose()
    }

    pub(crate) fn references_raw(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
        include_declaration: bool,
    ) -> Result<Option<Value>, String> {
        let params = ReferenceParams {
            text_document_position: text_document_position(uri, line, character)?,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: ReferenceContext {
                include_declaration,
            },
        };
        let response = block_on(self.client.request::<References>(params))
            .map_err(|e| cstr!("Failed to request references: {e}"))?;
        self.drain_pending_messages();
        response.map(value_to_json).transpose()
    }

    pub(crate) fn prepare_rename_raw(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Option<Value>, String> {
        let response = block_on(
            self.client
                .request::<PrepareRenameRequest>(text_document_position(uri, line, character)?),
        )
        .map_err(|e| cstr!("Failed to request prepareRename: {e}"))?;
        self.drain_pending_messages();
        response.map(value_to_json).transpose()
    }

    pub(crate) fn rename_raw(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
        new_name: &str,
    ) -> Result<Option<Value>, String> {
        let params = RenameParams {
            text_document_position: text_document_position(uri, line, character)?,
            new_name: new_name.into(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        let response = block_on(self.client.request::<Rename>(params))
            .map_err(|e| cstr!("Failed to request rename: {e}"))?;
        self.drain_pending_messages();
        response.map(value_to_json).transpose()
    }

    pub(crate) fn will_rename_files_raw(
        &mut self,
        renames: &[(&str, &str)],
    ) -> Result<Option<Value>, String> {
        let params = RenameFilesParams {
            files: renames
                .iter()
                .map(|(old_uri, new_uri)| FileRename {
                    old_uri: (*old_uri).into(),
                    new_uri: (*new_uri).into(),
                })
                .collect(),
        };
        let response = block_on(self.client.request::<WillRenameFiles>(params))
            .map_err(|e| cstr!("Failed to request willRenameFiles: {e}"))?;
        self.drain_pending_messages();
        response.map(value_to_json).transpose()
    }

    pub(crate) fn completion_raw(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Option<Value>, String> {
        let params = CompletionParams {
            text_document_position: text_document_position(uri, line, character)?,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: Some(CompletionContext {
                trigger_kind: lsp_types::CompletionTriggerKind::INVOKED,
                trigger_character: None,
            }),
        };
        let response = block_on(self.client.request::<Completion>(params))
            .map_err(|e| cstr!("Failed to request completion: {e}"))?;
        self.drain_pending_messages();
        response.map(value_to_json).transpose()
    }
}
