#!/usr/bin/env python3
import os
import platform
import shutil
import subprocess
import sys
from pathlib import Path
import tarfile
import zipfile

ROOT = Path(__file__).parent.resolve()
DIST = ROOT / "dist"
ARIAJDK_DIR = DIST / "AriaJDK"
BIN_DIR = ARIAJDK_DIR / "bin"
LIB_DIR = ARIAJDK_DIR / "lib"
INCLUDE_DIR = ARIAJDK_DIR / "include"

# -------------------------------
# Utility
# -------------------------------
def run(cmd, cwd=None):
    print(f"\n[RUN] {' '.join(cmd)}")
    result = subprocess.run(cmd, cwd=cwd)
    if result.returncode != 0:
        print(f"\n❌ Error running {' '.join(cmd)}")
        sys.exit(result.returncode)

def check_java():
    print("\n🔍 Checking Java environment...")
    java_home = os.environ.get("JAVA_HOME")
    result = shutil.which("java")
    if not java_home and not result:
        print("❌ JAVA_HOME is not set and 'java' not found in PATH.")
        print("Please install JDK 17+ and set JAVA_HOME before continuing.")
        sys.exit(1)
    print("✅ Java environment OK.")

# -------------------------------
# Build functions
# -------------------------------
def build_rust(path):
    print(f"\n🦀 Building Rust project: {path.name}")
    run(["cargo", "build", "--release"], cwd=path)

def build_kotlin(path):
    print(f"\n☕ Building Kotlin project: {path.name}")
    gradlew = path / ("gradlew.bat" if os.name == "nt" else "gradlew")

    if not gradlew.exists():
        run(["gradle", "wrapper"], cwd=path)

    if os.name == "nt":
        run([str(gradlew), "build"], cwd=path)
    else:
        run(["./gradlew", "build"], cwd=path)

# -------------------------------
# File placement
# -------------------------------
def copy_outputs():
    print("\n📦 Collecting build outputs...")

    # Create structure
    BIN_DIR.mkdir(parents=True, exist_ok=True)
    LIB_DIR.mkdir(parents=True, exist_ok=True)
    INCLUDE_DIR.mkdir(parents=True, exist_ok=True)

    # Rust binaries
    shutil.copy(ROOT / "launcher" / "target" / "release" / "aria", BIN_DIR / "java")   # JVM 실행기
    shutil.copy(ROOT / "tools" / "jar" / "target" / "release" / "jar", BIN_DIR / "jar")
    shutil.copy(ROOT / "core" / "target" / "release" / "libaria_core.a", LIB_DIR / "libaria_core.a")

    # Kotlin classlib
    shutil.copy(ROOT / "classlib" / "build" / "libs" / "classlib.jar", LIB_DIR / "aria-rt.jar")

    # ✅ JNI header copy (from core/include/)
    jni_source = ROOT / "core" / "include" / "jni.h"
    if jni_source.exists():
        shutil.copy(jni_source, INCLUDE_DIR / "jni.h")
        print(f"✅ Copied JNI header from {jni_source}")
    else:
        (INCLUDE_DIR / "jni.h").write_text("// JNI header placeholder (not found in source)\n")
        print("⚠️ No JNI header found in core/include/, created placeholder.")

    # release metadata
    (ARIAJDK_DIR / "release").write_text("ARIA_VERSION=0.1.0\nJAVA_VERSION=17\nVENDOR=Aria Foundation\n")

    print(f"✅ AriaJDK directory structure created at {ARIAJDK_DIR}")

# -------------------------------
# Packaging
# -------------------------------
def make_archive():
    print("\n🗜️ Packaging AriaJDK...")
    system = platform.system().lower()
    archive_name = f"ariajdk-{system}"

    if system == "windows":
        archive_path = DIST / f"{archive_name}.zip"
        with zipfile.ZipFile(archive_path, "w", zipfile.ZIP_DEFLATED) as zf:
            for root, _, files in os.walk(ARIAJDK_DIR):
                for file in files:
                    full_path = Path(root) / file
                    zf.write(full_path, arcname=str(full_path.relative_to(DIST)))
    else:
        archive_path = DIST / f"{archive_name}.tar.gz"
        with tarfile.open(archive_path, "w:gz") as tar:
            tar.add(ARIAJDK_DIR, arcname="AriaJDK")

    print(f"✅ Created archive: {archive_path}")
    return archive_path

# -------------------------------
# PATH integration
# -------------------------------
def add_to_path(bin_dir):
    system = platform.system().lower()
    print(f"\n⚙️  Adding {bin_dir} to PATH...")

    if system == "windows":
        run(["setx", "PATH", f"%PATH%;{bin_dir}"])
        print("🔗 PATH updated (restart terminal to apply).")
    elif system in ("linux", "darwin"):
        shell_rc = Path.home() / (".bashrc" if "bash" in os.environ.get("SHELL", "") else ".zshrc")
        with open(shell_rc, "a") as f:
            f.write(f"\n# Added by AriaJDK installer\nexport PATH=\"$PATH:{bin_dir}\"\n")
        print(f"🔗 PATH added to {shell_rc}. Restart shell to apply.")
    else:
        print("⚠️ Unknown OS, please add manually.")

# -------------------------------
# Main
# -------------------------------
def main():
    print("🚀 AriaJDK Universal Builder & Installer")
    print("========================================")

    check_java()

    # Clean build directory
    if DIST.exists():
        shutil.rmtree(DIST)
    DIST.mkdir(parents=True, exist_ok=True)

    # Build all modules
    build_rust(ROOT / "core")
    build_kotlin(ROOT / "classlib")
    build_kotlin(ROOT / "tools" / "compiler")
    build_rust(ROOT / "tools" / "jar")
    build_rust(ROOT / "launcher")

    # Copy & package
    copy_outputs()
    archive_path = make_archive()
    add_to_path(BIN_DIR)

    print("\n✅ All done!")
    print(f"🎉 AriaJDK successfully built and packaged at: {archive_path}")

if __name__ == "__main__":
    main()
