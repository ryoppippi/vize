//! LSP feature flags.

use serde::{Deserialize, Serialize};

/// IDE/LSP settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LspConfig {
    pub enabled: bool,
    pub diagnostics: bool,
    pub completion: bool,
    pub hover: bool,
    pub definition: bool,
    pub formatting: bool,
    #[serde(rename = "codeActions")]
    pub code_actions: bool,
    pub tsgo: bool,
}

impl LspConfig {
    /// Returns true when the config matches the built-in defaults.
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

impl Default for LspConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            diagnostics: true,
            completion: true,
            hover: true,
            definition: true,
            formatting: true,
            code_actions: true,
            tsgo: true,
        }
    }
}
