# Aria JDK

## Intall

You can build and install AriaJDK automatically using the unified Python build script.

```bash
python x.py
```

or, depending on your environment:

```bash
python3 x.py
```

### What this script does

- Build all Rust Components (`core`, `tools/jar`. `launcher`)
- Build all kotlin components (`classlib`, `tools/compiler`)
- Collects all binaries into `dist/bin/`
- Packages them into:
  - `ariajdk-windows.zip` on Windows
  - `ariajdk-linux.tar.gz` on Linux
  - `ariajdk-darwin.tar.gz` on macOS
- Automatically adds `dist/bin` to your system `PATH`

### After installation

Restart your terminal (or run `source ~/.bashrc` / `source ~/.zshrc` on macOS & Linux)

and you can use:

```bash
aria --version
```

to verify that AriaJDK was installed successfully.