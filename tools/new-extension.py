#!/usr/bin/env python3
"""Starts a new Volant list extension from a template, ready for ./build.sh.

Each template is the same small extension, which shows the typed text with its character and word
counts, plus a test. Rust builds for ABI 1 with the kit crate and joins the Cargo workspace.
TypeScript and Python build for ABI 2 and take their pinned toolchain scripts from the working npm and
wikipedia extensions, renamed, so a new extension builds the same way they do. The new folder has no
listing.json, so it stays out of the published catalog until one is added on purpose.

Usage:
  tools/new-extension.py rust|typescript|python FOLDER --name NAME [--id ID]
  tools/new-extension.py --self-test    start one of each in a scratch copy of the repo and run its tests
"""
import argparse
import json
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
TEMPLATES = ROOT / "templates"
LANGUAGES = ("rust", "typescript", "python")
FOLDER_PATTERN = re.compile(r"^[a-z][a-z0-9]{1,31}$")
ID_PATTERN = re.compile(r"^[a-z0-9]+(\.[a-z0-9-]+)+$")
PLACEHOLDER_HASH = "0" * 64
TYPESCRIPT_LICENSES = ("JAVY-LICENSE", "QUICKJS-LICENSE", "RQUICKJS-LICENSE", "JAVY-CRATES-LICENSES", "WASI-LIBC-LICENSE")
PYTHON_LICENSES = ("MICROPYTHON-LICENSE", "WASI-LIBC-LICENSE")
TYPESCRIPT_FILES = ("build.sh", "build.mjs", "package.json", "package-lock.json", "tsconfig.json",
                    "src/kit.ts", "src/main.ts") + TYPESCRIPT_LICENSES
PYTHON_FILES = ("build.sh", ".gitignore", "runtime/main.c", "runtime/mpconfigport.h", "runtime/embed.mk") + PYTHON_LICENSES


def manifest(folder, name, extension_id, abi):
    """The new extension's manifest; ./build.sh replaces the placeholder sha256 with the module's."""
    return {"abiVersion": abi, "id": extension_id, "name": name, "version": "0.1.0", "module": folder + ".wasm",
            "capabilities": [], "output": "items", "mode": "search", "placeholder": "Type some text",
            "timeoutSeconds": 2, "sha256": PLACEHOLDER_HASH}


def write(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def template(name, folder, display):
    """A template file with its folder and display name filled in."""
    return (TEMPLATES / name).read_text().replace("__FOLDER__", folder).replace("__NAME__", display)


def copy_renamed(root, source, target, files, replacements):
    """Copies files from a working extension, applying (old, new) text replacements in order."""
    for relative in files:
        text = (root / source / relative).read_text()
        for old, new in replacements:
            text = text.replace(old, new)
        destination = root / target / relative
        write(destination, text)
        shutil.copymode(root / source / relative, destination)


def add_to_workspace(root, folder):
    """Adds the folder to the Cargo workspace members, keeping kit first and the rest sorted."""
    path = root / "Cargo.toml"
    text = path.read_text()
    match = re.search(r"members = \[(.*?)\]", text)
    members = [member.strip().strip('"') for member in match.group(1).split(",") if member.strip()]
    others = sorted(set(members + [folder]) - {"kit"})
    path.write_text(text.replace(match.group(0), "members = [" + ", ".join(f'"{m}"' for m in ["kit"] + others) + "]"))


def create(root, language, folder, name, extension_id):
    target = root / folder
    if not FOLDER_PATTERN.match(folder):
        sys.exit("FOLDER must be 2 to 32 lowercase letters and digits, starting with a letter.")
    if target.exists():
        sys.exit(f"{folder} already exists.")
    if not ID_PATTERN.match(extension_id) or len(extension_id) > 128:
        sys.exit("ID must be a lowercase reverse-DNS name, such as com.example." + folder)
    if not 1 <= len(name) <= 128:
        sys.exit("NAME must be 1 to 128 characters.")
    abi = 1 if language == "rust" else 2
    if language == "rust":
        write(target / "Cargo.toml", template("rust/Cargo.toml", folder, name))
        write(target / "src/lib.rs", template("rust/lib.rs", folder, name))
        add_to_workspace(root, folder)
    elif language == "typescript":
        copy_renamed(root, "npm", folder, TYPESCRIPT_FILES, [
            ("Tests the npm extension", f"Tests the {name} extension"), ("builds npm.wasm", f"builds {folder}.wasm"),
            ("npm/build.sh", f"{folder}/build.sh"), ("}/npm.js", f"}}/{folder}.js"), ("}/npm.wasm", f"}}/{folder}.wasm"),
            ('"volant-npm"', f'"volant-{folder}"'),
            ('"./npm.ts"', f'"./{folder}.ts"')])
        write(target / f"src/{folder}.ts", template("typescript/extension.ts", folder, name))
        write(target / f"src/{folder}.test.ts", template("typescript/extension.test.ts", folder, name))
    else:
        copy_renamed(root, "wikipedia", folder, PYTHON_FILES, [
            ("Wikipedia", name), ("WIKIPEDIA", folder.upper()), ("wikipedia", folder)])
        write(target / f"src/{folder}.py", template("python/extension.py", folder, name))
        write(target / f"tests/test_{folder}.py", template("python/test_extension.py", folder, name))
    write(target / "manifest.json", json.dumps(manifest(folder, name, extension_id, abi), indent=2) + "\n")
    return target


def run(command, cwd):
    print("$", " ".join(command), flush=True)
    subprocess.run(command, cwd=cwd, check=True)


def self_test():
    """Starts one extension per language in a scratch copy of the tracked files and runs its tests,
    so a template or a renamed toolchain script that stops working fails CI."""
    with tempfile.TemporaryDirectory() as scratch:
        copy = pathlib.Path(scratch) / "repo"
        tracked = subprocess.run(["git", "ls-files", "-z"], cwd=ROOT, check=True, capture_output=True).stdout.split(b"\0")
        for relative in filter(None, (path.decode() for path in tracked)):
            if (ROOT / relative).is_file():
                (copy / relative).parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(ROOT / relative, copy / relative)
        for language, folder in (("rust", "samplerust"), ("typescript", "samplets"), ("python", "samplepy")):
            create(copy, language, folder, "Sample " + language.title(), "com.example." + folder)
            manifest_text = json.loads((copy / folder / "manifest.json").read_text())
            assert manifest_text["module"] == folder + ".wasm" and not (copy / folder / "listing.json").exists()
        run(["rustup", "run", toolchain(copy), "cargo", "test", "--quiet", "-p", "volant-samplerust"], copy)
        run(["npm", "ci", "--ignore-scripts", "--no-audit", "--no-fund", "--silent"], copy / "samplets")
        run(["npm", "test", "--silent"], copy / "samplets")
        run([sys.executable, "tests/test_samplepy.py"], copy / "samplepy")
        run([sys.executable, "tools/check-licenses.py"], copy)
    print("new-extension: every template starts and passes its tests")


def toolchain(root):
    return re.search(r'channel = "(.*)"', (root / "rust-toolchain.toml").read_text()).group(1)


def main():
    if sys.argv[1:] == ["--self-test"]:
        self_test()
        return
    parser = argparse.ArgumentParser(description="Start a new Volant list extension from a template.")
    parser.add_argument("language", choices=LANGUAGES)
    parser.add_argument("folder", help="the new folder, also the module and crate name, such as wordcount")
    parser.add_argument("--name", required=True, help="the name the launcher shows, such as \"Word Count\"")
    parser.add_argument("--id", help="a reverse-DNS ID; com.example.FOLDER when omitted")
    arguments = parser.parse_args()
    target = create(ROOT, arguments.language, arguments.folder, arguments.name, arguments.id or "com.example." + arguments.folder)
    print(f"Created {target.relative_to(ROOT)}/. Next: ./build.sh builds and pins it; copy the folder into Volant's "
          "Extensions folder to try it. Add a listing.json only when it is ready for the catalog.")


if __name__ == "__main__":
    main()
