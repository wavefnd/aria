use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const TARGET_JAVA_VERSION: &str = include_str!("../../../VERSION_JAVA");

#[cfg(windows)]
const EXE_SUFFIX: &str = ".exe";
#[cfg(not(windows))]
const EXE_SUFFIX: &str = "";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Create,
    List,
    Extract,
    Update,
}

#[derive(Clone)]
struct InputSpec {
    base_dir: PathBuf,
    rel_path: PathBuf,
}

struct ParsedArgs {
    mode: Mode,
    jar_file: PathBuf,
    verbose: bool,
    no_manifest: bool,
    input_specs: Vec<InputSpec>,
    entry_filters: Vec<String>,
}

fn print_usage() {
    eprintln!("Usage: jar [--create|--list|--extract|--update] --file <jar> [options] [files]");
    eprintln!("       jar c|t|x|u[f][v][M] <jar> [-C dir file] [file ...]");
    eprintln!();
    eprintln!("Supported options:");
    eprintln!("  -c, --create         Create archive");
    eprintln!("  -t, --list           List contents");
    eprintln!("  -x, --extract        Extract entries");
    eprintln!("  -u, --update         Update archive");
    eprintln!("  -f, --file <jar>     Archive path");
    eprintln!("  -v, --verbose        Verbose output");
    eprintln!("  -M, --no-manifest    Do not write default manifest on create");
    eprintln!("  -C <dir> <file>      Add file from a different base directory");
}

fn resolve_tool(tool: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    for dir in env::split_paths(&path) {
        let candidate = dir.join(format!("{tool}{EXE_SUFFIX}"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn set_mode(target: &mut Option<Mode>, mode: Mode) -> Result<(), String> {
    if let Some(existing) = target {
        if *existing != mode {
            return Err("Only one operation mode is allowed.".to_string());
        }
    } else {
        *target = Some(mode);
    }
    Ok(())
}

fn parse_cluster(
    cluster: &str,
    mode: &mut Option<Mode>,
    verbose: &mut bool,
    no_manifest: &mut bool,
    pending_file_inline: &mut Option<String>,
    needs_file_value: &mut bool,
) -> Result<(), String> {
    let mut iter = cluster.char_indices().peekable();
    while let Some((pos, ch)) = iter.next() {
        match ch {
            'c' => set_mode(mode, Mode::Create)?,
            't' => set_mode(mode, Mode::List)?,
            'x' => set_mode(mode, Mode::Extract)?,
            'u' => set_mode(mode, Mode::Update)?,
            'v' => *verbose = true,
            'M' => *no_manifest = true,
            'f' => {
                if let Some((next_pos, _)) = iter.peek().copied() {
                    let inline = cluster[next_pos..].to_string();
                    *pending_file_inline = Some(inline);
                    break;
                } else {
                    *needs_file_value = true;
                }
            }
            _ => {
                return Err(format!("Unknown short option: -{}", ch));
            }
        }

        if ch == 'f' && pos + ch.len_utf8() < cluster.len() {
            break;
        }
    }
    Ok(())
}

fn parse_args(args: &[String]) -> Result<ParsedArgs, String> {
    if args.is_empty() {
        return Err("No arguments.".to_string());
    }

    let mut mode: Option<Mode> = None;
    let mut jar_file: Option<PathBuf> = None;
    let mut verbose = false;
    let mut no_manifest = false;
    let mut input_specs: Vec<InputSpec> = Vec::new();
    let mut entry_filters: Vec<String> = Vec::new();
    let mut pending_base_dir: Option<PathBuf> = None;
    let mut stop_option_parse = false;
    let mut idx = 0usize;

    while idx < args.len() {
        let arg = &args[idx];

        if stop_option_parse {
            if matches!(mode, Some(Mode::Create | Mode::Update)) {
                let base = pending_base_dir.take().unwrap_or_else(|| PathBuf::from("."));
                input_specs.push(InputSpec {
                    base_dir: base,
                    rel_path: PathBuf::from(arg),
                });
            } else {
                entry_filters.push(arg.clone());
            }
            idx += 1;
            continue;
        }

        if idx == 0 && !arg.starts_with('-') {
            let mut inline_file = None;
            let mut needs_file = false;
            parse_cluster(
                arg,
                &mut mode,
                &mut verbose,
                &mut no_manifest,
                &mut inline_file,
                &mut needs_file,
            )?;
            if let Some(file) = inline_file {
                jar_file = Some(PathBuf::from(file));
            } else if needs_file {
                idx += 1;
                if idx >= args.len() {
                    return Err("Missing archive file value for -f.".to_string());
                }
                jar_file = Some(PathBuf::from(&args[idx]));
            }
            idx += 1;
            continue;
        }

        if arg == "--" {
            stop_option_parse = true;
            idx += 1;
            continue;
        }

        if arg == "-C" {
            idx += 1;
            let Some(dir) = args.get(idx) else {
                return Err("Missing directory for -C.".to_string());
            };
            pending_base_dir = Some(PathBuf::from(dir));
            idx += 1;
            continue;
        }

        if let Some(file) = arg.strip_prefix("--file=") {
            jar_file = Some(PathBuf::from(file));
            idx += 1;
            continue;
        }

        match arg.as_str() {
            "--create" | "-c" => {
                set_mode(&mut mode, Mode::Create)?;
                idx += 1;
                continue;
            }
            "--list" | "-t" => {
                set_mode(&mut mode, Mode::List)?;
                idx += 1;
                continue;
            }
            "--extract" | "-x" => {
                set_mode(&mut mode, Mode::Extract)?;
                idx += 1;
                continue;
            }
            "--update" | "-u" => {
                set_mode(&mut mode, Mode::Update)?;
                idx += 1;
                continue;
            }
            "--verbose" | "-v" => {
                verbose = true;
                idx += 1;
                continue;
            }
            "--no-manifest" | "-M" => {
                no_manifest = true;
                idx += 1;
                continue;
            }
            "--file" | "-f" => {
                idx += 1;
                let Some(file) = args.get(idx) else {
                    return Err("Missing value for --file/-f.".to_string());
                };
                jar_file = Some(PathBuf::from(file));
                idx += 1;
                continue;
            }
            _ => {}
        }

        if arg.starts_with('-') && arg.len() > 1 {
            let cluster = arg.trim_start_matches('-');
            let mut inline_file = None;
            let mut needs_file = false;
            parse_cluster(
                cluster,
                &mut mode,
                &mut verbose,
                &mut no_manifest,
                &mut inline_file,
                &mut needs_file,
            )?;
            if let Some(file) = inline_file {
                jar_file = Some(PathBuf::from(file));
            } else if needs_file {
                idx += 1;
                let Some(file) = args.get(idx) else {
                    return Err("Missing archive file value for -f.".to_string());
                };
                jar_file = Some(PathBuf::from(file));
            }
            idx += 1;
            continue;
        }

        if matches!(mode, Some(Mode::Create | Mode::Update)) {
            let base = pending_base_dir.take().unwrap_or_else(|| PathBuf::from("."));
            input_specs.push(InputSpec {
                base_dir: base,
                rel_path: PathBuf::from(arg),
            });
        } else {
            entry_filters.push(arg.clone());
        }
        idx += 1;
    }

    if pending_base_dir.is_some() {
        return Err("Option -C requires a following file path.".to_string());
    }

    let mode = mode.ok_or_else(|| "No operation mode specified.".to_string())?;
    let jar_file = jar_file.ok_or_else(|| "No archive file specified (--file/-f).".to_string())?;

    Ok(ParsedArgs {
        mode,
        jar_file,
        verbose,
        no_manifest,
        input_specs,
        entry_filters,
    })
}

fn to_absolute(path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    let cwd = env::current_dir().map_err(|e| format!("Failed to read current directory: {}", e))?;
    Ok(cwd.join(path))
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directory {}: {}", parent.display(), e))?;
    }
    Ok(())
}

fn run_status(mut cmd: Command, label: &str) -> Result<(), String> {
    let status = cmd
        .status()
        .map_err(|e| format!("Failed to execute {}: {}", label, e))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{} exited with status {}", label, status))
    }
}

fn write_default_manifest() -> Result<PathBuf, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("System time error: {}", e))?
        .as_millis();
    let temp_root = env::temp_dir().join(format!("aria-jar-manifest-{}-{}", std::process::id(), now));
    let meta_inf = temp_root.join("META-INF");
    fs::create_dir_all(&meta_inf)
        .map_err(|e| format!("Failed to create temp manifest directory {}: {}", meta_inf.display(), e))?;
    let manifest = meta_inf.join("MANIFEST.MF");
    let mut file =
        fs::File::create(&manifest).map_err(|e| format!("Failed to create {}: {}", manifest.display(), e))?;
    file.write_all(b"Manifest-Version: 1.0\nCreated-By: AriaJDK\n\n")
        .map_err(|e| format!("Failed to write {}: {}", manifest.display(), e))?;
    Ok(temp_root)
}

fn zip_add_spec(zip_bin: &Path, jar_abs: &Path, spec: &InputSpec, verbose: bool) -> Result<(), String> {
    let base = to_absolute(&spec.base_dir)?;
    let source_path = base.join(&spec.rel_path);
    if !source_path.exists() {
        return Err(format!(
            "Input path does not exist: {}",
            source_path.display()
        ));
    }

    let mut cmd = Command::new(zip_bin);
    cmd.current_dir(&base);
    if !verbose {
        cmd.arg("-q");
    }
    if source_path.is_dir() {
        cmd.arg("-r");
    }
    cmd.arg(jar_abs);
    cmd.arg(&spec.rel_path);
    run_status(cmd, "zip")
}

fn run_create_or_update(parsed: &ParsedArgs) -> Result<(), String> {
    let zip_bin = resolve_tool("zip").ok_or_else(|| {
        "zip command not found in PATH. Install zip utility to use Aria jar tool.".to_string()
    })?;

    let jar_abs = to_absolute(&parsed.jar_file)?;
    ensure_parent(&jar_abs)?;

    if parsed.mode == Mode::Create && jar_abs.exists() {
        fs::remove_file(&jar_abs)
            .map_err(|e| format!("Failed to replace {}: {}", jar_abs.display(), e))?;
    }

    let mut specs = parsed.input_specs.clone();
    let mut temp_manifest_root: Option<PathBuf> = None;

    if parsed.mode == Mode::Create && !parsed.no_manifest {
        let manifest_root = write_default_manifest()?;
        specs.insert(
            0,
            InputSpec {
                base_dir: manifest_root.clone(),
                rel_path: PathBuf::from("META-INF"),
            },
        );
        temp_manifest_root = Some(manifest_root);
    }

    if specs.is_empty() {
        return Err("No input files provided for archive operation.".to_string());
    }

    for spec in &specs {
        zip_add_spec(&zip_bin, &jar_abs, spec, parsed.verbose)?;
    }

    if let Some(path) = temp_manifest_root {
        let _ = fs::remove_dir_all(path);
    }

    Ok(())
}

fn run_list(parsed: &ParsedArgs) -> Result<(), String> {
    let unzip_bin = resolve_tool("unzip").ok_or_else(|| {
        "unzip command not found in PATH. Install unzip utility to use Aria jar tool.".to_string()
    })?;
    let jar_abs = to_absolute(&parsed.jar_file)?;
    if !jar_abs.exists() {
        return Err(format!("Archive not found: {}", jar_abs.display()));
    }

    if parsed.verbose {
        let mut cmd = Command::new(&unzip_bin);
        cmd.arg("-l").arg(&jar_abs);
        return run_status(cmd, "unzip -l");
    }

    let output = Command::new(&unzip_bin)
        .arg("-Z")
        .arg("-1")
        .arg(&jar_abs)
        .output()
        .map_err(|e| format!("Failed to execute unzip: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    let filter_set: HashSet<&str> = parsed.entry_filters.iter().map(|s| s.as_str()).collect();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if filter_set.is_empty() || filter_set.contains(line) {
            println!("{}", line);
        }
    }
    Ok(())
}

fn run_extract(parsed: &ParsedArgs) -> Result<(), String> {
    let unzip_bin = resolve_tool("unzip").ok_or_else(|| {
        "unzip command not found in PATH. Install unzip utility to use Aria jar tool.".to_string()
    })?;

    let jar_abs = to_absolute(&parsed.jar_file)?;
    if !jar_abs.exists() {
        return Err(format!("Archive not found: {}", jar_abs.display()));
    }

    let mut cmd = Command::new(unzip_bin);
    if !parsed.verbose {
        cmd.arg("-qq");
    }
    cmd.arg("-o");
    cmd.arg(jar_abs);
    if !parsed.entry_filters.is_empty() {
        cmd.args(&parsed.entry_filters);
    }
    run_status(cmd, "unzip -o")
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if matches!(args.as_slice(), [v] if v == "-version" || v == "--version") {
        println!("jar {}-aria", TARGET_JAVA_VERSION.trim());
        return;
    }
    if matches!(args.as_slice(), [v] if v == "-h" || v == "--help" || v == "-?") {
        print_usage();
        return;
    }

    let parsed = match parse_args(&args) {
        Ok(p) => p,
        Err(err) => {
            eprintln!("AriaJDK jar error: {}", err);
            print_usage();
            std::process::exit(2);
        }
    };

    let result = match parsed.mode {
        Mode::Create | Mode::Update => run_create_or_update(&parsed),
        Mode::List => run_list(&parsed),
        Mode::Extract => run_extract(&parsed),
    };

    if let Err(err) = result {
        eprintln!("AriaJDK jar error: {}", err);
        std::process::exit(1);
    }
}
