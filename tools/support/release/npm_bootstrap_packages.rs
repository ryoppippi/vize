//! Explicit first-publish approvals shared with the JS contract oracle.
use serde::Deserialize;
use std::sync::OnceLock;

pub const BOOTSTRAP_PACKAGE_PATH: &str = "npm/framework/nuxt-lint-config";
pub const REQUIRED_SUCCESSFUL_RELEASE_JOBS: &[&str] = &[
    "Build release npm packages",
    "Smoke release npm package installs",
    "release-preflight / Verify release safety contract",
];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BootstrapPackage {
    pub path: String,
    pub name: String,
    pub artifact: String,
    pub failed_jobs: Vec<String>,
    pub skipped_jobs: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Inventory {
    successful_jobs: Vec<String>,
    packages: Vec<BootstrapPackage>,
}

fn inventory() -> &'static Inventory {
    static INVENTORY: OnceLock<Inventory> = OnceLock::new();
    INVENTORY.get_or_init(|| {
        let inventory: Inventory = serde_json::from_str(include_str!(
            "../../config/release/npm-bootstrap-packages.json"
        ))
        .expect("the committed bootstrap inventory must be valid JSON");
        assert_eq!(inventory.successful_jobs, REQUIRED_SUCCESSFUL_RELEASE_JOBS);
        inventory
    })
}

pub fn approved_package(path: &str) -> Result<&'static BootstrapPackage, String> {
    inventory()
        .packages
        .iter()
        .find(|package| package.path == path)
        .ok_or_else(|| {
            format!(
                "Package path is not approved for npm bootstrap: {}",
                if path.is_empty() { "(empty)" } else { path }
            )
        })
}

pub fn approved_artifact(artifact: &str) -> Result<&'static BootstrapPackage, String> {
    inventory()
        .packages
        .iter()
        .find(|package| package.artifact == artifact)
        .ok_or_else(|| format!("Artifact is not approved for npm bootstrap: {artifact}"))
}
