# COSMol Viewer Documentation

The documentation site uses the same Dioxus shell, CSS, route contract,
Sphinx article integration, and Rust/WASM search as COSMolKit. The source
baseline is COSMolKit commit `dd608b8cf53f55ad1e14bd2ac695882d94220ecb`.
Viewer-specific changes cover the brand, project links, guide topics,
validation scope, package paths, and deployment configuration.

## Architecture

- `routes.toml` owns routes, navigation order, topic identities, publication
  status, canonical URLs, and legacy aliases.
- Python and JavaScript are API bindings. The language switch matches topics;
  unavailable counterparts lead to the other binding's overview.
- Python guides and API reference come from `../crates/python/docs/source`.
- JavaScript is a non-indexable placeholder for the standalone scene-building
  API. Existing Python notebook WebAssembly rendering is documented separately.
- Rust API guidance is part of the Python guide navigation and links to docs.rs.
- SSG produces readable HTML without the Dioxus client runtime. Only search
  loads its own WASM bundle, on demand, with ranking, debounce, pagination,
  query history, and API anchor links.
- This Cargo package is an independent workspace. Building it never compiles
  the parent renderer or Python extension.

## Build Locally

Run from the repository root:

```sh
uv venv --python 3.11 docs-web/.venv
uv pip install --python docs-web/.venv/bin/python --only-binary=:all: -r docs-web/requirements.txt
docs-web/.venv/bin/python -m playwright install chromium
docs-web/.venv/bin/python -m sphinx -W --keep-going -E -b html crates/python/docs/source crates/python/docs/_build/html
cargo binstall dioxus-cli --version 0.7.10 --no-confirm
rustup target add wasm32-unknown-unknown
docs-web/.venv/bin/python docs-web/scripts/build_search_bundle.py --install-tools
cd docs-web
dx build --release --web --fullstack true --ssg --features ssg --force-sequential
```

On Windows use `docs-web/.venv/Scripts/python.exe` instead of
`docs-web/.venv/bin/python`. Browser scripts accept `--channel msedge`.
The build script finds this environment automatically;
`COSMOL_VIEWER_DOCS_PYTHON` can override it.
`COSMOL_VIEWER_WASM_BINDGEN` can select a matching wasm-bindgen executable.

The documentation dependencies include the published
`cosmol-viewer==0.3.0` wheel. No editable install, generated extension stub,
root `pyproject.toml`, or `uv.lock` is required. The API reference reflects
that installed release; unreleased guide additions must be marked explicitly.

On the tested Windows toolchain, Dioxus's bundled `wasm-opt` reported
`0xc0000409` while optimizing the site client. Dioxus still completed SSG.
Deployment preparation removes that client runtime; the independent search
WASM and final static pages passed browser checks. This does not establish
that the optimized Dioxus client works on that toolchain.

Before rebuilding a prepared site, move its `public` directory aside:
SSG generates directory routes, while preparation flattens them.
Do not merge a fresh build into a previously flattened artifact.

From the repository root, prepare and validate the artifact:

```sh
docs-web/.venv/bin/python docs-web/scripts/prepare_deployment.py docs-web/target/dx/cosmol-viewer-docs-web/release/web/public crates/python/docs/_build/html
docs-web/.venv/bin/python docs-web/scripts/check_ssg_output.py docs-web/target/dx/cosmol-viewer-docs-web/release/web/public
docs-web/.venv/bin/python docs-web/scripts/check_search_browser.py docs-web/target/dx/cosmol-viewer-docs-web/release/web/public
docs-web/.venv/bin/python docs-web/scripts/check_layout_browser.py docs-web/target/dx/cosmol-viewer-docs-web/release/web/public
```

Preparation downloads the social card from `social_image_source` in
`routes.toml`, checks HTTP 200, PNG content type, HTTPS, and 1200 x 630
dimensions, then saves it as `/social-card.png` in the deployment artifact.
Open Graph and Twitter metadata use that site-local URL. Download or validation
failures stop preparation; no local image fallback is used.

The editable source remains `assets/social-card.svg`. To update the hosted
image, run `docs-web/.venv/bin/python docs-web/scripts/render_social_card.py`
(add `--channel msedge` on Windows), then upload `target/social-card.png`
to the configured object-storage URL. CI does not regenerate or upload it.
Browser-check screenshots are saved under `target/screenshots`.

## Development and Checks

After building Sphinx and installing the search tooling, run `dx serve --web`
from `docs-web`. Use `check_search_browser.py --url http://127.0.0.1:8080`
to exercise client navigation as well as the static deployment tests above.

```sh
docs-web/.venv/bin/python -m unittest discover -s docs-web/scripts -p 'test_*.py' -v
cargo test --manifest-path docs-web/Cargo.toml --lib --no-default-features --features search-engine --release --target-dir docs-web/target/search-engine --locked
```

The deployment validator checks metadata, canonical links, language routes,
redirects, sitemap membership, crawler assets, project links, and search assets.
Browser checks exercise real searches, history, load failures, API anchors,
mobile navigation, and desktop/mobile layout.

To preview the prepared artifact with its clean routes and redirects:

```sh
docs-web/.venv/bin/python docs-web/scripts/serve.py docs-web/target/dx/cosmol-viewer-docs-web/release/web/public --port 8088
```

## Deployment

`.github/workflows/docs.yml` builds and checks pull requests without deploying.
Main-branch pushes and manual main-branch runs deploy to the Cloudflare Pages
project `cosmol-viewer-docs-web`, using `CLOUDFLARE_API_TOKEN` and
`CLOUDFLARE_ACCOUNT_ID` secrets. The configured canonical origin is
`https://viewer.cosmol.org/`; the Pages project and custom domain must be
configured before the first deployment.

Clean routes and legacy 301 redirects follow COSMolKit's Cloudflare Pages
contract. GitHub Pages does not interpret `_redirects`; publishing this
artifact there unchanged would not preserve that routing behavior.
Package publishing and GitHub release workflows are independent and unchanged.
