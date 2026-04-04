use std::path::{Path, PathBuf};
use vize_carton::{cstr, String};

pub(super) fn find_node_modules_with_vue(start: &Path) -> Option<PathBuf> {
    let mut dir = start;
    loop {
        let node_modules = dir.join("node_modules");
        if node_modules.join("vue").is_dir() {
            return Some(node_modules);
        }
        dir = dir.parent()?;
    }
}

pub(super) fn resolve_temp_dir_base(project_root: Option<&Path>) -> PathBuf {
    let fallback_root = project_root
        .map(Path::to_path_buf)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));

    find_node_modules_with_vue(&fallback_root)
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .unwrap_or(fallback_root)
        .join("__agent_only")
        .join("vize-tsgo")
}

pub(super) fn find_tsgo_in_local_node_modules(working_dir: Option<&str>) -> Option<String> {
    let base_dir = working_dir
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())?;

    if let Some(path) = search_tsgo_in_dir(&base_dir) {
        return Some(path);
    }

    let mut current = base_dir.as_path();
    while let Some(parent) = current.parent() {
        if let Some(path) = search_tsgo_in_dir(parent) {
            return Some(path);
        }
        current = parent;
    }

    None
}

pub(super) fn find_tsgo_in_common_locations() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let candidates: [String; 10] = [
        cstr!("{home}/.npm-global/bin/tsgo"),
        cstr!("{home}/.npm/bin/tsgo"),
        cstr!("{home}/.local/share/pnpm/tsgo"),
        cstr!("{home}/.volta/bin/tsgo"),
        cstr!("{home}/.local/share/mise/shims/tsgo"),
        cstr!("{home}/.asdf/shims/tsgo"),
        cstr!("{home}/.local/share/fnm/node-versions/current/bin/tsgo"),
        cstr!("{home}/.nvm/versions/node/current/bin/tsgo"),
        "/opt/homebrew/bin/tsgo".into(),
        "/usr/local/bin/tsgo".into(),
    ];

    for path in candidates {
        if Path::new(path.as_str()).exists() {
            return Some(path);
        }
    }

    if let Ok(output) = std::process::Command::new("npm")
        .args(["root", "-g"])
        .output()
    {
        if output.status.success() {
            #[allow(clippy::disallowed_types)]
            let npm_root = std::string::String::from_utf8_lossy(&output.stdout);
            let npm_root = npm_root.trim();
            if let Some(lib_parent) = Path::new(npm_root).parent() {
                let tsgo_path = lib_parent.join("bin/tsgo");
                if tsgo_path.exists() {
                    return Some(tsgo_path.to_string_lossy().into());
                }
            }
        }
    }

    None
}

fn search_tsgo_in_dir(dir: &Path) -> Option<String> {
    let platform_suffix = platform_suffix();
    let pnpm_pattern = dir.join("node_modules/.pnpm");
    if pnpm_pattern.exists() {
        if let Ok(entries) = std::fs::read_dir(&pnpm_pattern) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("@typescript+native-preview-")
                    && name_str.contains(platform_suffix)
                {
                    let native_path = entry.path().join(&*cstr!(
                        "node_modules/@typescript/native-preview-{}/lib/tsgo",
                        platform_suffix
                    ));
                    if native_path.exists() {
                        return Some(native_path.to_string_lossy().into());
                    }
                }
            }
        }
    }

    let native_candidates = [
        dir.join(&*cstr!(
            "node_modules/@typescript/native-preview-{}/lib/tsgo",
            platform_suffix
        )),
        dir.join("node_modules/@typescript/native-preview/lib/tsgo"),
    ];
    for candidate in &native_candidates {
        if candidate.exists() {
            return Some(candidate.to_string_lossy().into());
        }
    }

    let fallback_candidates = [
        dir.join("node_modules/.bin/tsgo"),
        dir.join("node_modules/@typescript/native-preview/bin/tsgo"),
    ];
    for candidate in &fallback_candidates {
        if candidate.exists() {
            return Some(candidate.to_string_lossy().into());
        }
    }

    None
}

fn platform_suffix() -> &'static str {
    if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "darwin-arm64"
        } else {
            "darwin-x64"
        }
    } else if cfg!(target_os = "linux") {
        if cfg!(target_arch = "aarch64") {
            "linux-arm64"
        } else {
            "linux-x64"
        }
    } else if cfg!(target_os = "windows") {
        "win32-x64"
    } else {
        ""
    }
}
