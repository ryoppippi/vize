use super::{
    paths::{
        find_corsa_in_common_locations, find_corsa_in_local_node_modules, find_corsa_in_path,
        find_node_modules_with_vue, resolve_temp_dir_base,
    },
    utils::{json_from_value, parse_uri},
    CorsaLspClient,
};
use corsa_lsp::{LspClient, LspSpawnConfig, VirtualChange, VirtualDocument};
use corsa_runtime::block_on;
use lsp_types::{
    notification::{Exit, Initialized},
    request::{Initialize, Shutdown},
    InitializeParams, InitializedParams, WorkspaceFolder,
};
use serde_json::json;
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};
use vize_carton::{cstr, String, ToCompactString};

impl CorsaLspClient {
    /// Start the Corsa LSP server.
    pub fn new(corsa_path: Option<&str>, working_dir: Option<&str>) -> Result<Self, String> {
        let executable: String = corsa_path
            .map(String::from)
            .or_else(|| std::env::var("CORSA_PATH").ok().map(String::from))
            .or_else(|| std::env::var("TSGO_PATH").ok().map(String::from))
            .or_else(|| find_corsa_in_local_node_modules(working_dir))
            .or_else(find_corsa_in_common_locations)
            .or_else(find_corsa_in_path)
            .unwrap_or_else(|| "corsa".into());

        eprintln!("\x1b[90m[corsa] Using: {executable}\x1b[0m");

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

        install_node_modules_link(project_root.as_deref(), &temp_dir_path);
        write_temp_tsconfig(&temp_dir_path)?;

        let client = block_on(LspClient::spawn(
            LspSpawnConfig::new(executable.as_str()).with_cwd(temp_dir_path.clone()),
        ))
        .map_err(|e| cstr!("Failed to start Corsa LSP: {e}"))?;
        let overlay = client.overlay();
        let events = client.subscribe();

        let temp_root = temp_dir_path.canonicalize().ok();
        let mut client = Self {
            client,
            overlay,
            events,
            diagnostics: Default::default(),
            diagnostic_result_ids: Default::default(),
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
            .map_err(|e| cstr!("Failed to initialize Corsa LSP: {e}"))?;
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
        block_on(self.client.close()).map_err(|e| cstr!("Failed to close Corsa LSP: {e}"))?;
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
}

impl Drop for CorsaLspClient {
    fn drop(&mut self) {
        let _ = self.shutdown();
        if let Some(ref dir) = self.temp_dir {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

fn install_node_modules_link(project_root: Option<&Path>, temp_dir_path: &Path) {
    let node_modules_path = project_root.and_then(find_node_modules_with_vue);
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
}

/// Write a minimal `tsconfig.json` that keeps the native checker in strict mode.
fn write_temp_tsconfig(temp_dir_path: &Path) -> Result<(), String> {
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
    .map_err(|e| cstr!("Failed to write temp tsconfig.json: {e}"))
}
