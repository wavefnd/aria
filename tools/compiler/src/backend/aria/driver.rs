use std::fs;
use std::path::PathBuf;

use crate::backend::aria::codegen::emit_classes;
use crate::backend::aria::lexer::lex;
use crate::backend::aria::parser::parse;
use crate::backend::aria::sema::{analyze, Diagnostic, SourceFileAst};
use crate::config::{target_java_major, TARGET_JAVA_VERSION};

pub fn run_frontend_pipeline(args: &[String]) -> Result<(), String> {
    let inputs = parse_inputs(args)?;
    match inputs.action {
        ToolAction::Help => {
            print_javac_help();
            return Ok(());
        }
        ToolAction::Version => {
            println!("aria-javac {}", target_java_major());
            return Ok(());
        }
        ToolAction::FullVersion => {
            println!("aria-javac full version {}", TARGET_JAVA_VERSION.trim());
            return Ok(());
        }
        ToolAction::Compile => {}
    }

    let sources = inputs.sources;
    if sources.is_empty() {
        return Err("no Java source files provided.".to_string());
    }

    let mut parsed_files = Vec::new();
    let mut diagnostics = Vec::new();

    for source_path in sources {
        let path = normalize_path(source_path);
        let source = match fs::read_to_string(&path) {
            Ok(src) => src,
            Err(err) => {
                diagnostics.push(Diagnostic {
                    path: path.clone(),
                    line: 1,
                    col: 1,
                    message: format!("failed to read source file: {}", err),
                });
                continue;
            }
        };

        let tokens = match lex(&source) {
            Ok(tokens) => tokens,
            Err(err) => {
                diagnostics.push(Diagnostic {
                    path: path.clone(),
                    line: err.line,
                    col: err.col,
                    message: format!("lex error: {}", err.message),
                });
                continue;
            }
        };

        let unit = match parse(tokens) {
            Ok(unit) => unit,
            Err(err) => {
                diagnostics.push(Diagnostic {
                    path: path.clone(),
                    line: err.span.line,
                    col: err.span.col,
                    message: format!("parse error: {}", err.message),
                });
                continue;
            }
        };

        parsed_files.push(SourceFileAst { path, unit });
    }

    diagnostics.extend(analyze(&parsed_files));

    if !diagnostics.is_empty() {
        for diag in diagnostics {
            eprintln!(
                "{}:{}:{}: error: {}",
                diag.path.display(),
                diag.line,
                diag.col,
                diag.message
            );
        }
        return Err("compilation failed in Aria frontend pipeline".to_string());
    }

    match emit_classes(&parsed_files, &inputs.output_dir) {
        Ok(()) => Ok(()),
        Err(codegen_errors) => {
            for err in codegen_errors {
                eprintln!(
                    "{}:{}:{}: error: {}",
                    err.path.display(),
                    err.line,
                    err.col,
                    err.message
                );
            }
            Err("code generation failed in Aria backend".to_string())
        }
    }
}

#[derive(Debug, Clone)]
struct FrontendInputs {
    sources: Vec<PathBuf>,
    output_dir: PathBuf,
    action: ToolAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolAction {
    Compile,
    Help,
    Version,
    FullVersion,
}

fn parse_inputs(args: &[String]) -> Result<FrontendInputs, String> {
    let mut output_dir = PathBuf::from(".");
    let mut action = ToolAction::Compile;
    let mut collected = Vec::new();
    let mut expanded = Vec::new();
    for arg in args {
        if let Some(file) = arg.strip_prefix('@') {
            expanded.extend(read_argfile(file)?);
        } else {
            expanded.push(arg.clone());
        }
    }

    let mut idx = 0usize;
    while idx < expanded.len() {
        let arg = &expanded[idx];
        if is_help_flag(arg) {
            action = ToolAction::Help;
            idx += 1;
            continue;
        }
        if arg == "-version" || arg == "--version" {
            action = ToolAction::Version;
            idx += 1;
            continue;
        }
        if arg == "-fullversion" || arg == "--full-version" {
            action = ToolAction::FullVersion;
            idx += 1;
            continue;
        }
        if arg == "-d" {
            idx += 1;
            let Some(value) = expanded.get(idx) else {
                return Err("missing value for -d".to_string());
            };
            output_dir = PathBuf::from(value);
            idx += 1;
            continue;
        }
        if let Some(value) = arg.strip_prefix("-d=") {
            output_dir = PathBuf::from(value);
            idx += 1;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--directory=") {
            output_dir = PathBuf::from(value);
            idx += 1;
            continue;
        }

        if arg.starts_with('-') {
            if takes_value(arg) {
                idx += 1;
                let Some(value) = expanded.get(idx) else {
                    return Err(format!("missing value for {}", arg));
                };
                validate_java_level_option(arg, value)?;
                idx += 1;
                continue;
            }
            if is_supported_no_value_option(arg) {
                idx += 1;
                continue;
            }
            return Err(format!("unsupported option for aria backend: {}", arg));
        }

        if arg.ends_with(".java") {
            collected.push(PathBuf::from(arg));
            idx += 1;
            continue;
        }
        return Err(format!("unsupported input argument: {}", arg));
    }

    Ok(FrontendInputs {
        sources: if action == ToolAction::Compile {
            collected
        } else {
            Vec::new()
        },
        output_dir: normalize_path(output_dir),
        action,
    })
}

fn read_argfile(path: &str) -> Result<Vec<String>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("failed to read argfile '{}': {}", path, e))?;
    let mut args = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        args.extend(trimmed.split_whitespace().map(|s| s.to_string()));
    }
    Ok(args)
}

fn takes_value(option: &str) -> bool {
    matches!(
        option,
        "-cp"
            | "-classpath"
            | "--class-path"
            | "-d"
            | "-encoding"
            | "-sourcepath"
            | "--source-path"
            | "-processorpath"
            | "--processor-path"
            | "-bootclasspath"
            | "--module-path"
            | "--module-source-path"
            | "--system"
            | "--release"
            | "-source"
            | "-target"
            | "--limit-modules"
    )
}

fn is_help_flag(arg: &str) -> bool {
    matches!(arg, "-h" | "--help" | "-help" | "-?")
}

fn is_supported_no_value_option(arg: &str) -> bool {
    matches!(
        arg,
        "-g" | "-g:none"
            | "-nowarn"
            | "-deprecation"
            | "-verbose"
            | "-Werror"
            | "-parameters"
            | "-proc:none"
            | "-proc:only"
            | "-implicit:none"
            | "-implicit:class"
            | "--enable-preview"
    ) || arg.starts_with("-Xlint")
}

fn validate_java_level_option(option: &str, value: &str) -> Result<(), String> {
    if !matches!(option, "--release" | "-source" | "-target") {
        return Ok(());
    }
    let Some(major) = parse_java_major(value) else {
        return Err(format!(
            "invalid Java version value '{}' for {}",
            value, option
        ));
    };
    if major != 17 {
        return Err(format!(
            "aria backend currently supports only Java 17 for {} (received '{}')",
            option, value
        ));
    }
    Ok(())
}

fn parse_java_major(value: &str) -> Option<u16> {
    let token = value.split('.').next()?;
    token.parse::<u16>().ok()
}

fn print_javac_help() {
    println!("Usage: aria-javac <options> <source files>");
    println!("Supported subset (aria backend):");
    println!("  -d <directory>                Output directory");
    println!("  -cp|-classpath <path>         Classpath (accepted)");
    println!("  --release 17                  Source/target level (Java 17 only)");
    println!("  -source 17                    Source level (Java 17 only)");
    println!("  -target 17                    Target level (Java 17 only)");
    println!("  -h, --help, -help, -?         Print this help");
    println!("  -version, --version           Print compiler version");
    println!("  -fullversion, --full-version  Print full version");
    println!("Unsupported options fail fast for consistency.");
}

fn normalize_path(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(&path))
            .unwrap_or(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn collects_java_sources_from_args() {
        let args = vec![
            "-d".to_string(),
            "out".to_string(),
            "A.java".to_string(),
            "--class-path".to_string(),
            "libs".to_string(),
            "src/B.java".to_string(),
        ];
        let inputs = parse_inputs(&args).unwrap();
        assert_eq!(inputs.action, ToolAction::Compile);
        assert!(inputs.output_dir.ends_with("out"));
        assert_eq!(
            inputs.sources,
            vec![PathBuf::from("A.java"), PathBuf::from("src/B.java")]
        );
    }

    #[test]
    fn frontend_pipeline_accepts_simple_class() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("aria-frontend-test-{}", stamp));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("Sample.java");
        fs::write(
            &file,
            r#"
            public class Sample {
              public static void main(String[] args) {
                int x = 1;
                System.out.println(x);
              }
            }
            "#,
        )
        .unwrap();

        let result = run_frontend_pipeline(&[
            "-d".to_string(),
            dir.to_string_lossy().to_string(),
            file.to_string_lossy().to_string(),
        ]);
        assert!(dir.join("Sample.class").exists(), "expected class output");
        let _ = fs::remove_dir_all(&dir);
        assert!(result.is_ok(), "expected ok, got {:?}", result);
    }

    #[test]
    fn frontend_pipeline_accepts_control_flow_class() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("aria-frontend-flow-test-{}", stamp));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("Flow.java");
        fs::write(
            &file,
            r#"
            public class Flow {
              public static void main(String[] args) {
                int i = 0;
                int sum = 0;
                while (i < 5) {
                  if ((i % 2) == 0 || i == 3) {
                    sum = sum + i;
                  }
                  i = i + 1;
                }
                System.out.println(sum);
              }
            }
            "#,
        )
        .unwrap();

        let result = run_frontend_pipeline(&[
            "-d".to_string(),
            dir.to_string_lossy().to_string(),
            file.to_string_lossy().to_string(),
        ]);
        let class_file = dir.join("Flow.class");
        assert!(class_file.exists(), "expected class output");
        let bytes = fs::read(&class_file).expect("read class file");
        assert!(bytes.len() > 8, "class file too small");
        let major = u16::from_be_bytes([bytes[6], bytes[7]]);
        assert_eq!(major, 61, "expected Java 17 classfile major version");
        assert!(
            bytes
                .windows("StackMapTable".len())
                .any(|w| w == b"StackMapTable"),
            "expected StackMapTable in constant pool"
        );
        let _ = fs::remove_dir_all(&dir);
        assert!(result.is_ok(), "expected ok, got {:?}", result);
    }

    #[test]
    fn frontend_pipeline_accepts_instance_and_class_receiver_calls() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("aria-frontend-calls-test-{}", stamp));
        fs::create_dir_all(&dir).unwrap();

        let a_file = dir.join("A.java");
        fs::write(
            &a_file,
            r#"
            class A {
              int base;

              int add(int x) {
                return x + this.base;
              }

              int twice(int x) {
                return add(x) + this.base;
              }

              static int inc(int x) {
                return x + 1;
              }
            }
            "#,
        )
        .unwrap();

        let b_file = dir.join("B.java");
        fs::write(
            &b_file,
            r#"
            public class B {
              public static void main(String[] args) {
                System.out.println(A.inc(4));
              }
            }
            "#,
        )
        .unwrap();

        let result = run_frontend_pipeline(&[
            "-d".to_string(),
            dir.to_string_lossy().to_string(),
            a_file.to_string_lossy().to_string(),
            b_file.to_string_lossy().to_string(),
        ]);
        assert!(dir.join("A.class").exists(), "expected A.class output");
        assert!(dir.join("B.class").exists(), "expected B.class output");
        let _ = fs::remove_dir_all(&dir);
        assert!(result.is_ok(), "expected ok, got {:?}", result);
    }

    #[test]
    fn frontend_pipeline_accepts_object_creation_with_default_constructor() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("aria-frontend-new-test-{}", stamp));
        fs::create_dir_all(&dir).unwrap();

        let foo_file = dir.join("Foo.java");
        fs::write(
            &foo_file,
            r#"
            class Foo {
              int plusOne(int x) {
                return x + 1;
              }
            }
            "#,
        )
        .unwrap();

        let main_file = dir.join("Main.java");
        fs::write(
            &main_file,
            r#"
            public class Main {
              public static void main(String[] args) {
                Foo f = new Foo();
                System.out.println(f.plusOne(4));
              }
            }
            "#,
        )
        .unwrap();

        let result = run_frontend_pipeline(&[
            "-d".to_string(),
            dir.to_string_lossy().to_string(),
            foo_file.to_string_lossy().to_string(),
            main_file.to_string_lossy().to_string(),
        ]);
        assert!(dir.join("Foo.class").exists(), "expected Foo.class output");
        assert!(
            dir.join("Main.class").exists(),
            "expected Main.class output"
        );
        let _ = fs::remove_dir_all(&dir);
        assert!(result.is_ok(), "expected ok, got {:?}", result);
    }

    #[test]
    fn parse_inputs_accepts_help_without_sources() {
        let args = vec!["--help".to_string()];
        let inputs = parse_inputs(&args).unwrap();
        assert_eq!(inputs.action, ToolAction::Help);
        assert!(inputs.sources.is_empty());
    }

    #[test]
    fn parse_inputs_rejects_unsupported_option() {
        let args = vec!["--definitely-unsupported".to_string()];
        let err = parse_inputs(&args).unwrap_err();
        assert!(err.contains("unsupported option"));
    }

    #[test]
    fn parse_inputs_rejects_non_17_release() {
        let args = vec![
            "--release".to_string(),
            "21".to_string(),
            "A.java".to_string(),
        ];
        let err = parse_inputs(&args).unwrap_err();
        assert!(err.contains("Java 17"));
    }
}
