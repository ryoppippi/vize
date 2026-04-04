use corsa::{
    api::{
        ApiClient, ApiMode, ApiSpawnConfig, FileChangeSummary, FileChanges, ManagedSnapshot,
        ProjectHandle, TypeResponse, UpdateSnapshotParams,
    },
    runtime::block_on,
};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use vize_carton::{CompactString, String, ToCompactString};

const SESSION_DIR: &str = "vize-patina";
const VIRTUAL_FILE_NAME: &str = "active.patina.ts";
const TSCONFIG_FILE_NAME: &str = "tsconfig.json";
static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);
const TSCONFIG_CONTENTS: &str = r#"{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "strict": true,
    "noEmit": true,
    "skipLibCheck": true
  },
  "files": ["active.patina.ts"]
}
"#;

pub(crate) struct CorsaTypeAwareSession {
    client: ApiClient,
    project_root: PathBuf,
    session_root: PathBuf,
    virtual_file_path: PathBuf,
    config_path_wire: String,
    virtual_file_wire: String,
    initialized_snapshot: bool,
    closed: bool,
}

pub(super) struct ActiveProject {
    snapshot: ManagedSnapshot,
    project: ProjectHandle,
}

#[derive(Debug, Default)]
pub(super) struct TypeProbe {
    pub type_texts: Vec<CompactString>,
    pub property_names: Vec<CompactString>,
    pub property_types: Vec<Vec<CompactString>>,
    pub call_signatures: Vec<Vec<Vec<CompactString>>>,
    pub return_types: Vec<Vec<CompactString>>,
}

impl CorsaTypeAwareSession {
    pub(super) fn new(filename: &str) -> Result<Self, String> {
        let project_root = resolve_project_root(filename);
        let session_root = allocate_session_root(&project_root);
        std::fs::create_dir_all(&session_root).map_err(|error| {
            io_error_message(
                "Failed to create patina session directory",
                &session_root,
                &error,
            )
        })?;

        let config_path = session_root.join(TSCONFIG_FILE_NAME);
        std::fs::write(&config_path, TSCONFIG_CONTENTS).map_err(|error| {
            io_error_message("Failed to write patina tsconfig", &config_path, &error)
        })?;

        let virtual_file_path = session_root.join(VIRTUAL_FILE_NAME);
        let executable = resolve_tsgo_executable(&project_root);
        let client = block_on(ApiClient::spawn(
            ApiSpawnConfig::new(executable)
                .with_mode(ApiMode::SyncMsgpackStdio)
                .with_cwd(&session_root),
        ))
        .map_err(|error| {
            compact_error(
                "Failed to start corsa type-aware session",
                error.to_compact_string().as_str(),
            )
        })?;

        Ok(Self {
            client,
            project_root,
            session_root,
            config_path_wire: path_to_wire(&config_path),
            virtual_file_wire: path_to_wire(&virtual_file_path),
            virtual_file_path,
            initialized_snapshot: false,
            closed: false,
        })
    }

    pub(super) fn matches_source_file(&self, filename: &str) -> bool {
        self.project_root == resolve_project_root(filename)
    }

    pub(super) fn open_virtual_project(
        &mut self,
        generated_source: &str,
    ) -> Result<ActiveProject, String> {
        std::fs::write(&self.virtual_file_path, generated_source).map_err(|error| {
            io_error_message(
                "Failed to write patina virtual TypeScript",
                &self.virtual_file_path,
                &error,
            )
        })?;

        let file_changes = if self.initialized_snapshot {
            Some(FileChanges::Summary(FileChangeSummary {
                changed: vec![self.virtual_file_wire.as_str().into()],
                created: Vec::new(),
                deleted: Vec::new(),
            }))
        } else {
            self.initialized_snapshot = true;
            None
        };

        let snapshot = block_on(self.client.update_snapshot(UpdateSnapshotParams {
            open_project: Some(self.config_path_wire.as_str().into()),
            file_changes,
        }))
        .map_err(|error| {
            compact_error(
                "Failed to update patina type snapshot",
                error.to_compact_string().as_str(),
            )
        })?;

        let default_project =
            block_on(snapshot.get_default_project_for_file(self.virtual_file_wire.as_str()))
                .map_err(|error| {
                    compact_error(
                        "Failed to resolve patina project",
                        error.to_compact_string().as_str(),
                    )
                })?;

        let project = default_project
            .or_else(|| snapshot.projects.first().cloned())
            .ok_or_else(|| "Failed to resolve patina project".to_compact_string())?;

        Ok(ActiveProject {
            snapshot,
            project: project.id,
        })
    }

    pub(super) fn probe_type_at_offset(
        &self,
        active_project: &ActiveProject,
        generated_source: &str,
        generated_offset: u32,
        load_property_types: bool,
        load_signatures: bool,
    ) -> Result<Option<TypeProbe>, String> {
        let utf16_offset = byte_offset_to_utf16_offset(generated_source, generated_offset);
        let symbol = block_on(self.client.get_symbol_at_position(
            active_project.snapshot.handle.clone(),
            active_project.project.clone(),
            self.virtual_file_wire.as_str(),
            utf16_offset,
        ))
        .map_err(|error| {
            compact_error(
                "Failed to query checker symbol",
                error.to_compact_string().as_str(),
            )
        })?;
        let type_response = if let Some(symbol) = symbol {
            match block_on(self.client.get_type_of_symbol(
                active_project.snapshot.handle.clone(),
                active_project.project.clone(),
                symbol.id,
            ))
            .map_err(|error| {
                compact_error(
                    "Failed to query checker symbol type",
                    error.to_compact_string().as_str(),
                )
            })? {
                Some(type_response) => Some(type_response),
                None => block_on(self.client.get_type_at_position(
                    active_project.snapshot.handle.clone(),
                    active_project.project.clone(),
                    self.virtual_file_wire.as_str(),
                    utf16_offset,
                ))
                .map_err(|error| {
                    compact_error(
                        "Failed to query checker type",
                        error.to_compact_string().as_str(),
                    )
                })?,
            }
        } else {
            block_on(self.client.get_type_at_position(
                active_project.snapshot.handle.clone(),
                active_project.project.clone(),
                self.virtual_file_wire.as_str(),
                utf16_offset,
            ))
            .map_err(|error| {
                compact_error(
                    "Failed to query checker type",
                    error.to_compact_string().as_str(),
                )
            })?
        };
        let Some(type_response) = type_response else {
            return Ok(None);
        };

        let mut probe = TypeProbe {
            type_texts: self.collect_type_texts(active_project, &type_response)?,
            property_names: Vec::new(),
            property_types: Vec::new(),
            call_signatures: Vec::new(),
            return_types: Vec::new(),
        };

        let properties = block_on(self.client.get_properties_of_type(
            active_project.snapshot.handle.clone(),
            active_project.project.clone(),
            type_response.id.clone(),
        ))
        .map_err(|error| {
            compact_error(
                "Failed to query checker properties",
                error.to_compact_string().as_str(),
            )
        })?;

        probe.property_names.reserve(properties.len());
        for property in &properties {
            probe
                .property_names
                .push(property.name.as_str().to_compact_string());
        }

        if load_property_types && !properties.is_empty() {
            let property_types = block_on(
                self.client.get_types_of_symbols(
                    active_project.snapshot.handle.clone(),
                    active_project.project.clone(),
                    properties
                        .iter()
                        .map(|property| property.id.clone())
                        .collect(),
                ),
            )
            .map_err(|error| {
                compact_error(
                    "Failed to query checker property types",
                    error.to_compact_string().as_str(),
                )
            })?;

            probe.property_types.reserve(property_types.len());
            for property_type in property_types {
                if let Some(property_type) = property_type {
                    probe
                        .property_types
                        .push(self.collect_type_texts(active_project, &property_type)?);
                } else {
                    probe.property_types.push(Vec::new());
                }
            }
        }

        if load_signatures {
            let signatures = block_on(self.client.get_signatures_of_type(
                active_project.snapshot.handle.clone(),
                active_project.project.clone(),
                type_response.id,
                0,
            ))
            .map_err(|error| {
                compact_error(
                    "Failed to query checker signatures",
                    error.to_compact_string().as_str(),
                )
            })?;

            probe.call_signatures.reserve(signatures.len());
            probe.return_types.reserve(signatures.len());
            for signature in signatures {
                if signature.parameters.is_empty() {
                    probe.call_signatures.push(Vec::new());
                } else {
                    let parameter_types = block_on(self.client.get_types_of_symbols(
                        active_project.snapshot.handle.clone(),
                        active_project.project.clone(),
                        signature.parameters.clone(),
                    ))
                    .map_err(|error| {
                        compact_error(
                            "Failed to query checker signature parameter types",
                            error.to_compact_string().as_str(),
                        )
                    })?;

                    let mut rendered_parameters = Vec::with_capacity(parameter_types.len());
                    for parameter_type in parameter_types {
                        if let Some(parameter_type) = parameter_type {
                            rendered_parameters
                                .push(self.collect_type_texts(active_project, &parameter_type)?);
                        } else {
                            rendered_parameters.push(Vec::new());
                        }
                    }
                    probe.call_signatures.push(rendered_parameters);
                }

                let return_type = block_on(self.client.get_return_type_of_signature(
                    active_project.snapshot.handle.clone(),
                    active_project.project.clone(),
                    signature.id,
                ))
                .map_err(|error| {
                    compact_error(
                        "Failed to query checker signature return type",
                        error.to_compact_string().as_str(),
                    )
                })?;

                if let Some(return_type) = return_type {
                    probe
                        .return_types
                        .push(self.collect_type_texts(active_project, &return_type)?);
                } else {
                    probe.return_types.push(Vec::new());
                }
            }
        }

        Ok(Some(probe))
    }

    pub(super) fn close(&mut self) {
        if self.closed {
            return;
        }
        self.closed = true;
        let _ = block_on(self.client.close());
        let _ = std::fs::remove_dir_all(&self.session_root);
    }

    fn collect_type_texts(
        &self,
        active_project: &ActiveProject,
        type_response: &TypeResponse,
    ) -> Result<Vec<CompactString>, String> {
        let mut texts = Vec::with_capacity(type_response.texts.len() + 1);
        for text in &type_response.texts {
            push_unique_text(&mut texts, text.as_str());
        }
        if texts.is_empty() {
            let rendered = block_on(self.client.type_to_string(
                active_project.snapshot.handle.clone(),
                active_project.project.clone(),
                type_response.id.clone(),
                None,
                None,
            ))
            .map_err(|error| {
                compact_error(
                    "Failed to render checker type",
                    error.to_compact_string().as_str(),
                )
            })?;
            push_unique_text(&mut texts, rendered.as_str());
        }
        Ok(texts)
    }
}

impl Drop for CorsaTypeAwareSession {
    fn drop(&mut self) {
        self.close();
    }
}

fn path_to_wire(path: &Path) -> String {
    path.to_string_lossy().as_ref().to_compact_string()
}

fn allocate_session_root(project_root: &Path) -> PathBuf {
    let session_name = next_session_directory_name();
    project_root
        .join("__agent_only")
        .join(SESSION_DIR)
        .join(session_name.as_str())
}

fn next_session_directory_name() -> String {
    let counter = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id() as u64;
    let mut name = String::with_capacity(32);
    name.push_str("session-");
    push_u64(&mut name, pid);
    name.push('-');
    push_u64(&mut name, counter);
    name
}

fn resolve_project_root(filename: &str) -> PathBuf {
    let start_dir = source_directory(filename);
    let mut current = start_dir.as_path();
    let mut package_root = None;

    loop {
        if current.join("node_modules").join("vue").is_dir() {
            return current.to_path_buf();
        }
        if package_root.is_none() && current.join("package.json").is_file() {
            package_root = Some(current.to_path_buf());
        }
        let Some(parent) = current.parent() else {
            break;
        };
        current = parent;
    }

    package_root.unwrap_or(start_dir)
}

fn source_directory(filename: &str) -> PathBuf {
    let path = Path::new(filename);
    if path.is_absolute() {
        return path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| path.to_path_buf());
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let joined = cwd.join(path);
    joined.parent().map(Path::to_path_buf).unwrap_or(cwd)
}

fn resolve_tsgo_executable(project_root: &Path) -> PathBuf {
    if let Some(tsgo_path) = std::env::var_os("TSGO_PATH") {
        let path = PathBuf::from(tsgo_path);
        if path.exists() {
            return path;
        }
    }
    if let Some(tsgo_path) = std::env::var_os("TSGO_EXECUTABLE") {
        let path = PathBuf::from(tsgo_path);
        if path.exists() {
            return path;
        }
    }

    for current in project_root.ancestors() {
        for candidate in real_tsgo_candidates(current) {
            if candidate.exists() {
                return candidate;
            }
        }
        if let Some(parent) = current.parent() {
            for candidate in real_tsgo_candidates(&parent.join("corsa-bind")) {
                if candidate.exists() {
                    return candidate;
                }
            }
        }
        let direct = current.join("node_modules").join(".bin").join("tsgo");
        if direct.exists() {
            return direct;
        }
        let native_preview = current
            .join("node_modules")
            .join("@typescript")
            .join("native-preview")
            .join("bin")
            .join("tsgo");
        if native_preview.exists() {
            return native_preview;
        }
    }

    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        let common_locations = [
            home.join(".local/share/mise/shims/tsgo"),
            home.join(".asdf/shims/tsgo"),
            home.join(".volta/bin/tsgo"),
            home.join(".npm-global/bin/tsgo"),
            home.join(".npm/bin/tsgo"),
        ];
        for location in common_locations {
            if location.exists() {
                return location;
            }
        }
    }

    PathBuf::from("tsgo")
}

fn real_tsgo_candidates(root: &Path) -> [PathBuf; 6] {
    [
        root.join(".cache").join("tsgo"),
        root.join(".cache").join("tsgo.exe"),
        root.join("ref")
            .join("typescript-go")
            .join(".cache")
            .join("tsgo"),
        root.join("ref")
            .join("typescript-go")
            .join(".cache")
            .join("tsgo.exe"),
        root.join("ref")
            .join("typescript-go")
            .join("built")
            .join("local")
            .join("tsgo"),
        root.join("ref")
            .join("typescript-go")
            .join("built")
            .join("local")
            .join("tsgo.exe"),
    ]
}

fn byte_offset_to_utf16_offset(source: &str, byte_offset: u32) -> u32 {
    let mut clamped = usize::min(byte_offset as usize, source.len());
    while clamped > 0 && !source.is_char_boundary(clamped) {
        clamped -= 1;
    }
    source[..clamped].encode_utf16().count() as u32
}

fn push_unique_text(texts: &mut Vec<CompactString>, text: &str) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }
    if texts.iter().any(|existing| existing.as_str() == trimmed) {
        return;
    }
    texts.push(trimmed.to_compact_string());
}

fn push_u64(buffer: &mut String, value: u64) {
    let rendered = value.to_compact_string();
    buffer.push_str(rendered.as_str());
}

fn io_error_message(prefix: &str, path: &Path, error: &std::io::Error) -> String {
    let mut message = prefix.to_compact_string();
    message.push_str(": ");
    message.push_str(path.to_string_lossy().as_ref());
    message.push_str(": ");
    let detail = error.to_compact_string();
    message.push_str(detail.as_str());
    message
}

fn compact_error(prefix: &str, detail: &str) -> String {
    let mut message = prefix.to_compact_string();
    if !detail.is_empty() {
        message.push_str(": ");
        message.push_str(detail);
    }
    message
}

#[cfg(test)]
mod tests {
    use super::{byte_offset_to_utf16_offset, next_session_directory_name};

    #[test]
    fn utf16_offsets_follow_typescript_rules() {
        assert_eq!(byte_offset_to_utf16_offset("hello", 5), 5);
        assert_eq!(byte_offset_to_utf16_offset("a😀b", 1), 1);
        assert_eq!(byte_offset_to_utf16_offset("a😀b", 5), 3);
    }

    #[test]
    fn session_directory_names_are_unique() {
        let first = next_session_directory_name();
        let second = next_session_directory_name();
        assert_ne!(first, second);
    }
}
