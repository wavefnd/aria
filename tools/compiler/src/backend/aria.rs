pub mod ast;
mod codegen;
mod driver;
pub mod lexer;
pub mod parser;
pub mod sema;

use self::driver::run_frontend_pipeline;
use crate::backend::CompilerBackend;
use crate::cli::CompileRequest;
use crate::config::BackendKind;

pub struct AriaBackend;

impl CompilerBackend for AriaBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Aria
    }

    fn status_line(&self) -> &'static str {
        "aria: native frontend (lexer/parser/sema) in progress"
    }

    fn compile(&self, request: &CompileRequest) -> Result<i32, String> {
        run_frontend_pipeline(&request.forwarded_args).map(|_| 0)
    }
}
