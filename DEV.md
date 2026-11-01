# Project Development

Run the commands below from the repository root. On Windows, activate the
Python development environment with `conda activate COS` first.

Component-specific instructions:

- [Python bindings](python/DEV.md): extension builds, generated stubs,
  Python tests, and documentation.
- [Browser bindings](wasm/DEV.md): official wasm-bindgen generation,
  WASM builds, and JavaScript/browser tests.

## Upgrade the Release Version

Preview and then apply a release bump:

```bash
python tools/bump_version.py 0.6.0-dev.1 --dry-run
python tools/bump_version.py 0.6.0-dev.1
```

The script accepts stable `X.Y.Z`, RC (`X.Y.ZrcN` or `X.Y.Z-rc.N`),
and development (`X.Y.Z.devN` or `X.Y.Z-dev.N`) versions. As in COSMolKit,
Rust uses `0.6.0-dev.1` and Python uses `0.6.0.dev1` for the same development
release. RC versions use `0.6.0-rc.1` and `0.6.0rc1`, respectively.

It updates the workspace package version, the three versioned internal workspace
dependencies, and `python/pyproject.toml`. The Rust facade, core,
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
