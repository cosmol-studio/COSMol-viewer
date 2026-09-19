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

The text between the `release-header` markers is prepended to every GitHub
Release. The release workflow extracts the section matching the pushed tag.

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
[0.3.0]: https://github.com/cosmol-studio/COSMol-viewer/compare/v0.2.26...v0.3.0
