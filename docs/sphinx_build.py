"""Run Sphinx, including sphinx-js 5's Windows process compatibility fixes.

Arguments are identical to ``python -m sphinx``. Linux uses Sphinx unchanged.
The compatibility layer only handles process paths and temporary-file sharing;
it does not modify declarations, TypeDoc output, or generated documentation.
"""

from contextlib import ExitStack
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from unittest.mock import patch

from sphinx.cmd.build import build_main


def main(arguments=None):
    temporary_paths = []
    original_run = subprocess.run
    original_temporary_file = tempfile.NamedTemporaryFile

    def run(args, *positional, **options):
        args = list(args) if isinstance(args, (list, tuple)) else args
        if isinstance(args, list) and str(args[0]).replace("\\", "/").endswith("typedoc/bin/typedoc"):
            args.insert(0, "node")
        if isinstance(args, list) and str(args[0]).lower().endswith("npx.cmd"):
            args = [Path(value).as_posix() if isinstance(value, str) and len(value) > 2
                    and value[1] == ":" else value for value in args]
            for index, arg in enumerate(args[:-1]):
                if arg == "--import" and Path(args[index + 1]).is_absolute():
                    args[index + 1] = Path(args[index + 1]).as_uri()
            if "--import" in args:
                index = args.index("--import")
                args[index:index] = ["--import", Path(__file__).with_name("node_windows_loader.mjs").as_uri()]
            if "env" in options:
                options["env"] = dict(options["env"])
                if "TYPEDOC_NODE_MODULES" in options["env"]:
                    options["env"]["TYPEDOC_NODE_MODULES"] = Path(options["env"]["TYPEDOC_NODE_MODULES"]).as_posix()
        return original_run(args, *positional, **options)

    def temporary_file(*args, **kwargs):
        kwargs["delete"] = False
        result = original_temporary_file(*args, **kwargs)
        temporary_paths.append(Path(result.name))
        return result

    try:
        with ExitStack() as stack:
            if os.name == "nt":
                import sphinx_js.typedoc
                stack.enter_context(patch.object(subprocess, "run", run))
                stack.enter_context(patch.object(sphinx_js.typedoc, "NamedTemporaryFile", temporary_file))
            return build_main(sys.argv[1:] if arguments is None else arguments)
    finally:
        for path in temporary_paths:
            path.unlink(missing_ok=True)


if __name__ == "__main__":
    raise SystemExit(main())
