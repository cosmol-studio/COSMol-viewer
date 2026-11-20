"""Serve documentation article routes as static HTML and keep the live homepage."""

from __future__ import annotations

import re
import sys
from pathlib import Path


HYDRATION_SCRIPT = re.compile(
    r"<script>window\.hydrate_queue=.*?</script>", re.DOTALL
)
HYDRATION_DATA = re.compile(
    r"<script>window\.initial_dioxus_hydration_data=.*?</script>", re.DOTALL
)
CLIENT_PRELOAD = re.compile(
    r'<link rel="preload" as="script" href="[^"]*cosmol-viewer-docs-web[^"]*"[^>]*>'
)
CLIENT_MODULE = re.compile(
    r'<script type="module"[^>]*cosmol-viewer-docs-web[^>]*></script>', re.DOTALL
)


def strip_client_runtime(public: Path) -> int:
    changed = 0
    for page in public.rglob("*.html"):
        if page == public / "index.html" or not page.is_file():
            continue
        source = page.read_text(encoding="utf-8")
        result = HYDRATION_SCRIPT.sub("", source)
        result = HYDRATION_DATA.sub("", result)
        result = CLIENT_PRELOAD.sub("", result)
        result = CLIENT_MODULE.sub("", result)
        if result != source:
            page.write_text(result, encoding="utf-8")
            changed += 1
    return changed


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("usage: strip_client_runtime.py PUBLIC_DIR")
    public_dir = Path(sys.argv[1])
    if not public_dir.is_dir():
        raise SystemExit(f"not a directory: {public_dir}")
    print(f"Removed client runtime from {strip_client_runtime(public_dir)} HTML pages")
