"""Preview clean documentation routes and redirects on localhost."""

import argparse
from functools import partial
from http.server import ThreadingHTTPServer
from pathlib import Path

from check_search_browser import StaticPagesHandler


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("public", type=Path)
    parser.add_argument("--port", type=int, default=8088)
    args = parser.parse_args()
    if not (args.public / "_redirects").is_file():
        parser.error("run prepare_deployment.py before serving the artifact")
    server = ThreadingHTTPServer(
        ("127.0.0.1", args.port),
        partial(StaticPagesHandler, directory=str(args.public.resolve())),
    )
    print(f"Documentation preview: http://127.0.0.1:{args.port}", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
