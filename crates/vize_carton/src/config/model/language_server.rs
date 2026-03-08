//! Language server feature flags.

use serde::{Deserialize, Serialize};

/// IDE language server settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LanguageServerConfig {
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

impl LanguageServerConfig {
    /// Returns true when the config matches the built-in defaults.
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

impl Default for LanguageServerConfig {
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

pub type LspConfig = LanguageServerConfig;
