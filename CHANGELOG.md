# Changelog

<!-- release-header:start -->
**COSMol Viewer** is a molecular visualization library for Rust, Python, and the web.

[Source repository](https://github.com/cosmol-studio/COSMol-viewer) ·
[Documentation](https://cosmol-studio.github.io/COSMol-viewer/) ·
[Web tools](https://tools.cosmol.org/) ·
[Rust crate](https://crates.io/crates/cosmol_viewer) ·
[Python package](https://pypi.org/project/cosmol-viewer/).
<!-- release-header:end -->

All notable changes to COSMol Viewer are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

The manually triggered GitHub Release workflow uses an unreleased `v*` tag on
the selected commit. It combines the release-header, matching dated version
section and release-footer without modifying an existing Release.

## [0.6.0-rc.1] - 2026-11-02

### Added

- Source-bound molecule/protein selections and independent display styles for
  Rust, Python and JavaScript, including named scene projections and source-based
  camera centering.
- A runnable browser example covering molecular and protein displays, selection
  highlighting, scene updates, animation and PNG screenshots. It installs the
  local Viewer build as an ordinary npm dependency.

### Changed

- Scientific objects now come directly from COSMolKit 0.6.0. Removed the viewer's
  Molecule/Protein wrappers; geometry generation is shared across all bindings.
- Rendering reads stored coordinates without implicit chemical transformations.
  Multiple molecular conformers require explicit selection.
- Python and JavaScript validate source identity and changed rendering inputs
  before inserting or replacing displays.
- Notebook bootstrapping uses a self-contained bundle of the generated browser
  module; public JavaScript callers retain the typed Scene and Animation APIs.

### Fixed

- Aligned publishing with COSMolKit: validate matching Rust/Python/WASM versions,
  build wheels in a shared platform matrix, and publish packages only for matching
  version tags or explicitly requested dev/RC builds.
- GitHub Releases are created separately on demand, preserve existing releases,
  accept equivalent Cargo/Python RC versions, and mark prereleases correctly.
- Notebook command tests pass large payloads through stdin instead of exceeding
  the Windows command-line length limit.

## [0.3.0] - 2026-08-28

### Changed

- Updated the COSMolKit integration from `0.2.12` to `0.3.0`.
- Converted COSMolKit's double-precision protein coordinates at the import
  boundary to the viewer's single-precision geometry representation.
- GitHub Releases now wait for the corresponding tag-triggered publish
  workflow to complete successfully before they are created.
- Updated the Rust workspace and Python package versions to `0.3.0` and
  refreshed transitive dependencies.

## [0.2.26] - 2026-08-23

### Added

- Added `Animation.to_payload()` for exporting compressed animation data for
  direct browser playback through `WebHandle.initiate_viewer_and_play`.
- Added an explicit `CMV1:` animation payload format marker and decoder
  support for both versioned and legacy unprefixed animation payloads.

### Changed

- Animation payload serialization now prepares a cloned animation, preserving
  the Python animation object and its scenes for later use.
- Expanded the Python API documentation for animation payload export and
  browser playback.
- Updated workspace and Python package metadata to version `0.2.26` and
  refreshed the lockfile dependencies.

### Fixed

- Kept the Linux EGL loader resident for the process lifetime to avoid native
  crashes caused by unloading the GL library while offscreen rendering state
  is still being released.
- Made isolated offscreen rendering exit cleanly after the image has been
  written, avoiding Python/Mesa teardown crashes in headless environments.

## [0.2.25] - 2026-08-20

### Added

- Added an isolated Mesa softpipe rendering profile for Colab and detailed
  offscreen EGL/GL stage tracing for native crash diagnostics.
- Added a complete Sphinx/Furo documentation site covering installation,
  scenes, camera and lighting controls, molecule representations, protein
  surfaces, geometric shapes, static rendering, interactive viewers,
  animation, Rust usage, and the generated Python API reference.
- Added strict documentation builds and GitHub Pages deployment for changes on
  the main branch.
- Added a dedicated GitHub Release workflow that validates release versions and
  uses the matching changelog section as the release notes.

### Changed

- Single-sample offscreen rendering now explicitly disables OpenGL
  multisampling so framebuffer configuration and GL state remain consistent.
- Updated COSMolKit integration from 0.2.11 to 0.2.12.
- Updated Rust dependencies used for spatial indexing, asset generation, and
  Linux dynamic loading.

### Fixed

- Fixed Google Colab static rendering crashes caused by Mesa llvmpipe even
  after the framebuffer had fallen back to a single sample.
- Fixed the documentation workflow's mismatched source and upload paths and
  prevented pull requests from deploying GitHub Pages.

[0.2.25]: https://github.com/cosmol-studio/COSMol-viewer/compare/v0.2.24...v0.2.25
[0.2.26]: https://github.com/cosmol-studio/COSMol-viewer/compare/v0.2.25...v0.2.26

<!-- release-footer:start -->
Installation and API usage: [COSMol Viewer documentation](https://viewer.cosmol.org/).
<!-- release-footer:end -->
[0.3.0]: https://github.com/cosmol-studio/COSMol-viewer/compare/v0.2.26...v0.3.0
