pub mod aria;
pub mod bootstrap;

use crate::cli::CompileRequest;
use crate::config::BackendKind;

pub trait CompilerBackend {
    fn kind(&self) -> BackendKind;
    fn status_line(&self) -> &'static str;
    fn compile(&self, request: &CompileRequest) -> Result<i32, String>;
}
