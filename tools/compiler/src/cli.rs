use crate::config::BackendKind;

#[derive(Debug, Clone)]
pub enum Command {
    PrintWrapperVersion,
    PrintBackendStatus,
    PrintWrapperHelp,
    Compile(CompileRequest),
}

#[derive(Debug, Clone)]
pub struct CompileRequest {
    pub backend: Option<BackendKind>,
    pub forwarded_args: Vec<String>,
}

pub fn parse(raw_args: &[String]) -> Result<Command, String> {
    if matches!(raw_args, [v] if v == "--aria-version") {
        return Ok(Command::PrintWrapperVersion);
    }
    if matches!(raw_args, [v] if v == "--aria-self-host-status") {
        return Ok(Command::PrintBackendStatus);
    }
    if matches!(raw_args, [v] if v == "--aria-help") {
        return Ok(Command::PrintWrapperHelp);
    }

    let mut backend: Option<BackendKind> = None;
    let mut forwarded_args: Vec<String> = Vec::with_capacity(raw_args.len());
    let mut idx = 0usize;
    while idx < raw_args.len() {
        let arg = &raw_args[idx];
        if arg == "--aria-backend" {
            idx += 1;
            let Some(value) = raw_args.get(idx) else {
                return Err("Missing value for --aria-backend.".to_string());
            };
            backend = Some(BackendKind::parse(value)?);
            idx += 1;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--aria-backend=") {
            backend = Some(BackendKind::parse(value)?);
            idx += 1;
            continue;
        }

        forwarded_args.push(arg.clone());
        idx += 1;
    }

    Ok(Command::Compile(CompileRequest {
        backend,
        forwarded_args,
    }))
}

pub fn print_wrapper_help() {
    eprintln!("Aria javac wrapper options:");
    eprintln!("  --aria-version            Print Aria wrapper version");
    eprintln!("  --aria-self-host-status   Print backend implementation status");
    eprintln!("  --aria-backend=<name>     Select backend (bootstrap|aria)");
    eprintln!("  --aria-help               Show wrapper options");
    eprintln!();
    eprintln!("All other arguments are forwarded to the selected backend.");
    eprintln!("Default backend: bootstrap (delegates to host javac).");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BackendKind;

    #[test]
    fn parses_backend_inline() {
        let args = vec![
            "--aria-backend=bootstrap".to_string(),
            "-d".to_string(),
            "out".to_string(),
        ];
        let cmd = parse(&args).unwrap();
        match cmd {
            Command::Compile(req) => {
                assert_eq!(req.backend, Some(BackendKind::Bootstrap));
                assert_eq!(
                    req.forwarded_args,
                    vec!["-d".to_string(), "out".to_string()]
                );
            }
            _ => panic!("expected compile command"),
        }
    }
}
