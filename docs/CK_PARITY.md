# Documentation parity with COSMolKit

The documentation infrastructure is ported from the local COSMolKit checkout
at commit `16c7f305d70641e6faee0203d7148f620bc6bc5f`. Shared implementation is
copied, with project identifiers changed to COSMol Viewer.

| Area | Shared implementation |
| --- | --- |
| Presentation | Dioxus layout, responsive styles, navigation, footer, API member accordions, flat table of contents, anchor positioning |
| Source generation | One Sphinx project, Python stub-derived types, TypeDoc/sphinx-js JavaScript extraction, shared API layout and separate language indexes |
| Routing | Declarative route contract, generated Rust routes, language counterparts, clean URLs and legacy redirects |
| Search | Same Rust search engine, wasm-bindgen bundle, lazy loading, pagination, query history and failure recovery |
| Versions | Same catalog validation, native fallback menu, live catalog module, archive-origin detection, CORS and cache headers |
| Static deployment | Native Dioxus rendering, asset preparation, hydration removal, metadata, sitemap, crawler assets and Cloudflare Pages workflow |
| Checks | Route/metadata/deployment/API layout/version module tests, Rust search tests and real-browser layout/search/navigation checks |

## User-requested shared guide layout

Quick Start now uses one shared source with Python/JavaScript code selectors.
Its URL determines the language of all examples and capability-specific notes.
This explicitly requested layout extends the original CK presentation; other
pages have not yet been converted to the shared-guide format.

## Viewer-specific adaptations

- Python user guides do not include Rust setup or binding-maintainer build steps.
  Rust usage is kept in `cosmol_viewer/USAGE.rst`; developer setup remains in
  `python/DEV.md` and `wasm/DEV.md`. The former `/python/rust` route is removed.
- Validation and the empty Benchmarks placeholder are not Viewer documentation
  and have no page, route, navigation entry or sitemap entry.
- Branding, domains, repository/crate/package links, crawler verification and
  social assets identify Viewer. CK chemistry-specific crates are not linked.
- Tutorials, route topics, primary API class order and search examples describe
  Viewer scenes, selections, representations, animation and image export.
- Python imports the staged checkout extension from
  `target/python-bindings/package`, and reads `python/cosmol_viewer.pyi`.
  Import validation prevents the same-named Rust facade directory from being
  mistaken for an empty Python namespace package.
- JavaScript types and generated enum documentation come from `wasm/pkg`.
  Neither generated binding declarations nor generated JavaScript are patched.
- CI builds WASM before Python because Viewer embeds that bundle in its Python
  notebook integration. Both APIs are generated before Sphinx runs.
- The version catalog contains only actual configured Viewer deployments.
  CK archive URLs and release history are not Viewer releases. Add published
  Viewer archive roots to `docs-web/versions.json`; see the version setup in
  [the site README](../docs-web/README.md#documentation-versions).
- Test fixtures that exercise real APIs use Viewer symbols and routes. Generic
  synthetic fixtures retain their original arbitrary names. The migrated
  `tools/test_python_docs.py` covers language indexes, TypeDoc optional and
  rest arguments, factory aliases, getters/setters, overloads, documentation
  metadata and actual native Viewer signatures. CK's binding-generator AST
  rewriting is not used by Viewer; the documentation metadata handler is tested
  directly, and alias/overload protocols use a synthetic stub without editing
  generated Viewer declarations.
- Client-side fragment restoration opens the enclosing API member accordion
  before scrolling. The baseline CK client only scrolled, leaving deep-linked
  members hidden because scripts inside inserted article HTML do not execute.
  This explicit bug fix is covered by the development-mode browser check.
- The browser test supplies the local version catalog through request routing
  instead of depending on the live website. Search links are checked against
  indexable routes, including the homepage.
- Shared Python package versions in `uv.lock` match the CK baseline lock.
  CK-only chemistry/reference-test dependencies are not installed in Viewer.

## Local verification

Use the commands in [the source README](README.md) and
[the site README](../docs-web/README.md). The CI Sphinx command runs on Linux.
Windows builds use the checked-in `docs/sphinx_build.py` runner and Node loader
modules for sphinx-js 5 executable paths, drive-letter imports and temporary-file
sharing. This is an explicit build-tool addition to the CK baseline. It does not
alter API declarations or generated HTML; on Linux it calls Sphinx unchanged.
Both the documented Windows command and Dioxus's automatic Sphinx fallback use
this runner. No untracked `target/` helper is needed.

The GitHub Actions workflow includes the migrated documentation tests. Local
build/browser checks do not imply a remote Actions deployment has been run.
