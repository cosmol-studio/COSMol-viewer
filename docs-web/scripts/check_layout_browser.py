"""Check the prepared documentation at desktop and mobile viewport sizes."""

import argparse
from functools import partial
from http.server import ThreadingHTTPServer
from pathlib import Path
from threading import Thread

from playwright.sync_api import sync_playwright

from check_search_browser import StaticPagesHandler
from route_contract import PAGES


def check_layout(public, channel=None):
    output = Path(__file__).resolve().parents[1] / "target/screenshots"
    output.mkdir(parents=True, exist_ok=True)
    server = ThreadingHTTPServer(
        ("127.0.0.1", 0),
        partial(StaticPagesHandler, directory=str(public.resolve())),
    )
    thread = Thread(target=server.serve_forever, daemon=True)
    thread.start()
    base = f"http://127.0.0.1:{server.server_port}"
    try:
        with sync_playwright() as playwright:
            browser = playwright.chromium.launch(channel=channel)
            try:
                for width, height in ((1440, 1000), (390, 844), (320, 740)):
                    page = browser.new_page(viewport={"width": width, "height": height})
                    errors = []
                    page.on("pageerror", lambda error: errors.append(str(error)))
                    for route in PAGES:
                        response = page.goto(base + route["path"])
                        assert response.status == 200, route["path"]
                        page.evaluate("document.fonts.ready")
                        page.wait_for_function("""() => [...document.images].every(
                            image => image.complete && image.naturalWidth > 0
                        )""")
                        assert page.locator("main").count() == 1
                        assert page.locator("h1").count() == 1
                        dimensions = page.evaluate("""() => ({
                            viewport: innerWidth,
                            content: document.documentElement.scrollWidth,
                            header: document.querySelector('header').getBoundingClientRect().bottom,
                            heading: document.querySelector('h1').getBoundingClientRect().top
                        })""")
                        assert dimensions["content"] <= width + 1, (route["path"], dimensions)
                        assert dimensions["heading"] >= dimensions["header"], (route["path"], dimensions)
                        overlap = page.evaluate("""() => {
                            const links = [...document.querySelectorAll('header a')]
                                .map(a => ({text:a.textContent.trim(), box:a.getBoundingClientRect()}))
                                .filter(a => a.box.width && a.box.height);
                            for (let i=0; i<links.length; i++)
                                for (let j=i+1; j<links.length; j++) {
                                    const a=links[i].box, b=links[j].box;
                                    if (Math.min(a.right,b.right)-Math.max(a.left,b.left)>1
                                        && Math.min(a.bottom,b.bottom)-Math.max(a.top,b.top)>1)
                                        return [links[i].text, links[j].text];
                                }
                            return null;
                        }""")
                        assert overlap is None, (route["path"], width, overlap)
                        if width <= 390 and route.get("source") == "sphinx":
                            menu = page.locator(".docs-mobile-navigation")
                            assert menu.is_visible()
                            menu.locator("summary").click()
                            assert menu.locator(".docs-nav-link.is-active").is_visible()
                            menu.locator("summary").click()
                        if route["path"] in ("/", "/python", "/python/api", "/python/proteins", "/javascript"):
                            name = route["path"].strip("/").replace("/", "-") or "home"
                            page.screenshot(path=str(output / f"{name}-{width}.png"))
                        assert not errors, (route["path"], errors)
                    page.close()
                    print(f"PASS: {len(PAGES)} routes at {width} x {height}")
            finally:
                browser.close()
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("public", type=Path)
    parser.add_argument("--channel")
    args = parser.parse_args()
    check_layout(args.public, args.channel)
