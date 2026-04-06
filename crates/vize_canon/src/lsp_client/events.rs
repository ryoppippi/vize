use super::CorsaLspClient;
use lsp_types::{
    notification::{Notification, PublishDiagnostics},
    Diagnostic, DocumentDiagnosticReport, DocumentDiagnosticReportKind,
    DocumentDiagnosticReportResult, PublishDiagnosticsParams, Uri,
};
use serde_json::Value;
use std::time::{Duration, Instant};
use vize_carton::String;

impl CorsaLspClient {
    pub(crate) fn diagnostics_cache_len(&self) -> usize {
        self.diagnostics.len()
    }

    pub(crate) fn clear_diagnostics_cache(&mut self) {
        self.diagnostics.clear();
        self.diagnostic_result_ids.clear();
    }

    pub(super) fn drain_pending_messages(&mut self) {
        self.collect_events_until_idle(Duration::ZERO, Duration::ZERO);
    }

    pub(super) fn read_notifications(&mut self) -> Result<(), String> {
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
        if self.supports_project_diagnostics_api() || self.supports_file_diagnostics_api() {
            return;
        }

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

    pub(super) fn collect_events_until_idle(&mut self, step: Duration, idle_timeout: Duration) {
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

    pub(super) fn wait_for_uri_diagnostics(&mut self, uri: &str, timeout: Duration) -> bool {
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

    pub(super) fn apply_document_diagnostic_result(
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

    pub(super) fn cached_diagnostics(&self, uri: &str) -> Vec<Diagnostic> {
        self.diagnostics.get(uri).cloned().unwrap_or_default()
    }

    pub(super) fn clear_document_state(&mut self, uri: &str) {
        self.diagnostics.remove(uri);
        self.diagnostic_result_ids.remove(uri);
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
}
