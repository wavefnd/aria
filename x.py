#!/usr/bin/env python3
import os
import platform
import shutil
import subprocess
import sys
from pathlib import Path
import tarfile
import zipfile
import datetime

# ============================================================
# Color Definitions
# ============================================================
RED = "\033[91m"
GREEN = "\033[92m"
YELLOW = "\033[93m"
WHITE = "\033[97m"
RESET = "\033[0m"

def ctext(text, color):
    return f"{color}{text}{RESET}"

# ============================================================
# Paths
# ============================================================
ROOT = Path(__file__).parent.resolve()
DIST = ROOT / "dist"
ARIAJDK_DIR = DIST / "AriaJDK"
BIN_DIR = ARIAJDK_DIR / "bin"
LIB_DIR = ARIAJDK_DIR / "lib"
INCLUDE_DIR = ARIAJDK_DIR / "include"

# ============================================================
# Version Readers
# ============================================================
def get_version():
    version_file = ROOT / "VERSION"
    if version_file.exists():
        return version_file.read_text().strip()
    print(ctext("VERSION file not found, defaulting to 0.0.0", YELLOW))
    return "0.0.0"

def get_java_version():
    version_file = ROOT / "VERSION_JAVA"
    if version_file.exists():
        return version_file.read_text().strip()
    print(ctext("VERSION_JAVA file not found, defaulting to unknown", YELLOW))
    return "unknown"

# ============================================================
# Utility
# ============================================================
def run(cmd, cwd=None):
    print(ctext(f"\n[RUN] {' '.join(cmd)}", YELLOW))
    result = subprocess.run(cmd, cwd=cwd)
    if result.returncode != 0:
        print(ctext(f"\n✗ Error running {' '.join(cmd)}", RED))
        sys.exit(result.returncode)

def check_java():
    print(ctext("\nChecking Java environment...", WHITE))
    java_home = os.environ.get("JAVA_HOME")
    result = shutil.which("java")
    if not java_home and not result:
        print(ctext("JAVA_HOME is not set and 'java' command not found in PATH.", RED))
        print(ctext("Please install a JDK and set JAVA_HOME before continuing.", YELLOW))
        sys.exit(1)
    print(ctext("Java environment OK.", GREEN))

# ============================================================
# Build Functions
# ============================================================
def build_rust(path):
    print(ctext(f"\nBuilding Rust project: {path.name}", WHITE))
    run(["cargo", "build", "--release"], cwd=path)

def build_kotlin(path):
    print(ctext(f"\nBuilding Kotlin project: {path.name}", WHITE))
    gradlew = path / ("gradlew.bat" if os.name == "nt" else "gradlew")
    if not gradlew.exists():
        run(["gradle", "wrapper"], cwd=path)
    if os.name == "nt":
        run([str(gradlew), "build"], cwd=path)
    else:
        run(["./gradlew", "build"], cwd=path)

# ============================================================
# Output & Packaging
# ============================================================
def copy_outputs():
    print(ctext("\nCollecting build outputs...", WHITE))

    BIN_DIR.mkdir(parents=True, exist_ok=True)
    LIB_DIR.mkdir(parents=True, exist_ok=True)
    INCLUDE_DIR.mkdir(parents=True, exist_ok=True)

    # Rust outputs
    shutil.copy(ROOT / "launcher" / "target" / "release" / "aria", BIN_DIR / "java")
    shutil.copy(ROOT / "tools" / "jar" / "target" / "release" / "jar", BIN_DIR / "jar")
    shutil.copy(ROOT / "core" / "target" / "release" / "libaria_core.a", LIB_DIR / "libaria_core.a")

    # Kotlin outputs
    shutil.copy(ROOT / "classlib" / "build" / "libs" / "classlib.jar", LIB_DIR / "aria-rt.jar")

    # JNI Header
    jni_source = ROOT / "core" / "include" / "jni.h"
    if jni_source.exists():
        shutil.copy(jni_source, INCLUDE_DIR / "jni.h")
        print(ctext(f"Copied JNI header from {jni_source}", GREEN))
    else:
        (INCLUDE_DIR / "jni.h").write_text("// JNI header placeholder\n")
        print(ctext("No JNI header found; created placeholder.", YELLOW))

    # Metadata
    aria_version = get_version()
    java_version = get_java_version()
    build_date = datetime.datetime.now().strftime("%Y-%m-%d")

    release_content = f'''IMPLEMENTOR="Wave Foundation"
IMPLEMENTOR_VERSION="{aria_version}"
JAVA_VERSION="{java_version}"
OS_ARCH="{platform.machine()}"
OS_NAME="{platform.system().lower()}"
SOURCE="https://github.com/wavefnd/aria"
BUILD_DATE="{build_date}"
'''
    (ARIAJDK_DIR / "release").write_text(release_content)
    print(ctext("Created release metadata", GREEN))

    add_javac_stub()
    ensure_java_base_module()
    verify_runtime()

    print(ctext(f"AriaJDK directory structure ready at {ARIAJDK_DIR}", GREEN))

# ============================================================
# Enhancements
# ============================================================
def verify_runtime():
    print(ctext("\nVerifying AriaJDK runtime output...", WHITE))
    java_exec = BIN_DIR / "java"
    if not java_exec.exists():
        print(ctext("Runtime binary not found!", RED))
        sys.exit(1)
    result = subprocess.run([str(java_exec), "-version"], capture_output=True, text=True)
    output = result.stdout.strip() or result.stderr.strip()
    if result.returncode == 0:
        print(ctext("Runtime responded:\n", GREEN) + ctext(output, WHITE))
    else:
        print(ctext("Runtime did not respond correctly. Check launcher build.", RED))

def add_javac_stub():
    print(ctext("\nEnsuring javac stub exists...", WHITE))
    javac_stub = BIN_DIR / ("javac.bat" if os.name == "nt" else "javac")
    if not javac_stub.exists():
        content = (
            "@echo off\n"
            "echo AriaJDK does not include a Java compiler.\n"
            "echo Use external javac or kotlinc instead.\n"
            if os.name == "nt" else
            "#!/usr/bin/env bash\n"
            "echo 'AriaJDK does not include a Java compiler.'\n"
            "echo 'Use external javac or kotlinc instead.'\n"
        )
        javac_stub.write_text(content)
        if os.name != "nt":
            os.chmod(javac_stub, 0o755)
        print(ctext(f"Added javac stub at {javac_stub}", GREEN))
    else:
        print(ctext("javac stub already exists.", GREEN))

def ensure_java_base_module():
    print(ctext("\nEnsuring java.base module structure...", WHITE))
    base_dir = LIB_DIR / "modules" / "java.base" / "java" / "lang"
    base_dir.mkdir(parents=True, exist_ok=True)
    object_class = base_dir / "Object.class"
    if not object_class.exists():
        object_class.write_bytes(b"\xca\xfe\xba\xbe")  # dummy header
        print(ctext("Added dummy java.lang.Object.class", GREEN))
    else:
        print(ctext("java.base module already present.", GREEN))

# ============================================================
# Packaging
# ============================================================
def make_archive():
    print(ctext("\nPackaging AriaJDK...", WHITE))
    system = platform.system().lower()
    aria_version = get_version()
    archive_name = f"ariajdk-{aria_version}-{system}"

    if system == "windows":
        archive_path = DIST / f"{archive_name}.zip"
        with zipfile.ZipFile(archive_path, "w", zipfile.ZIP_DEFLATED) as zf:
            for root, _, files in os.walk(ARIAJDK_DIR):
                for file in files:
                    full_path = Path(root) / file
                    zf.write(full_path, arcname=str(full_path.relative_to(ARIAJDK_DIR.parent)))
    else:
        archive_path = DIST / f"{archive_name}.tar.gz"
        with tarfile.open(archive_path, "w:gz") as tar:
            tar.add(ARIAJDK_DIR, arcname="AriaJDK")

    print(ctext(f"Created archive: {archive_path}", GREEN))
    return archive_path

# ============================================================
# PATH Integration
# ============================================================
def add_to_path(bin_dir):
    system = platform.system().lower()
    print(ctext(f"\nAdding {bin_dir} to PATH...", WHITE))
    if system == "windows":
        run(["setx", "PATH", f"%PATH%;{bin_dir}"])
        print(ctext("PATH updated. Restart terminal to apply.", GREEN))
    elif system in ("linux", "darwin"):
        shell_rc = Path.home() / (".bashrc" if "bash" in os.environ.get("SHELL", "") else ".zshrc")
        with open(shell_rc, "a") as f:
            f.write(f"\n# Added by AriaJDK installer\nexport PATH=\"$PATH:{bin_dir}\"\n")
        print(ctext(f"PATH added to {shell_rc}. Restart shell to apply.", GREEN))
    else:
        print(ctext("Unknown OS, please add manually.", YELLOW))

# ============================================================
# Main
# ============================================================
def main():
    print(ctext("AriaJDK Universal Builder & Installer", GREEN))
    print(ctext("======================================", WHITE))

    check_java()

    if DIST.exists():
        shutil.rmtree(DIST)
    DIST.mkdir(parents=True, exist_ok=True)

    build_rust(ROOT / "core")
    build_kotlin(ROOT / "classlib")
    build_kotlin(ROOT / "tools" / "compiler")
    build_rust(ROOT / "tools" / "jar")
    build_rust(ROOT / "launcher")

    copy_outputs()
    archive_path = make_archive()
    add_to_path(BIN_DIR)

    print(ctext("\nBuild completed successfully!", GREEN))
    print(ctext(f"AriaJDK package: {archive_path}", WHITE))

# ============================================================
# Entry Point
# ============================================================
if __name__ == "__main__":
    try:
        main()
    except Exception as e:
        print(ctext(f"\nBuild failed: {e}", RED))
        if DIST.exists():
            print(ctext("Cleaning dist directory...", YELLOW))
            shutil.rmtree(DIST, ignore_errors=True)
        sys.exit(1)
