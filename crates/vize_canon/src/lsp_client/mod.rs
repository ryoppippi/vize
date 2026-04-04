//! LSP Client for tsgo.
//!
//! This module preserves the legacy `TsgoLspClient` surface used across the
//! workspace while delegating process management and virtual document syncing to
//! `corsa-bind`'s `corsa_lsp`.

use corsa_lsp::{LspClient, LspOverlay};
use corsa_runtime::BroadcastReceiver;
use lsp_types::Diagnostic;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use vize_carton::{FxHashMap, String};

mod diagnostics;
mod events;
mod lifecycle;
mod paths;
mod queries;
mod utils;

#[cfg(test)]
mod tests;

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
