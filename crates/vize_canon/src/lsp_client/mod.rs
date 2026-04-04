//! LSP Client for tsgo.
//!
//! This module preserves the legacy `TsgoLspClient` surface used across the
//! workspace while delegating process management and virtual document syncing to
//! `corsa-bind`'s `corsa_lsp`.

use corsa_lsp::{LspClient, LspOverlay, LspSpawnConfig, VirtualChange, VirtualDocument};
use corsa_runtime::{block_on, spawn, BroadcastReceiver};
use lsp_types::{
    notification::{Exit, Initialized, Notification, PublishDiagnostics},
    request::{
        Completion, DocumentDiagnosticRequest, GotoDefinition, HoverRequest, Initialize,
        PrepareRenameRequest, References, Rename, Shutdown, WillRenameFiles,
    },
    CompletionContext, CompletionParams, Diagnostic, DocumentDiagnosticParams,
    DocumentDiagnosticReport, DocumentDiagnosticReportKind, DocumentDiagnosticReportResult,
    FileRename, HoverParams, InitializeParams, InitializedParams, PartialResultParams, Position,
    PublishDiagnosticsParams, ReferenceContext, ReferenceParams, RenameFilesParams, RenameParams,
    TextDocumentIdentifier, TextDocumentPositionParams, Uri, WorkDoneProgressParams,
    WorkspaceFolder,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    str::FromStr,
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};
use vize_carton::{cstr, FxHashMap, String, ToCompactString};

/// LSP Client for tsgo
pub struct TsgoLspClient {
    client: LspClient,
    overlay: LspOverlay,
    events: BroadcastReceiver<corsa_lsp::jsonrpc::InboundEvent>,
    /// Pending diagnostics received via publishDiagnostics
    pub(crate) diagnostics: FxHashMap<String, Vec<Diagnostic>>,
    diagnostic_result_ids: FxHashMap<String, String>,
    /// Temporary directory for tsconfig.json (cleaned up on drop)
    temp_dir: Option<PathBuf>,
    closed: bool,
}

/// LSP Diagnostic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspDiagnostic {
    pub range: LspRange,
    pub severity: Option<i32>,
    pub code: Option<Value>,
    pub source: Option<String>,
    pub message: String,
}

/// LSP Range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

/// LSP Position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspPosition {
    pub line: u32,
    pub character: u32,
}

pub(crate) struct DiagnosticFetch {
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) used_cache: bool,
}

fn find_node_modules_with_vue(start: &Path) -> Option<PathBuf> {
    let mut dir = start;
    loop {
        let node_modules = dir.join("node_modules");
        if node_modules.join("vue").is_dir() {
            return Some(node_modules);
        }
        dir = dir.parent()?;
    }
}

fn resolve_temp_dir_base(project_root: Option<&Path>) -> PathBuf {
    let fallback_root = project_root
        .map(Path::to_path_buf)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));

    find_node_modules_with_vue(&fallback_root)
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .unwrap_or(fallback_root)
        .join("__agent_only")
        .join("vize-tsgo")
}

impl TsgoLspClient {
    /// Start tsgo LSP server.
    pub fn new(tsgo_path: Option<&str>, working_dir: Option<&str>) -> Result<Self, String> {
        let tsgo: String = tsgo_path
            .map(String::from)
            .or_else(|| std::env::var("TSGO_PATH").ok().map(String::from))
            .or_else(|| Self::find_tsgo_in_local_node_modules(working_dir))
            .or_else(Self::find_tsgo_in_common_locations)
            .unwrap_or_else(|| "tsgo".into());

        eprintln!("\x1b[90m[tsgo] Using: {tsgo}\x1b[0m");

        let project_root = working_dir
            .map(PathBuf::from)
            .or_else(|| std::env::current_dir().ok())
            .and_then(|path| path.canonicalize().ok());

        static NEXT_CLIENT_ID: AtomicUsize = AtomicUsize::new(0);

        let client_id = NEXT_CLIENT_ID.fetch_add(1, Ordering::Relaxed);
        let temp_dir_base = resolve_temp_dir_base(project_root.as_deref());
        let temp_dir_path = temp_dir_base.join(&*cstr!("{}-{}", std::process::id(), client_id));

        let _ = std::fs::remove_dir_all(&temp_dir_path);
        std::fs::create_dir_all(&temp_dir_path)
            .map_err(|e| cstr!("Failed to create agent directory: {e}"))?;

        let node_modules_path = project_root.as_deref().and_then(find_node_modules_with_vue);
        if let Some(ref node_modules_path) = node_modules_path {
            let symlink_target = temp_dir_path.join("node_modules");
            let _ = std::fs::remove_file(&symlink_target);
            #[cfg(unix)]
            {
                let _ = std::os::unix::fs::symlink(node_modules_path, &symlink_target);
            }
            #[cfg(windows)]
            {
                let _ = std::os::windows::fs::symlink_dir(node_modules_path, &symlink_target);
            }
        }

        let tsconfig_content = json!({
            "compilerOptions": {
                "target": "ES2022",
                "module": "ESNext",
                "moduleResolution": "bundler",
                "lib": ["ES2022", "DOM", "DOM.Iterable"],
                "strict": true,
                "noEmit": true,
                "skipLibCheck": true
            }
        });
        std::fs::write(
            temp_dir_path.join("tsconfig.json"),
            tsconfig_content.to_compact_string(),
        )
        .map_err(|e| cstr!("Failed to write temp tsconfig.json: {e}"))?;

        let client = block_on(LspClient::spawn(
            LspSpawnConfig::new(tsgo.as_str()).with_cwd(temp_dir_path.clone()),
        ))
        .map_err(|e| cstr!("Failed to start tsgo LSP: {e}"))?;
        let overlay = client.overlay();
        let events = client.subscribe();

        let temp_root = temp_dir_path.canonicalize().ok();
        let mut client = Self {
            client,
            overlay,
            events,
            diagnostics: FxHashMap::default(),
            diagnostic_result_ids: FxHashMap::default(),
            temp_dir: Some(temp_dir_path),
            closed: false,
        };
        client.initialize(temp_root.as_ref())?;
        client.drain_pending_messages();

        Ok(client)
    }

    fn initialize(&mut self, project_root: Option<&PathBuf>) -> Result<(), String> {
        let root_uri = project_root
            .map(|path| cstr!("file://{}", path.display()))
            .map(|uri| parse_uri(uri.as_str()))
            .transpose()?;
        let workspace_folders = root_uri.clone().map(|uri| {
            vec![WorkspaceFolder {
                uri,
                name: "workspace".into(),
            }]
        });
        let params: InitializeParams = json_from_value(json!({
            "processId": std::process::id(),
            "capabilities": {
                "textDocument": {
                    "publishDiagnostics": {
                        "relatedInformation": true
                    },
                    "diagnostic": {
                        "dynamicRegistration": false
                    }
                },
                "workspace": {
                    "workspaceFolders": true,
                    "configuration": true
                }
            },
            "rootUri": root_uri,
            "workspaceFolders": workspace_folders
        }))?;

        let _response = block_on(self.client.request::<Initialize>(params))
            .map_err(|e| cstr!("Failed to initialize tsgo LSP: {e}"))?;
        self.client
            .notify::<Initialized>(InitializedParams {})
            .map_err(|e| cstr!("Failed to send initialized notification: {e}"))?;

        Ok(())
    }

    /// Shutdown the LSP server.
    pub fn shutdown(&mut self) -> Result<(), String> {
        if self.closed {
            return Ok(());
        }

        self.drain_pending_messages();
        let _ = block_on(self.client.request::<Shutdown>(()));
        let _ = self.client.notify::<Exit>(());
        block_on(self.client.close()).map_err(|e| cstr!("Failed to close tsgo LSP: {e}"))?;
        self.closed = true;
        Ok(())
    }

    /// Open a virtual document (waits briefly for diagnostics).
    pub fn did_open(&mut self, uri: &str, content: &str) -> Result<(), String> {
        self.did_open_fast(uri, content)?;
        self.read_notifications()?;
        Ok(())
    }

    /// Open a virtual document without waiting for diagnostics.
    pub fn did_open_fast(&mut self, uri: &str, content: &str) -> Result<(), String> {
        let uri = parse_uri(uri)?;
        self.clear_document_state(uri.as_str());

        if self.overlay.document(&uri).is_some() {
            self.overlay
                .replace(&uri, content)
                .map_err(|e| cstr!("Failed to update virtual document: {e}"))?;
        } else {
            self.overlay
                .open(VirtualDocument::new(uri, "typescript", content))
                .map_err(|e| cstr!("Failed to open virtual document: {e}"))?;
        }

        self.drain_pending_messages();
        Ok(())
    }

    /// Update an already-open virtual document.
    pub fn did_change(&mut self, uri: &str, content: &str) -> Result<(), String> {
        let uri = parse_uri(uri)?;
        self.clear_document_state(uri.as_str());

        if self.overlay.document(&uri).is_some() {
            self.overlay
                .change(&uri, [VirtualChange::replace(content)])
                .map_err(|e| cstr!("Failed to change virtual document: {e}"))?;
        } else {
            self.overlay
                .open(VirtualDocument::new(uri, "typescript", content))
                .map_err(|e| cstr!("Failed to open virtual document: {e}"))?;
        }

        self.drain_pending_messages();
        Ok(())
    }

    /// Close a virtual document.
    pub fn did_close(&mut self, uri: &str) -> Result<(), String> {
        let uri = parse_uri(uri)?;
        self.overlay
            .close(&uri)
            .map_err(|e| cstr!("Failed to close virtual document: {e}"))?;
        self.clear_document_state(uri.as_str());
        Ok(())
    }

    /// Get cached diagnostics for a URI.
    pub fn get_diagnostics(&self, uri: &str) -> Vec<LspDiagnostic> {
        self.diagnostics
            .get(uri)
            .map(|diagnostics| convert_diagnostics(diagnostics))
            .unwrap_or_default()
    }

    /// Request diagnostics using `textDocument/diagnostic`.
    pub fn request_diagnostics(&mut self, uri: &str) -> Result<Vec<LspDiagnostic>, String> {
        self.request_diagnostics_full(uri)
            .map(|fetch| convert_diagnostics(&fetch.diagnostics))
    }

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

    /// Request diagnostics for multiple URIs in batch.
    pub fn request_diagnostics_batch(
        &mut self,
        uris: &[String],
    ) -> Vec<(String, Vec<LspDiagnostic>)> {
        let mut handles = Vec::with_capacity(uris.len());

        for uri in uris {
            let uri_string = uri.clone();
            let previous_result_id = self
                .diagnostic_result_ids
                .get(uri.as_str())
                .cloned()
                .map(Into::into);
            let client = self.client.clone();
            handles.push(spawn(async move {
                let uri = parse_uri(uri_string.as_str())
                    .map_err(|err| cstr!("Invalid diagnostic URI `{uri_string}`: {err}"))?;
                let params = DocumentDiagnosticParams {
                    text_document: TextDocumentIdentifier::new(uri),
                    identifier: None,
                    previous_result_id,
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                };
                let response = client
                    .request::<DocumentDiagnosticRequest>(params)
                    .await
                    .map_err(|err| cstr!("{err}"));
                Ok::<_, String>((uri_string, response))
            }));
        }

        let mut joined = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.join() {
                Ok(Ok((uri, response))) => joined.push((uri, response)),
                Ok(Err(err)) => {
                    eprintln!("\x1b[90m[tsgo] batch request skipped: {err}\x1b[0m");
                }
                Err(_) => {}
            }
        }

        self.collect_events_until_idle(Duration::from_millis(5), Duration::from_millis(30));

        let mut results = Vec::with_capacity(uris.len());
        for (uri, response) in joined {
            let diagnostics = match response {
                Ok(report) => self
                    .apply_document_diagnostic_result(uri.as_str(), report)
                    .unwrap_or_else(|| self.cached_diagnostics(uri.as_str())),
                Err(_) => self.cached_diagnostics(uri.as_str()),
            };
            results.push((uri, convert_diagnostics(&diagnostics)));
        }

        results
    }

    pub(crate) fn request_diagnostics_full(
        &mut self,
        uri: &str,
    ) -> Result<DiagnosticFetch, String> {
        let uri = parse_uri(uri)?;
        let previous_result_id = self
            .diagnostic_result_ids
            .get(uri.as_str())
            .cloned()
            .map(Into::into);
        let params = DocumentDiagnosticParams {
            text_document: TextDocumentIdentifier::new(uri.clone()),
            identifier: None,
            previous_result_id,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let response = block_on(self.client.request::<DocumentDiagnosticRequest>(params));
        self.drain_pending_messages();

        match response {
            Ok(report) => {
                if let Some(diagnostics) =
                    self.apply_document_diagnostic_result(uri.as_str(), report.clone())
                {
                    let used_cache = matches!(
                        report,
                        DocumentDiagnosticReportResult::Report(
                            DocumentDiagnosticReport::Unchanged(_)
                        ) | DocumentDiagnosticReportResult::Partial(_)
                    );
                    return Ok(DiagnosticFetch {
                        diagnostics,
                        used_cache,
                    });
                }
            }
            Err(error) => {
                if self.wait_for_uri_diagnostics(uri.as_str(), Duration::from_millis(500)) {
                    return Ok(DiagnosticFetch {
                        diagnostics: self.cached_diagnostics(uri.as_str()),
                        used_cache: true,
                    });
                }
                return Err(cstr!("Failed to request diagnostics: {error}"));
            }
        }

        if self.wait_for_uri_diagnostics(uri.as_str(), Duration::from_millis(100)) {
            return Ok(DiagnosticFetch {
                diagnostics: self.cached_diagnostics(uri.as_str()),
                used_cache: true,
            });
        }

        Ok(DiagnosticFetch {
            diagnostics: self.cached_diagnostics(uri.as_str()),
            used_cache: true,
        })
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

    pub(crate) fn diagnostics_cache_len(&self) -> usize {
        self.diagnostics.len()
    }

    pub(crate) fn clear_diagnostics_cache(&mut self) {
        self.diagnostics.clear();
        self.diagnostic_result_ids.clear();
    }

    pub(crate) fn drain_pending_messages(&mut self) {
        self.collect_events_until_idle(Duration::ZERO, Duration::ZERO);
    }

    pub(crate) fn read_notifications(&mut self) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_millis(50);
        while Instant::now() < deadline {
            match self.events.recv_timeout(Duration::from_millis(1)) {
                Ok(event) => {
                    let saw_publish = matches!(
                        event,
                        corsa_lsp::jsonrpc::InboundEvent::Notification { ref method, .. }
                            if method.as_str() == PublishDiagnostics::METHOD
                    );
                    self.handle_event(event);
                    if saw_publish {
                        break;
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        Ok(())
    }

    pub fn wait_for_diagnostics(&mut self, expected_count: usize) {
        let max_wait = Duration::from_secs(30);
        let idle_timeout = Duration::from_millis(30);
        let start = Instant::now();
        let initial_count = self.diagnostics.len();
        let mut last_message: Option<Instant> = None;

        loop {
            if start.elapsed() > max_wait {
                break;
            }

            if self.diagnostics.len().saturating_sub(initial_count) >= expected_count {
                self.collect_events_until_idle(Duration::from_millis(1), Duration::from_millis(5));
                break;
            }

            if let Some(last_message) = last_message {
                if last_message.elapsed() > idle_timeout {
                    break;
                }
            }

            match self.events.recv_timeout(Duration::from_millis(1)) {
                Ok(event) => {
                    last_message = Some(Instant::now());
                    self.handle_event(event);
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    fn collect_events_until_idle(&mut self, step: Duration, idle_timeout: Duration) {
        let mut last_event = None;

        loop {
            match self.events.recv_timeout(step) {
                Ok(event) => {
                    last_event = Some(Instant::now());
                    self.handle_event(event);
                    if step.is_zero() {
                        continue;
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if step.is_zero() {
                        break;
                    }
                    match last_event {
                        Some(last_event) if last_event.elapsed() < idle_timeout => continue,
                        _ => break,
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    fn handle_event(&mut self, event: corsa_lsp::jsonrpc::InboundEvent) {
        match event {
            corsa_lsp::jsonrpc::InboundEvent::Request { id, .. } => {
                let _ = self.client.respond(id, Value::Null);
            }
            corsa_lsp::jsonrpc::InboundEvent::Notification { method, params } => {
                if method.as_str() != PublishDiagnostics::METHOD {
                    return;
                }

                if let Ok(params) = serde_json::from_value::<PublishDiagnosticsParams>(params) {
                    self.diagnostics
                        .insert(params.uri.as_str().into(), params.diagnostics);
                }
            }
        }
    }

    fn wait_for_uri_diagnostics(&mut self, uri: &str, timeout: Duration) -> bool {
        if self.diagnostics.contains_key(uri) {
            return true;
        }

        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let wait = remaining.min(Duration::from_millis(10));
            match self.events.recv_timeout(wait) {
                Ok(event) => {
                    self.handle_event(event);
                    if self.diagnostics.contains_key(uri) {
                        return true;
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }

        self.diagnostics.contains_key(uri)
    }

    fn apply_document_diagnostic_result(
        &mut self,
        uri: &str,
        result: DocumentDiagnosticReportResult,
    ) -> Option<Vec<Diagnostic>> {
        match result {
            DocumentDiagnosticReportResult::Report(report) => {
                Some(self.apply_document_diagnostic_report(uri, report))
            }
            DocumentDiagnosticReportResult::Partial(partial) => {
                self.store_related_document_reports(partial.related_documents);
                self.diagnostics.get(uri).cloned()
            }
        }
    }

    fn apply_document_diagnostic_report(
        &mut self,
        uri: &str,
        report: DocumentDiagnosticReport,
    ) -> Vec<Diagnostic> {
        match report {
            DocumentDiagnosticReport::Full(full) => {
                self.store_related_document_reports(full.related_documents);
                let result_id = full.full_document_diagnostic_report.result_id;
                let diagnostics = full.full_document_diagnostic_report.items;
                self.set_result_id(uri, result_id);
                self.diagnostics.insert(uri.into(), diagnostics.clone());
                diagnostics
            }
            DocumentDiagnosticReport::Unchanged(unchanged) => {
                self.store_related_document_reports(unchanged.related_documents);
                self.set_result_id(
                    uri,
                    Some(unchanged.unchanged_document_diagnostic_report.result_id),
                );
                self.cached_diagnostics(uri)
            }
        }
    }

    fn store_related_document_reports<I>(&mut self, related_documents: Option<I>)
    where
        I: IntoIterator<Item = (Uri, DocumentDiagnosticReportKind)>,
    {
        let Some(related_documents) = related_documents else {
            return;
        };

        for (uri, report) in related_documents {
            let key = uri.as_str();
            match report {
                DocumentDiagnosticReportKind::Full(full) => {
                    self.set_result_id(key, full.result_id.clone());
                    self.diagnostics.insert(key.into(), full.items);
                }
                DocumentDiagnosticReportKind::Unchanged(unchanged) => {
                    self.set_result_id(key, Some(unchanged.result_id));
                }
            }
        }
    }

    fn set_result_id<S>(&mut self, uri: &str, result_id: Option<S>)
    where
        S: Into<String>,
    {
        if let Some(result_id) = result_id {
            self.diagnostic_result_ids
                .insert(uri.into(), result_id.into());
        } else {
            self.diagnostic_result_ids.remove(uri);
        }
    }

    fn cached_diagnostics(&self, uri: &str) -> Vec<Diagnostic> {
        self.diagnostics.get(uri).cloned().unwrap_or_default()
    }

    fn clear_document_state(&mut self, uri: &str) {
        self.diagnostics.remove(uri);
        self.diagnostic_result_ids.remove(uri);
    }

    /// Find tsgo in local node_modules.
    fn find_tsgo_in_local_node_modules(working_dir: Option<&str>) -> Option<String> {
        let base_dir = working_dir
            .map(PathBuf::from)
            .or_else(|| std::env::current_dir().ok())?;

        let platform_suffix = if cfg!(target_os = "macos") {
            if cfg!(target_arch = "aarch64") {
                "darwin-arm64"
            } else {
                "darwin-x64"
            }
        } else if cfg!(target_os = "linux") {
            if cfg!(target_arch = "aarch64") {
                "linux-arm64"
            } else {
                "linux-x64"
            }
        } else if cfg!(target_os = "windows") {
            "win32-x64"
        } else {
            ""
        };

        let search_in_dir = |dir: &Path| -> Option<String> {
            let pnpm_pattern = dir.join("node_modules/.pnpm");
            if pnpm_pattern.exists() {
                if let Ok(entries) = std::fs::read_dir(&pnpm_pattern) {
                    for entry in entries.flatten() {
                        let name = entry.file_name();
                        let name_str = name.to_string_lossy();
                        if name_str.starts_with("@typescript+native-preview-")
                            && name_str.contains(platform_suffix)
                        {
                            let native_path = entry.path().join(&*cstr!(
                                "node_modules/@typescript/native-preview-{}/lib/tsgo",
                                platform_suffix
                            ));
                            if native_path.exists() {
                                return Some(native_path.to_string_lossy().into());
                            }
                        }
                    }
                }
            }

            let native_candidates = [
                dir.join(&*cstr!(
                    "node_modules/@typescript/native-preview-{}/lib/tsgo",
                    platform_suffix
                )),
                dir.join("node_modules/@typescript/native-preview/lib/tsgo"),
            ];
            for candidate in &native_candidates {
                if candidate.exists() {
                    return Some(candidate.to_string_lossy().into());
                }
            }

            let fallback_candidates = [
                dir.join("node_modules/.bin/tsgo"),
                dir.join("node_modules/@typescript/native-preview/bin/tsgo"),
            ];
            for candidate in &fallback_candidates {
                if candidate.exists() {
                    return Some(candidate.to_string_lossy().into());
                }
            }

            None
        };

        if let Some(path) = search_in_dir(&base_dir) {
            return Some(path);
        }

        let mut current = base_dir.as_path();
        while let Some(parent) = current.parent() {
            if let Some(path) = search_in_dir(parent) {
                return Some(path);
            }
            current = parent;
        }

        None
    }

    /// Find tsgo in common npm global install locations.
    fn find_tsgo_in_common_locations() -> Option<String> {
        let home = std::env::var("HOME").ok()?;
        let candidates: [String; 10] = [
            cstr!("{home}/.npm-global/bin/tsgo"),
            cstr!("{home}/.npm/bin/tsgo"),
            cstr!("{home}/.local/share/pnpm/tsgo"),
            cstr!("{home}/.volta/bin/tsgo"),
            cstr!("{home}/.local/share/mise/shims/tsgo"),
            cstr!("{home}/.asdf/shims/tsgo"),
            cstr!("{home}/.local/share/fnm/node-versions/current/bin/tsgo"),
            cstr!("{home}/.nvm/versions/node/current/bin/tsgo"),
            "/opt/homebrew/bin/tsgo".into(),
            "/usr/local/bin/tsgo".into(),
        ];

        for path in candidates {
            if Path::new(path.as_str()).exists() {
                return Some(path);
            }
        }

        if let Ok(output) = std::process::Command::new("npm")
            .args(["root", "-g"])
            .output()
        {
            if output.status.success() {
                #[allow(clippy::disallowed_types)]
                let npm_root = std::string::String::from_utf8_lossy(&output.stdout);
                let npm_root = npm_root.trim();
                if let Some(lib_parent) = Path::new(npm_root).parent() {
                    let tsgo_path = lib_parent.join("bin/tsgo");
                    if tsgo_path.exists() {
                        return Some(tsgo_path.to_string_lossy().into());
                    }
                }
            }
        }

        None
    }
}

impl Drop for TsgoLspClient {
    fn drop(&mut self) {
        let _ = self.shutdown();
        if let Some(ref dir) = self.temp_dir {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

fn parse_uri(uri: &str) -> Result<Uri, String> {
    Uri::from_str(uri).map_err(|e| cstr!("Invalid URI `{uri}`: {e}"))
}

fn text_document_position(
    uri: &str,
    line: u32,
    character: u32,
) -> Result<TextDocumentPositionParams, String> {
    Ok(TextDocumentPositionParams::new(
        TextDocumentIdentifier::new(parse_uri(uri)?),
        Position::new(line, character),
    ))
}

fn json_from_value<T>(value: Value) -> Result<T, String>
where
    T: DeserializeOwned,
{
    serde_json::from_value(value).map_err(|e| cstr!("Failed to decode JSON params: {e}"))
}

fn value_to_json<T>(value: T) -> Result<Value, String>
where
    T: Serialize,
{
    serde_json::to_value(value).map_err(|e| cstr!("Failed to encode JSON response: {e}"))
}

fn convert_diagnostics(diagnostics: &[Diagnostic]) -> Vec<LspDiagnostic> {
    diagnostics
        .iter()
        .filter_map(|diagnostic| {
            serde_json::to_value(diagnostic)
                .ok()
                .and_then(|value| serde_json::from_value(value).ok())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{convert_diagnostics, resolve_temp_dir_base};
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
                "tsgo-temp-dir-{name}-{}-{case_id}",
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
                .join("vize-tsgo")
        );

        let _ = fs::remove_dir_all(&case_dir);
    }

    #[test]
    fn falls_back_to_nearest_available_node_modules_root() {
        let case_dir = unique_case_dir("fallback");
        let source_dir = case_dir.join("playground").join("src").join("shared");

        let _ = fs::remove_dir_all(&case_dir);
        fs::create_dir_all(&source_dir).unwrap();

        let resolved = resolve_temp_dir_base(Some(&source_dir));
        let expected_root = source_dir
            .ancestors()
            .find(|path| path.join("node_modules").join("vue").is_dir())
            .unwrap()
            .to_path_buf();

        assert_eq!(
            resolved,
            expected_root.join("__agent_only").join("vize-tsgo")
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
}
