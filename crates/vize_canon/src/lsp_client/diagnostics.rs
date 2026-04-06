use super::{
    diagnostics_api::{flatten_file_diagnostics, map_project_diagnostics},
    session::uri_document_identifier,
    utils::{convert_diagnostics, parse_uri},
    CorsaLspClient, DiagnosticFetch, LspDiagnostic,
};
use corsa_runtime::{block_on, spawn};
use lsp_types::{
    request::DocumentDiagnosticRequest, DocumentDiagnosticParams, DocumentDiagnosticReport,
    DocumentDiagnosticReportResult, PartialResultParams, TextDocumentIdentifier,
    WorkDoneProgressParams,
};
use std::time::Duration;
use vize_carton::{cstr, String};

impl CorsaLspClient {
    /// Get cached diagnostics for a URI.
    pub fn get_diagnostics(&self, uri: &str) -> Vec<LspDiagnostic> {
        self.diagnostics
            .get(uri)
            .map(|diagnostics| convert_diagnostics(diagnostics))
            .unwrap_or_default()
    }

    /// Request diagnostics for a single document.
    pub fn request_diagnostics(&mut self, uri: &str) -> Result<Vec<LspDiagnostic>, String> {
        self.request_diagnostics_full(uri)
            .map(|fetch| convert_diagnostics(&fetch.diagnostics))
    }

    /// Request diagnostics for multiple URIs in batch.
    pub fn request_diagnostics_batch(
        &mut self,
        uris: &[String],
    ) -> Vec<(String, Vec<LspDiagnostic>)> {
        if self.supports_project_diagnostics_api()
            && uris
                .iter()
                .all(|uri| self.can_use_api_for_uri(uri.as_str()))
        {
            if let Some(results) = self.request_diagnostics_batch_via_api(uris) {
                return results;
            }
        }

        self.request_diagnostics_batch_via_lsp(uris)
    }

    pub(crate) fn request_diagnostics_full(
        &mut self,
        uri: &str,
    ) -> Result<DiagnosticFetch, String> {
        if self.supports_file_diagnostics_api() && self.can_use_api_for_uri(uri) {
            if let Some(fetch) = self.request_diagnostics_full_via_api(uri) {
                return fetch;
            }
        }

        self.request_diagnostics_full_via_lsp(uri)
    }

    fn request_diagnostics_batch_via_api(
        &mut self,
        uris: &[String],
    ) -> Option<Vec<(String, Vec<LspDiagnostic>)>> {
        let report = block_on(self.session.get_diagnostics_for_project()).ok()?;
        Some(map_project_diagnostics(self, &report, uris))
    }

    fn request_diagnostics_batch_via_lsp(
        &mut self,
        uris: &[String],
    ) -> Vec<(String, Vec<LspDiagnostic>)> {
        const MAX_INFLIGHT_DIAGNOSTIC_REQUESTS: usize = 64;
        let mut results = Vec::with_capacity(uris.len());

        for uri_chunk in uris.chunks(MAX_INFLIGHT_DIAGNOSTIC_REQUESTS) {
            let mut handles = Vec::with_capacity(uri_chunk.len());

            for uri in uri_chunk {
                let uri_string = uri.clone();
                let previous_result_id = self
                    .diagnostic_result_ids
                    .get(uri.as_str())
                    .cloned()
                    .map(|result_id| result_id.as_str().into());
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
                        eprintln!("\x1b[90m[corsa] batch request skipped: {err}\x1b[0m")
                    }
                    Err(_) => {}
                }
            }

            self.collect_events_until_idle(Duration::from_millis(5), Duration::from_millis(30));

            for (uri, response) in joined {
                let diagnostics = match response {
                    Ok(report) => self
                        .apply_document_diagnostic_result(uri.as_str(), report)
                        .unwrap_or_else(|| self.cached_diagnostics(uri.as_str())),
                    Err(_) => self.cached_diagnostics(uri.as_str()),
                };
                results.push((uri, convert_diagnostics(&diagnostics)));
            }
        }

        results
    }

    fn request_diagnostics_full_via_api(
        &mut self,
        uri: &str,
    ) -> Option<Result<DiagnosticFetch, String>> {
        let response = block_on(
            self.session
                .get_diagnostics_for_file(uri_document_identifier(uri)),
        )
        .ok()?;
        let diagnostics = flatten_file_diagnostics(&response);
        self.diagnostics.insert(uri.into(), diagnostics.clone());
        Some(Ok(DiagnosticFetch {
            diagnostics,
            used_cache: false,
        }))
    }

    fn request_diagnostics_full_via_lsp(&mut self, uri: &str) -> Result<DiagnosticFetch, String> {
        let uri = parse_uri(uri)?;
        let previous_result_id = self
            .diagnostic_result_ids
            .get(uri.as_str())
            .cloned()
            .map(|result_id| result_id.as_str().into());
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
}
