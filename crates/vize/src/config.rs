//! Configuration file loading for vize.
//!
//! Reads `vize.config.pkl` (preferred) or `vize.config.json` from the current
//! working directory. Also provides JSON Schema generation for editor autocompletion.

#![allow(clippy::disallowed_types)]

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Top-level vize configuration.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct VizeConfig {
    /// JSON Schema reference (for editor autocompletion).
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    /// Type checking configuration.
    #[serde(default)]
    pub check: CheckConfig,

    /// Formatting configuration.
    #[cfg(feature = "glyph")]
    #[serde(default)]
    pub fmt: vize_glyph::FormatOptions,
}

/// Configuration for the `check` command.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CheckConfig {
    /// Path to a `.d.ts` file that augments `ComponentCustomProperties`.
    ///
    /// The file should follow Vue's standard module augmentation pattern:
    /// ```ts
    /// declare module 'vue' {
    ///   interface ComponentCustomProperties {
    ///     $t: (...args: any[]) => string
    ///   }
    /// }
    /// ```
    ///
    /// Resolved relative to `vize.config.json`.
    /// When omitted or null, no plugin globals are declared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub globals: Option<String>,
}

/// Load configuration from `vize.config.pkl` (preferred) or `vize.config.json`.
///
/// PKL takes priority when both files exist. If the PKL file exists but parsing
/// fails (e.g. `pkl` binary not on PATH), falls back to defaults with a warning.
pub fn load_config(dir: Option<&Path>) -> VizeConfig {
    let base = dir
        .map(|d| d.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    // Try PKL first
    let pkl_path = base.join("vize.config.pkl");
    if pkl_path.exists() {
        match rpkl::from_config::<VizeConfig>(&pkl_path) {
            Ok(config) => return config,
            Err(e) => {
                eprintln!(
                    "\x1b[33mWarning:\x1b[0m Failed to parse {}: {}",
                    pkl_path.display(),
                    e
                );
                return VizeConfig::default();
            }
        }
    }

    // Fall back to JSON
    let json_path = base.join("vize.config.json");
    if !json_path.exists() {
        return VizeConfig::default();
    }

    match std::fs::read_to_string(&json_path) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(config) => config,
            Err(e) => {
                eprintln!(
                    "\x1b[33mWarning:\x1b[0m Failed to parse {}: {}",
                    json_path.display(),
                    e
                );
                VizeConfig::default()
            }
        },
        Err(e) => {
            eprintln!(
                "\x1b[33mWarning:\x1b[0m Failed to read {}: {}",
                json_path.display(),
                e
            );
            VizeConfig::default()
        }
    }
}

/// JSON Schema for `vize.config.json`.
///
/// Generated from Pkl schema definitions in `npm/vize/pkl/`.
pub const VIZE_CONFIG_SCHEMA: &str =
    include_str!("../../../npm/vize/schemas/vize.config.schema.json");

#[cfg(all(test, feature = "glyph"))]
mod tests {
    use super::load_config;

    #[test]
    fn load_config_returns_defaults_when_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let config = load_config(Some(dir.path()));
        assert_eq!(config.fmt.print_width, 100);
        assert_eq!(config.fmt.tab_width, 2);
        assert!(!config.fmt.use_tabs);
        assert!(config.fmt.semi);
        assert!(!config.fmt.single_quote);
        assert!(config.fmt.sort_attributes);
        assert!(config.fmt.normalize_directive_shorthands);
    }

    #[test]
    fn load_config_parses_fmt_section() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("vize.config.json");
        std::fs::write(
            &config_path,
            r#"{
                "fmt": {
                    "printWidth": 80,
                    "tabWidth": 4,
                    "useTabs": true,
                    "semi": false,
                    "singleQuote": true,
                    "sortAttributes": false,
                    "normalizeDirectiveShorthands": false
                }
            }"#,
        )
        .unwrap();

        let config = load_config(Some(dir.path()));
        assert_eq!(config.fmt.print_width, 80);
        assert_eq!(config.fmt.tab_width, 4);
        assert!(config.fmt.use_tabs);
        assert!(!config.fmt.semi);
        assert!(config.fmt.single_quote);
        assert!(!config.fmt.sort_attributes);
        assert!(!config.fmt.normalize_directive_shorthands);
    }

    #[test]
    fn load_config_partial_fmt_uses_defaults_for_missing() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("vize.config.json");
        std::fs::write(&config_path, r#"{ "fmt": { "printWidth": 120 } }"#).unwrap();

        let config = load_config(Some(dir.path()));
        assert_eq!(config.fmt.print_width, 120);
        // defaults preserved
        assert_eq!(config.fmt.tab_width, 2);
        assert!(!config.fmt.use_tabs);
        assert!(config.fmt.semi);
    }

    #[test]
    fn load_config_returns_defaults_on_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("vize.config.json");
        std::fs::write(&config_path, "not valid json {{{").unwrap();

        let config = load_config(Some(dir.path()));
        // should fall back to defaults
        assert_eq!(config.fmt.print_width, 100);
        assert_eq!(config.fmt.tab_width, 2);
    }

    #[test]
    fn load_config_with_check_and_fmt() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("vize.config.json");
        std::fs::write(
            &config_path,
            r#"{
                "check": { "globals": "globals.d.ts" },
                "fmt": { "singleQuote": true, "maxAttributesPerLine": 3 }
            }"#,
        )
        .unwrap();

        let config = load_config(Some(dir.path()));
        // check section
        let globals = config.check.globals.unwrap();
        assert_eq!(globals, "globals.d.ts");
        // fmt section
        assert!(config.fmt.single_quote);
        assert_eq!(config.fmt.max_attributes_per_line, Some(3));
    }

    #[test]
    #[ignore = "requires pkl runtime installed"]
    fn load_config_parses_pkl() {
        let dir = tempfile::tempdir().unwrap();
        let pkl_path = dir.path().join("vize.config.pkl");
        std::fs::write(&pkl_path, "check {\n    globals = \"globals.d.ts\"\n}\n").unwrap();

        let config = load_config(Some(dir.path()));
        assert_eq!(config.check.globals.as_deref(), Some("globals.d.ts"));
    }
}

/// Write the JSON Schema to `node_modules/.vize/vize.config.schema.json`.
pub fn write_schema(dir: Option<&Path>) {
    let base = dir
        .map(|d| d.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let schema_dir = base.join("node_modules/.vize");
    if std::fs::create_dir_all(&schema_dir).is_ok() {
        let schema_path = schema_dir.join("vize.config.schema.json");
        let _ = std::fs::write(&schema_path, VIZE_CONFIG_SCHEMA);
    }
}
