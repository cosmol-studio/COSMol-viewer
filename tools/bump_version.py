#!/usr/bin/env python3
"""Update explicit Rust/Python release fields, then refresh workspace lock entries."""

import argparse
import difflib
from pathlib import Path
import re
import subprocess
import tomllib


ROOT = Path(__file__).resolve().parents[1]
FILES = ("Cargo.toml", "crates/python/pyproject.toml")
INTERNAL_DEPENDENCIES = {
    "cosmol_viewer": "cosmol_viewer",
    "cosmol_viewer_core": "crates/core",
}
LOCK_REFRESH = ["cargo", "update", "--workspace"]


def release_versions(version):
    """Accept X.Y.Z, X.Y.ZrcN, or X.Y.Z-rc.N; return Cargo and Python forms."""
    number = r"(?:0|[1-9][0-9]*)"
    match = re.fullmatch(
        rf"(?P<base>{number}\.{number}\.{number})(?:(?:rc|-rc\.)(?P<rc>{number}))?",
        version,
    )
    if match is None:
        raise ValueError("expected X.Y.Z, X.Y.ZrcN, or X.Y.Z-rc.N")
    base, rc = match.group("base", "rc")
    if rc is None:
        return base, base
    return f"{base}-rc.{rc}", f"{base}rc{rc}"


def update_field(text, section, key, version, *, dependency=False):
    """Change one named field while preserving comments, formatting, and newlines."""
    # Check the value structurally before doing a surgical text edit.
    value = tomllib.loads(text)
    try:
        for part in section.split("."):
            value = value[part]
        value = value[key]
    except KeyError as error:
        raise ValueError(f"missing field: {section}.{key}") from error
    if dependency:
        if not isinstance(value, dict) or not isinstance(value.get("version"), str):
            raise ValueError(f"expected versioned dependency: {section}.{key}")
    elif not isinstance(value, str):
        raise ValueError(f"expected string field: {section}.{key}")

    lines = text.splitlines(keepends=True)
    header = re.compile(rf"\[{re.escape(section)}\]\s*(?:#.*)?")
    sections = [index for index, line in enumerate(lines) if header.fullmatch(line.strip())]
    if len(sections) != 1:
        raise ValueError(f"expected one section: {section}")
    start = sections[0] + 1
    end = next((index for index in range(start, len(lines)) if lines[index].lstrip().startswith("[")), len(lines))
    field = re.compile(rf"^[ \t]*{re.escape(key)}[ \t]*=")
    matches = [index for index in range(start, end) if field.match(lines[index])]
    if len(matches) != 1:
        raise ValueError(f"expected one field: {section}.{key}")
    index = matches[0]
    prefix, old, suffix = lines[index].split('"', 2)
    layout = rf"[ \t]*{re.escape(key)}[ \t]*=[ \t]*"
    if dependency:
        layout += r"\{[ \t]*version[ \t]*=[ \t]*"
    if re.fullmatch(layout, prefix) is None:
        raise ValueError(f"unexpected field layout: {section}.{key}")
    lines[index] = prefix + '"' + version + '"' + suffix
    return "".join(lines)


def prepare_updates(original, version):
    """Validate and prepare every edit before changing any repository file."""
    rust_version, python_version = release_versions(version)
    updated = original.copy()
    manifest = tomllib.loads(original["Cargo.toml"])
    for name, path in INTERNAL_DEPENDENCIES.items():
        dependency = manifest["workspace"]["dependencies"].get(name)
        if not isinstance(dependency, dict) or dependency.get("path") != path:
            raise ValueError(f"expected internal path dependency: {name}")
    text = update_field(original["Cargo.toml"], "workspace.package", "version", rust_version)
    for name in INTERNAL_DEPENDENCIES:
        text = update_field(text, "workspace.dependencies", name, rust_version, dependency=True)
    updated["Cargo.toml"] = text
    updated["crates/python/pyproject.toml"] = update_field(
        original["crates/python/pyproject.toml"], "project", "version", python_version
    )
    return updated


def read_release_files(root):
    original = {}
    for file in FILES:
        with (root / file).open(encoding="utf-8", newline="") as stream:
            original[file] = stream.read()
    return original


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("version", help="for example: 0.5.0rc1 or 0.5.0-rc.1")
    parser.add_argument("--dry-run", action="store_true", help="show edits without writing or invoking Cargo")
    args = parser.parse_args(argv)
    try:
        release_versions(args.version)
        original = read_release_files(ROOT)
        updated = prepare_updates(original, args.version)
    except (ValueError, OSError) as error:
        parser.error(str(error))

    for file in FILES:
        if original[file] == updated[file]:
            continue
        if args.dry_run:
            print("".join(difflib.unified_diff(
                original[file].replace("\r\n", "\n").splitlines(True),
                updated[file].replace("\r\n", "\n").splitlines(True), file, file
            )), end="")
        else:
            with (ROOT / file).open("w", encoding="utf-8", newline="") as stream:
                stream.write(updated[file])
            print(f"Updated {file}")
    if args.dry_run:
        print("Would then run cargo update --workspace.")
    elif subprocess.run(LOCK_REFRESH, cwd=ROOT).returncode:
        parser.exit(1, "cargo update --workspace failed; version edits remain.\n")


if __name__ == "__main__":
    main()
