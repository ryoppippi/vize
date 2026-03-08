//! Shared config model.

mod formatter;
mod global_types;
mod lsp;
mod type_checker;

use serde::{Deserialize, Serialize};

use crate::String;

pub use formatter::{
    ArrowParens, AttributeSortOrder, EndOfLine, FormatterConfig, QuoteProps, TrailingComma,
};
pub use global_types::{GlobalTypeDeclaration, GlobalTypesConfig, RawGlobalTypesConfig};
pub use lsp::LspConfig;
pub use type_checker::TypeCheckerConfig;

/// Effective shared configuration.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(default)]
pub struct VizeConfig {
    /// JSON Schema reference for legacy JSON editor autocompletion.
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    /// Formatter settings shared by CLI and IDE formatting.
    #[serde(skip_serializing_if = "FormatterConfig::is_default")]
    pub formatter: FormatterConfig,
    /// Type checker settings shared by CLI and IDE diagnostics.
    #[serde(
        rename = "typeChecker",
        skip_serializing_if = "TypeCheckerConfig::is_default"
    )]
    pub type_checker: TypeCheckerConfig,
    /// IDE/LSP feature flags.
    #[serde(skip_serializing_if = "LspConfig::is_default")]
    pub lsp: LspConfig,
    /// Template global declarations.
    #[serde(
        rename = "globalTypes",
        skip_serializing_if = "GlobalTypesConfig::is_empty"
    )]
    pub global_types: GlobalTypesConfig,
}

/// Raw config representation with legacy aliases preserved for migration.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub(crate) struct RawVizeConfig {
    #[serde(rename = "$schema")]
    pub schema: Option<String>,
    pub formatter: FormatterConfig,
    #[serde(rename = "typeChecker")]
    pub type_checker: TypeCheckerConfig,
    pub lsp: LspConfig,
    #[serde(rename = "globalTypes")]
    pub global_types: RawGlobalTypesConfig,
    #[serde(rename = "check")]
    legacy_check: Option<LegacyCheckConfig>,
    #[serde(rename = "fmt")]
    legacy_formatter: Option<FormatterConfig>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct LegacyCheckConfig {
    globals: Option<String>,
}

impl From<RawVizeConfig> for VizeConfig {
    fn from(raw: RawVizeConfig) -> Self {
        let mut type_checker = raw.type_checker;
        if type_checker.globals_file.is_none() {
            if let Some(legacy_check) = raw.legacy_check {
                type_checker.globals_file = legacy_check.globals;
            }
        }

        let formatter = if raw.formatter == FormatterConfig::default() {
            raw.legacy_formatter.unwrap_or(raw.formatter)
        } else {
            raw.formatter
        };

        Self {
            schema: raw.schema,
            formatter,
            type_checker,
            lsp: raw.lsp,
            global_types: raw.global_types.into(),
        }
    }
}
