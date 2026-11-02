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
python tools/bump_version.py 0.6.0-rc.1 --dry-run
python tools/bump_version.py 0.6.0-rc.1
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

## Build and publish releases

The release flow follows COSMolKit: build artifacts first, publish packages in a
single gated job, and create a GitHub Release separately on demand.

- `publish.yml` runs on `v*` tag pushes, pull requests and manual dispatch.
  `tools/publish_policy.py` checks matching Rust/Python/WASM versions and requires
  a pushed tag to equal `v` plus the Cargo version.
- Manual dispatch with `publish=false` only builds and validates. Setting it to
  `true` permits matching dev/RC versions; stable releases require a tag push.
  npm uses the `dev`, `rc` or `latest` dist-tag for the corresponding version.
- Browser bindings are tested before the optimized npm package is built.
  Python declarations and Notebook tests use that browser artifact. Wheel and
  sdist jobs then consume the same browser package and generated declarations.
- A single wheel matrix retains Linux x86_64/x86/aarch64, Windows x86_64/x86,
  and macOS x86_64/aarch64. Windows/macOS use `rust-cache`; Linux keeps the
  manylinux cache mounts and builds its bundled renderer inside the container.
- `release.yml` is manually dispatched on the selected commit/tag. It creates a
  Release only for one matching, unreleased `v*` tag; it never overwrites an
  existing Release. Select the intended tag ref when several pending tags point
  to one commit. Notes combine the changelog header, dated version section and
  footer. Dev/RC Releases are prereleases and do not replace the latest stable.

Local policy and workflow checks (without publishing):

```sh
python -m unittest discover -s tools -p 'test_*.py' -v
python tools/publish_policy.py
python wasm/build.py --check --browser-check
python python/build.py --check
python python/build_native_viewer.py
maturin build --locked --generate-stubs --manifest-path python/Cargo.toml --out target/release-check/wheels
maturin sdist --manifest-path python/Cargo.toml --out target/release-check/sdist
npm pack --dry-run --json ./wasm/pkg
```

After rebuilding WASM, run `npm ci` in `wasm/examples/browser` to refresh its
installed local package. The generated npm package and example lockfile should
report the same Cargo version before committing a release bump.

This workflow publishes Python and npm packages. Rust crate publication remains
separate, in dependency order: derive helper, core, then facade.
