use super::{
    paths::{find_corsa_in_common_locations, find_corsa_in_local_node_modules, find_corsa_in_path},
    session::spawn_project_session,
    CorsaLspClient,
};
use corsa_lsp::{LspClient, LspSpawnConfig};
use corsa_runtime::block_on;
use std::path::PathBuf;
use vize_carton::{cstr, String};

pub(super) fn resolve_corsa_executable(
    corsa_path: Option<&str>,
    working_dir: Option<&str>,
) -> String {
    corsa_path
        .map(String::from)
        .or_else(|| std::env::var("CORSA_PATH").ok().map(String::from))
        .or_else(|| std::env::var("TSGO_PATH").ok().map(String::from))
        .or_else(|| find_corsa_in_local_node_modules(working_dir))
        .or_else(find_corsa_in_common_locations)
        .or_else(find_corsa_in_path)
        .unwrap_or_else(|| "corsa".into())
}

impl CorsaLspClient {
    pub(super) fn spawn_initialized_client(
        executable: &str,
        cwd: PathBuf,
        root_path: Option<PathBuf>,
        temp_dir: Option<PathBuf>,
    ) -> Result<Self, String> {
        let project_root = root_path.as_deref().unwrap_or(&cwd);
        let (session, capabilities) = spawn_project_session(executable, &cwd, project_root)?;
        let client = block_on(LspClient::spawn(
            LspSpawnConfig::new(executable).with_cwd(cwd),
        ))
        .map_err(|e| cstr!("Failed to start Corsa LSP: {e}"))?;
        let overlay = client.overlay();
        let events = client.subscribe();

        let mut client = Self {
            client,
            overlay,
            session,
            capabilities,
            events,
            diagnostics: Default::default(),
            diagnostic_result_ids: Default::default(),
            overlay_versions: Default::default(),
            document_texts: Default::default(),
            temp_dir,
            closed: false,
        };
        client.initialize(root_path.as_ref())?;
        client.drain_pending_messages();
        Ok(client)
    }
}
