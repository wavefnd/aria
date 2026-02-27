use aria_core::exec::interpreter::Interpreter;
use aria_core::loader::class_loader::ClassLoader;
use aria_core::runtime::heap::{Heap, HeapValue};
use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn has_javac() -> bool {
    Command::new("javac").arg("-version").output().is_ok()
}

fn compile_java(temp_dir: &std::path::Path, file_name: &str, source: &str) {
    let file_path = temp_dir.join(file_name);
    fs::write(&file_path, source).expect("write java source");

    let output = Command::new("javac")
        .arg("--release")
        .arg("17")
        .arg(file_path.to_string_lossy().to_string())
        .current_dir(temp_dir)
        .output()
        .expect("spawn javac");

    assert!(
        output.status.success(),
        "javac failed:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn executes_branch_and_ireturn_path() {
    if !has_javac() {
        return;
    }

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("aria-core-sum-{}", stamp));
    fs::create_dir_all(&dir).expect("mkdir");

    compile_java(
        &dir,
        "Main.java",
        r#"
        public class Main {
          public static int sum(int n) {
            int i = 0;
            int s = 0;
            while (i <= n) {
              s = s + i;
              i = i + 1;
            }
            return s;
          }
        }
        "#,
    );

    let mut loader = ClassLoader::new();
    loader.add_classpath(&dir);
    let class = loader.load_class("Main").expect("load class");
    let mut heap = Heap::new();
    let interp = Interpreter::new(false);

    let result = interp.execute_method(
        &mut loader,
        &class,
        "sum",
        "(I)I",
        &mut heap,
        &[HeapValue::Int(5)],
    );
    let _ = fs::remove_dir_all(&dir);

    match result {
        Some(HeapValue::Int(v)) => assert_eq!(v, 15),
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn executes_new_constructor_and_field_access() {
    if !has_javac() {
        return;
    }

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("aria-core-obj-{}", stamp));
    fs::create_dir_all(&dir).expect("mkdir");

    compile_java(
        &dir,
        "Main.java",
        r#"
        public class Main {
          int base;

          Main() {
            this.base = 40;
          }

          int add(int x) {
            return this.base + x;
          }

          public static int run() {
            Main c = new Main();
            return c.add(2);
          }
        }
        "#,
    );

    let mut loader = ClassLoader::new();
    loader.add_classpath(&dir);
    let class = loader.load_class("Main").expect("load class");
    let mut heap = Heap::new();
    let interp = Interpreter::new(false);

    let result = interp.execute_method(&mut loader, &class, "run", "()I", &mut heap, &[]);
    let _ = fs::remove_dir_all(&dir);

    match result {
        Some(HeapValue::Int(v)) => assert_eq!(v, 42),
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn executes_static_clinit_before_getstatic() {
    if !has_javac() {
        return;
    }

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("aria-core-clinit-{}", stamp));
    fs::create_dir_all(&dir).expect("mkdir");

    compile_java(
        &dir,
        "Main.java",
        r#"
        public class Main {
          static int seed;
          static {
            seed = 7;
          }

          public static int run() {
            return seed * 6;
          }
        }
        "#,
    );

    let mut loader = ClassLoader::new();
    loader.add_classpath(&dir);
    let class = loader.load_class("Main").expect("load class");
    let mut heap = Heap::new();
    let interp = Interpreter::new(false);

    let result = interp.execute_method(&mut loader, &class, "run", "()I", &mut heap, &[]);
    let _ = fs::remove_dir_all(&dir);

    match result {
        Some(HeapValue::Int(v)) => assert_eq!(v, 42),
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn executes_invokeinterface_dispatch() {
    if !has_javac() {
        return;
    }

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("aria-core-invokeinterface-{}", stamp));
    fs::create_dir_all(&dir).expect("mkdir");

    compile_java(
        &dir,
        "Main.java",
        r#"
        interface Adder {
          int add(int a, int b);
        }

        class Impl implements Adder {
          public int add(int a, int b) {
            return a + b;
          }
        }

        public class Main {
          public static int run() {
            Adder a = new Impl();
            return a.add(20, 22);
          }
        }
        "#,
    );

    let mut loader = ClassLoader::new();
    loader.add_classpath(&dir);
    let class = loader.load_class("Main").expect("load class");
    let mut heap = Heap::new();
    let interp = Interpreter::new(false);

    let result = interp.execute_method(&mut loader, &class, "run", "()I", &mut heap, &[]);
    let _ = fs::remove_dir_all(&dir);

    match result {
        Some(HeapValue::Int(v)) => assert_eq!(v, 42),
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn executes_invokedynamic_string_concat() {
    if !has_javac() {
        return;
    }

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("aria-core-indy-{}", stamp));
    fs::create_dir_all(&dir).expect("mkdir");

    compile_java(
        &dir,
        "Main.java",
        r#"
        public class Main {
          public static String run(int n) {
            return "value=" + n;
          }
        }
        "#,
    );

    let mut loader = ClassLoader::new();
    loader.add_classpath(&dir);
    let class = loader.load_class("Main").expect("load class");
    let mut heap = Heap::new();
    let interp = Interpreter::new(false);

    let result = interp.execute_method(
        &mut loader,
        &class,
        "run",
        "(I)Ljava/lang/String;",
        &mut heap,
        &[HeapValue::Int(5)],
    );
    let _ = fs::remove_dir_all(&dir);

    match result {
        Some(HeapValue::Object(obj)) => {
            let value = heap
                .get(obj.id)
                .and_then(|o| o.get_field("value"))
                .cloned()
                .expect("string value");
            match value {
                HeapValue::String(s) => assert_eq!(s, "value=5"),
                other => panic!("unexpected string payload: {:?}", other),
            }
        }
        other => panic!("unexpected result: {:?}", other),
    }
}
