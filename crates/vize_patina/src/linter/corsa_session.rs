use corsa::api::{ApiClient, ManagedSnapshot, ProjectHandle};
use std::path::PathBuf;
use vize_carton::{CompactString, String};

mod errors;
mod paths;
mod probe;
mod session;

#[cfg(test)]
mod tests;

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

impl Drop for CorsaTypeAwareSession {
    fn drop(&mut self) {
        self.close();
    }
}
