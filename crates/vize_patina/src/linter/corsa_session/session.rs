use super::{
    errors::{compact_error, io_error_message},
    paths::{
        allocate_session_root, path_to_wire, resolve_corsa_executable, resolve_project_root,
        TSCONFIG_CONTENTS, TSCONFIG_FILE_NAME, VIRTUAL_FILE_NAME,
    },
    ActiveProject, CorsaTypeAwareSession,
};
use corsa::{
    api::{
        ApiClient, ApiMode, ApiSpawnConfig, FileChangeSummary, FileChanges, UpdateSnapshotParams,
    },
    runtime::block_on,
};
use vize_carton::{String, ToCompactString};

impl CorsaTypeAwareSession {
    pub(in crate::linter) fn new(filename: &str) -> Result<Self, String> {
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
        let executable = resolve_corsa_executable(&project_root);
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

    pub(in crate::linter) fn matches_source_file(&self, filename: &str) -> bool {
        self.project_root == resolve_project_root(filename)
    }

    pub(in crate::linter) fn open_virtual_project(
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

    pub(in crate::linter) fn close(&mut self) {
        if self.closed {
            return;
        }
        self.closed = true;
        let _ = block_on(self.client.close());
        let _ = std::fs::remove_dir_all(&self.session_root);
    }
}
