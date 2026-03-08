//! Config loading helpers.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{String, ToCompactString};

use super::model::{RawVizeConfig, VizeConfig};

const CONFIG_FILE_NAMES: [&str; 2] = ["vize.config.pkl", "vize.config.json"];

/// Loaded config and its source path.
#[derive(Debug, Clone)]
pub struct LoadedConfig {
    /// Effective configuration with defaults applied.
    pub config: VizeConfig,
    /// Path of the config file that was used, if any.
    pub source_path: Option<PathBuf>,
}

/// Load configuration from a directory or file path.
pub fn load_config(path: Option<&Path>) -> VizeConfig {
    load_config_with_source(path).config
}

/// Load configuration from a directory or file path and return its source path.
pub fn load_config_with_source(path: Option<&Path>) -> LoadedConfig {
    let base = path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    if let Some(file_path) = resolve_file_path(&base) {
        return LoadedConfig {
            config: parse_config_file(&file_path).unwrap_or_default(),
            source_path: Some(file_path),
        };
    }

    let Some(dir_path) = resolve_dir_path(&base) else {
        return LoadedConfig {
            config: VizeConfig::default(),
            source_path: None,
        };
    };

    for file_name in CONFIG_FILE_NAMES {
        let candidate = dir_path.join(file_name);
        if !candidate.exists() {
            continue;
        }

        if let Some(config) = try_parse_candidate(&candidate) {
            return LoadedConfig {
                config,
                source_path: Some(candidate),
            };
        }
    }

    LoadedConfig {
        config: VizeConfig::default(),
        source_path: None,
    }
}

fn resolve_file_path(base: &Path) -> Option<PathBuf> {
    if base.is_file() {
        Some(base.to_path_buf())
    } else {
        None
    }
}

fn resolve_dir_path(base: &Path) -> Option<PathBuf> {
    if base.is_dir() {
        return Some(base.to_path_buf());
    }

    if base.extension().is_none() {
        return Some(base.to_path_buf());
    }

    None
}

fn try_parse_candidate(path: &Path) -> Option<VizeConfig> {
    match parse_config_file(path) {
        Ok(config) => Some(config),
        Err(error) => {
            eprintln!(
                "\x1b[33mWarning:\x1b[0m Failed to parse {}: {}",
                path.display(),
                error
            );
            None
        }
    }
}

fn parse_config_file(path: &Path) -> Result<VizeConfig, Box<dyn std::error::Error>> {
    let config = match path.extension().and_then(|ext| ext.to_str()) {
        Some("pkl") => parse_pkl_config(path)?,
        Some("json") => {
            let content = std::fs::read_to_string(path)?;
            serde_json::from_str::<RawVizeConfig>(&content)?.into()
        }
        _ => return Ok(VizeConfig::default()),
    };

    Ok(config)
}

fn parse_pkl_config(path: &Path) -> Result<VizeConfig, Box<dyn std::error::Error>> {
    if let Some(json) = evaluate_pkl_with_cli(path)? {
        return Ok(serde_json::from_str::<RawVizeConfig>(&json)?.into());
    }

    Ok(rpkl::from_config::<RawVizeConfig>(path)?.into())
}

fn evaluate_pkl_with_cli(path: &Path) -> Result<Option<String>, Box<dyn std::error::Error>> {
    for command in pkl_command_candidates(path) {
        if let Some(output) = run_pkl_command(path, &command)? {
            return Ok(Some(output));
        }
    }

    Ok(None)
}

fn pkl_command_candidates(path: &Path) -> Vec<PklCommand> {
    let mut commands = Vec::new();

    for ancestor in path.ancestors() {
        for binary in local_pkl_candidates(ancestor) {
            if binary.exists() {
                commands.push(PklCommand::Binary(binary));
            }
        }
    }

    commands.push(PklCommand::Binary(PathBuf::from("pkl")));
    commands.push(PklCommand::Npx);
    commands
}

fn local_pkl_candidates(base: &Path) -> [PathBuf; 4] {
    [
        base.join("node_modules/.bin/pkl"),
        base.join("node_modules/.bin/pkl.cmd"),
        base.join("node_modules/@pkl-community/pkl/pkl"),
        base.join("node_modules/@pkl-community/pkl/pkl.exe"),
    ]
}

fn run_pkl_command(
    path: &Path,
    command: &PklCommand,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let output = match command {
        PklCommand::Binary(binary) => Command::new(binary)
            .args(["eval", "-f", "json"])
            .arg(path)
            .output(),
        PklCommand::Npx => Command::new("npx")
            .args(["--yes", "@pkl-community/pkl", "eval", "-f", "json"])
            .arg(path)
            .output(),
    };

    let Ok(output) = output else {
        return Ok(None);
    };

    if !output.status.success() {
        return Ok(None);
    }

    Ok(Some(
        std::str::from_utf8(&output.stdout)?.to_compact_string(),
    ))
}

enum PklCommand {
    Binary(PathBuf),
    Npx,
}

#[cfg(test)]
mod tests {
    use super::load_config_with_source;

    #[test]
    fn load_config_uses_explicit_file_path() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("custom.json");
        std::fs::write(&config_path, r#"{ "formatter": { "singleQuote": true } }"#).unwrap();

        let loaded = load_config_with_source(Some(&config_path));
        assert_eq!(loaded.source_path.as_deref(), Some(config_path.as_path()));
        assert!(loaded.config.formatter.single_quote);
    }
}
