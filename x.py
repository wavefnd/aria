#!/usr/bin/env python3
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).parent.resolve()

def run(cmd, cwd=None):
    print(f"\n[RUN] {' '.join(cmd)}")
    result = subprocess.run(cmd, cwd=cwd)
    if result.returncode != 0:
        print(f"\n❌ Error running {' '.join(cmd)}")
        sys.exit(result.returncode)

def build_rust(path):
    print(f"\n🦀 Building Rust project: {path.name}")
    run(["cargo", "build", "--release"], cwd=path)

def build_kotlin(path):
    print(f"\n☕ Building Kotlin project: {path.name}")
    gradlew = "gradlew" if os.name == "nt" else "./gradlew"
    if not (path / gradlew).exists():
        run(["gradle", "wrapper"], cwd=path)
    run([gradlew, "build"], cwd=path)

def main():
    print("🚀 AriaJDK Build System")
    print("=======================")

    build_rust(ROOT / "core")
    build_kotlin(ROOT / "classlib")
    build_kotlin(ROOT / "tools" / "compiler")
    build_rust(ROOT / "tools" / "jar")
    build_rust(ROOT / "launcher")

    print("\n✅ Build completed successfully!")
    print("Outputs:")
    print(" - core/target/release/libaria_core.a")
    print(" - classlib/build/libs/classlib.jar")
    print(" - launcher/target/release/aria")

if __name__ == "__main__":
    main()
