use std::env;
use std::path::PathBuf;

#[cfg(windows)]
const EXE_SUFFIX: &str = ".exe";
#[cfg(not(windows))]
const EXE_SUFFIX: &str = "";

pub fn resolve_tool(tool: &str) -> Option<PathBuf> {
    if let Some(java_home) = env::var_os("JAVA_HOME") {
        let candidate = PathBuf::from(java_home)
            .join("bin")
            .join(format!("{tool}{EXE_SUFFIX}"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    let path = env::var_os("PATH")?;
    for dir in env::split_paths(&path) {
        let candidate = dir.join(format!("{tool}{EXE_SUFFIX}"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
