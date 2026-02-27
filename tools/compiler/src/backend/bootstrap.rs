use std::process::Command;

use crate::backend::CompilerBackend;
use crate::cli::CompileRequest;
use crate::config::BackendKind;
use crate::toolchain::resolve_tool;

pub struct BootstrapBackend;

impl CompilerBackend for BootstrapBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Bootstrap
    }

    fn status_line(&self) -> &'static str {
        "bootstrap: host javac delegation (compatibility path)"
    }

    fn compile(&self, request: &CompileRequest) -> Result<i32, String> {
        let javac = resolve_tool("javac")
            .ok_or_else(|| "host javac not found. Set JAVA_HOME or PATH.".to_string())?;

        let status = Command::new(&javac)
            .args(&request.forwarded_args)
            .status()
            .map_err(|e| format!("failed to run {:?}: {}", javac, e))?;

        Ok(status.code().unwrap_or(1))
    }
}
