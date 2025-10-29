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
BIN_DIR = DIST / "bin"

def run(cmd, cwd=None):
    print(f"\n[RUN] {' '.join(cmd)}")
    result = subprocess.run(cmd, cwd=cwd)
    if result.returncode != 0:
        print(f"\n❌ Error running {' '.join(cmd)}")
        sys.exit(result.returncode)

def build_rust(path):
    print(f"\n🦀 Building Rust project: {path.name}")
    run(["cargo", "build", "--release"], cwd=path)

def check_java():
    print("\n🔍 Checking Java environment...")
    java_home = os.environ.get("JAVA_HOME")
    result = shutil.which("java")
    if not java_home and not result:
        print("❌ JAVA_HOME is not set and 'java' not found in PATH.")
        print("Please install JDK 17+ and set JAVA_HOME before continuing.")
        sys.exit(1)
    print("✅ Java environment OK.")


def build_kotlin(path):
    check_java()
    print(f"\n☕ Building Kotlin project: {path.name}")

    gradlew = path / ("gradlew.bat" if os.name == "nt" else "gradlew")

    if not gradlew.exists():
        run(["gradle", "wrapper"], cwd=path)

    if os.name == "nt":
        run([str(gradlew), "build"], cwd=path)
    else:
        run(["./gradlew", "build"], cwd=path)

def copy_outputs():
    print("\n📦 Collecting build outputs...")

    BIN_DIR.mkdir(parents=True, exist_ok=True)

    shutil.copy(ROOT / "launcher" / "target" / "release" / "aria", BIN_DIR)
    shutil.copy(ROOT / "core" / "target" / "release" / "libaria_core.a", BIN_DIR)
    shutil.copy(ROOT / "classlib" / "build" / "libs" / "classlib.jar", BIN_DIR)

    print(f"✅ Copied to {BIN_DIR}")

def make_archive():
    print("\n🗜️ Packaging binaries...")
    system = platform.system().lower()
    archive_name = f"ariajdk-{system}"

    if system == "windows":
        archive_path = DIST / f"{archive_name}.zip"
        with zipfile.ZipFile(archive_path, "w", zipfile.ZIP_DEFLATED) as zf:
            for file in BIN_DIR.iterdir():
                zf.write(file, arcname=file.name)
    else:
        archive_path = DIST / f"{archive_name}.tar.gz"
        with tarfile.open(archive_path, "w:gz") as tar:
            for file in BIN_DIR.iterdir():
                tar.add(file, arcname=file.name)

    print(f"✅ Created archive: {archive_path}")
    return archive_path

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

def main():
    print("🚀 AriaJDK Build & Installer System")
    print("===================================")

    # Clean dist
    if DIST.exists():
        shutil.rmtree(DIST)
    DIST.mkdir(parents=True, exist_ok=True)

    # Build steps
    build_rust(ROOT / "core")
    build_kotlin(ROOT / "classlib")
    build_kotlin(ROOT / "tools" / "compiler")
    build_rust(ROOT / "tools" / "jar")
    build_rust(ROOT / "launcher")

    # Collect, package, and install
    copy_outputs()
    archive_path = make_archive()
    add_to_path(BIN_DIR)

    print("\n✅ All done!")
    print(f"🎉 AriaJDK is ready! Archive: {archive_path}")

if __name__ == "__main__":
    main()
