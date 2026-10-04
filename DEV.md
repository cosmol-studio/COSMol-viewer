# Project Development

Run the commands below from the repository root. On Windows, activate the
Python development environment with `conda activate COS` first.

Component-specific instructions:

- [Python bindings](crates/python/DEV.md): extension builds, generated stubs,
  Python tests, and documentation.
- [Browser bindings](crates/wasm/tools/wasm_binding/README.md): Alef generation,
  WASM builds, and JavaScript/browser tests.

## Upgrade the Release Version

Preview and then apply a release bump:

```bash
python tools/bump_version.py 0.5.0rc3 --dry-run
python tools/bump_version.py 0.5.0rc3
```

The script accepts `X.Y.Z`, Python-style `X.Y.ZrcN`, and Rust-style
`X.Y.Z-rc.N`. For example, Python uses `0.5.0rc3` while Rust uses
`0.5.0-rc.3` for the same prerelease.

It updates the workspace package version, the three versioned internal workspace
dependencies, and `crates/python/pyproject.toml`. The Rust facade, core,
Python binding, WASM binding, and derive helper inherit the workspace version.
It then runs `cargo update --workspace` to refresh their lockfile entries without
requesting a blanket third-party dependency upgrade.

`--dry-run` only prints the planned edits; it does not write files or run Cargo.
All manifest edits are prepared before writing. If the Cargo command fails,
the script reports failure and leaves the version edits available for inspection
and retry.

COSMolKit dependency versions, historical changelog entries, and the documentation
site's published-package pin are not release fields managed by this script.
Generated WASM assets and native binaries must be rebuilt through the normal
build workflow; the script does not publish packages, create tags, or commit
changes.

Regression check:

```bash
python tools/test_bump_version.py -v
```
