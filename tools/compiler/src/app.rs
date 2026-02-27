use crate::backend::aria::AriaBackend;
use crate::backend::bootstrap::BootstrapBackend;
use crate::backend::CompilerBackend;
use crate::cli::{parse, print_wrapper_help, Command};
use crate::config::{target_java_major, BackendKind, BACKEND_ENV, TARGET_JAVA_VERSION};

pub fn run(raw_args: Vec<String>) -> i32 {
    let command = match parse(&raw_args) {
        Ok(cmd) => cmd,
        Err(err) => {
            eprintln!("aria-javac: {}", err);
            eprintln!("Use --aria-help for wrapper options.");
            return 2;
        }
    };

    match command {
        Command::PrintWrapperVersion => {
            println!("aria-javac wrapper {}", TARGET_JAVA_VERSION.trim());
            println!("default target major {}", target_java_major());
            0
        }
        Command::PrintWrapperHelp => {
            print_wrapper_help();
            0
        }
        Command::PrintBackendStatus => {
            println!("{}", BootstrapBackend.status_line());
            println!("{}", AriaBackend.status_line());
            0
        }
        Command::Compile(request) => {
            let backend_kind = match resolve_backend_kind(request.backend) {
                Ok(kind) => kind,
                Err(err) => {
                    eprintln!("aria-javac: {}", err);
                    return 2;
                }
            };

            let backend: Box<dyn CompilerBackend> = match backend_kind {
                BackendKind::Bootstrap => Box::new(BootstrapBackend),
                BackendKind::Aria => Box::new(AriaBackend),
            };

            match backend.compile(&request) {
                Ok(code) => code,
                Err(err) => {
                    eprintln!(
                        "aria-javac backend '{}' error: {}",
                        backend.kind().as_str(),
                        err
                    );
                    1
                }
            }
        }
    }
}

fn resolve_backend_kind(from_cli: Option<BackendKind>) -> Result<BackendKind, String> {
    if let Some(kind) = from_cli {
        return Ok(kind);
    }
    if let Some(value) = std::env::var_os(BACKEND_ENV) {
        let value = value.to_string_lossy();
        return BackendKind::parse(&value);
    }
    Ok(BackendKind::Bootstrap)
}
