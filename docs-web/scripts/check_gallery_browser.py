"""Check the docking and conformer workflows in the built Dioxus homepage."""

import argparse
from base64 import b64decode, b64encode
from functools import partial
from http.server import ThreadingHTTPServer
from pathlib import Path
from threading import Thread
from urllib.parse import urlsplit

from playwright.sync_api import expect, sync_playwright

from check_search_browser import StaticPagesHandler


def check_hero(page):
    canvas = page.locator("#hero-canvas")
    expect(canvas).to_have_attribute("data-live", "true", timeout=60_000)
    page.wait_for_timeout(350)
    first = canvas.screenshot()
    page.wait_for_timeout(450)
    assert canvas.screenshot() != first, "The hero structure should rotate automatically"

    bounds = canvas.bounding_box()
    x, y = bounds["x"] + bounds["width"] / 2, bounds["y"] + bounds["height"] / 2
    page.mouse.move(x, y)
    page.mouse.down()
    page.mouse.move(x + 15, y + 10, steps=5)
    page.wait_for_timeout(200)
    first = canvas.screenshot()
    page.mouse.move(x + 95, y + 45, steps=8)
    page.wait_for_timeout(200)
    assert canvas.screenshot() != first, "Dragging should rotate the hero structure"
    page.mouse.up()


def receptor_bounds(page, png):
    return page.evaluate("""async (png) => {
        const image = new Image();
        image.src = `data:image/png;base64,${png}`;
        await image.decode();
        const canvas = document.createElement('canvas');
        const width = canvas.width = image.width;
        const height = canvas.height = image.height;
        const context = canvas.getContext('2d');
        context.drawImage(image, 0, 0);
        const { data } = context.getImageData(0, 0, width, height);
        let left = width, right = 0, top = height, bottom = 0;
        for (let y = 0; y < height; y++) {
            for (let x = 0; x < width; x++) {
                const i = 4 * (y * width + x);
                // Ignore the dark background and look for the blue receptor ribbon.
                if (x < 220 && y > height - 46) continue; // Legend overlay.
                if (data[i + 2] < 115 || data[i + 2] - data[i] < 25) continue;
                left = Math.min(left, x); right = Math.max(right, x);
                top = Math.min(top, y); bottom = Math.max(bottom, y);
            }
        }
        return [left / width, right / width, top / height, bottom / height];
    }""", b64encode(png).decode("ascii"))


def check_docking(page):
    canvas = page.locator("#docking-canvas")
    canvas.scroll_into_view_if_needed()
    expect(canvas).to_have_attribute("data-live", "true", timeout=60_000)
    expect(page.locator("#docking-pose")).to_be_enabled()
    page.locator("#docking-rotate").click()
    page.wait_for_timeout(300)
    first = canvas.screenshot()
    page.locator("#docking-pose").select_option("3")
    page.wait_for_timeout(300)
    assert canvas.screenshot() != first, "Selecting a pose should change the rendered structure"
    page.locator("#docking-reference").uncheck()
    expect(page.locator("#crystal-legend")).to_be_hidden()
    page.locator("#docking-reference").check()
    expect(page.locator("#crystal-legend")).to_be_visible()
    view = page.locator("#docking-view")
    view.click()
    expect(view).to_have_text("Binding site")
    page.wait_for_timeout(300)
    fitted = canvas.screenshot()
    box = receptor_bounds(page, fitted)
    left, right, top, bottom = box
    assert .05 < left < right < .95 and .05 < top < bottom < .95, box
    assert right - left > .25 and bottom - top > .25, box
    assert .38 < (left + right) / 2 < .62 and .38 < (top + bottom) / 2 < .62, box
    view.click()
    bounds = canvas.bounding_box()
    page.mouse.move(bounds["x"] + bounds["width"] / 2, bounds["y"] + bounds["height"] / 2)
    page.mouse.wheel(0, -850)
    page.wait_for_timeout(350)
    view.click()
    page.wait_for_timeout(350)
    refitted = receptor_bounds(page, canvas.screenshot())
    assert max(abs(before - after) for before, after in zip(box, refitted)) < .06, (box, refitted)
    view.click()


def capture_canvas_frame(canvas):
    # Read the next painted canvas frame after the frame number updates.
    return b64decode(canvas.evaluate("""async (canvas) => {
        const stream = canvas.captureStream(0);
        const track = stream.getVideoTracks()[0];
        const video = document.createElement("video");
        video.muted = true;
        video.srcObject = stream;
        let timeout;
        try {
            const painted = new Promise((resolve, reject) => {
                video.requestVideoFrameCallback(resolve);
                timeout = setTimeout(() => reject(new Error("No rendered canvas frame received")), 10000);
            });
            track.requestFrame();
            await Promise.all([video.play(), painted]);
            const frame = document.createElement("canvas");
            frame.width = video.videoWidth;
            frame.height = video.videoHeight;
            frame.getContext("2d").drawImage(video, 0, 0);
            return frame.toDataURL("image/png").split(",")[1];
        } finally {
            clearTimeout(timeout);
            track.stop();
            video.srcObject = null;
        }
    }"""))


def check_conformers(page, screenshots):
    canvas = page.locator("#conformer-canvas")
    canvas.scroll_into_view_if_needed()
    expect(canvas).to_have_attribute("data-live", "true", timeout=60_000)
    readout = page.locator("#conformer-readout")
    play = page.locator("#conformer-play")
    smooth = page.locator("#conformer-smooth")
    expect(smooth).to_be_checked()
    expect(readout).not_to_have_text("Frame 1 / 50")
    play.click()
    expect(play).to_have_text("Play")
    paused = readout.inner_text()
    page.wait_for_timeout(300)
    expect(readout).to_have_text(paused)

    slider = page.locator("#conformer-frame")
    slider.fill("1")
    expect(readout).to_have_text("Frame 1 / 50")
    initial = capture_canvas_frame(canvas)
    smooth.uncheck()
    toggled = capture_canvas_frame(canvas)
    if toggled != initial:
        before = screenshots / "conformer-smooth-before.png"
        after = screenshots / "conformer-smooth-after.png"
        before.write_bytes(initial)
        after.write_bytes(toggled)
        raise AssertionError(f"Turning smoothing off changed a paused source frame: {before}, {after}")
    slider.fill("50")
    expect(readout).to_have_text("Frame 50 / 50")
    assert capture_canvas_frame(canvas) != initial, "The last source frame should change the geometry"

    page.locator("#conformer-series").select_option("2")
    expect(readout).to_have_text("Frame 1 / 50")
    assert capture_canvas_frame(canvas) != initial, "A different sequence should display different coordinates"
    play.click()
    expect(readout).not_to_have_text("Frame 1 / 50")
    smooth.check()
    expect(play).to_have_text("Pause")


def check_interface(page):
    canvas = page.locator("#interface-canvas")
    canvas.scroll_into_view_if_needed()
    expect(canvas).to_have_attribute("data-live", "true", timeout=60_000)
    focus = page.locator("#interface-focus")
    expect(focus).to_be_enabled()
    page.wait_for_timeout(300)
    whole = canvas.screenshot()
    surface = page.locator("#interface-surface")
    expect(surface).to_be_checked()
    surface.uncheck()
    page.wait_for_timeout(300)
    assert whole != canvas.screenshot(), "The molecular surface should change the rendered view"
    surface.check()
    focus.click()
    expect(focus).to_have_text("Whole complex")
    page.wait_for_timeout(300)
    assert whole != canvas.screenshot(), "Focusing should frame and draw the contact region"
    focus.click()
    expect(focus).to_have_text("Focus interface")


def check_alignment(page):
    canvas = page.locator("#alignment-canvas")
    canvas.scroll_into_view_if_needed()
    expect(canvas).to_have_attribute("data-live", "true", timeout=60_000)
    mode = page.locator("#alignment-mode")
    expect(mode).to_be_enabled()
    page.wait_for_timeout(300)
    overlay = canvas.screenshot()
    mode.select_option("1")
    expect(page.locator("#alignment-closed-legend")).to_be_hidden()
    page.wait_for_timeout(300)
    open_view = canvas.screenshot()
    mode.select_option("2")
    expect(page.locator("#alignment-open-legend")).to_be_hidden()
    page.wait_for_timeout(300)
    assert len({overlay, open_view, canvas.screenshot()}) == 3
    mode.select_option("0")


def check_fullscreen(page, name):
    canvas = page.locator(f"#{name}-canvas")
    button = page.locator(f"#{name}-fullscreen")
    initial_size = canvas.evaluate("canvas => [canvas.clientWidth, canvas.clientHeight]")
    initial_label = button.get_attribute("aria-label")
    button.click()
    page.wait_for_function("id => document.fullscreenElement?.id === id", arg=f"{name}-panel")
    expect(button).to_have_text("Exit fullscreen")
    button.click()
    page.wait_for_function("document.fullscreenElement === null")
    expect(button).to_have_attribute("aria-label", initial_label)
    expect(canvas).to_have_js_property("clientWidth", initial_size[0])
    expect(canvas).to_have_js_property("clientHeight", initial_size[1])


def check(public, channel):
    server = ThreadingHTTPServer(("127.0.0.1", 0), partial(StaticPagesHandler, directory=str(public)))
    Thread(target=server.serve_forever, daemon=True).start()
    origin = f"http://127.0.0.1:{server.server_port}"
    screenshots = public.parent / "gallery-check"
    screenshots.mkdir(exist_ok=True)
    try:
        with sync_playwright() as pw:
            browser = pw.chromium.launch(channel=channel, args=["--use-angle=swiftshader", "--enable-unsafe-swiftshader"])
            try:
                page = browser.new_page(viewport={"width": 1440, "height": 1000})
                errors, requests = [], []
                page.on("pageerror", lambda error: errors.append(str(error)))
                page.on("request", lambda request: requests.append(request.url))
                page.goto(origin)
                expect(page.get_by_role("heading", level=1)).to_have_text("COSMol Viewer")
                intro = page.get_by_role("region", name="Documentation overview")
                expect(intro.locator("canvas")).to_have_count(1)
                expect(page.locator("#applications article canvas")).to_have_count(4)
                expect(intro.get_by_role("link", name="Python quick start")).to_have_attribute("href", "/python/quickstart")
                expect(intro.get_by_role("link", name="JavaScript quick start")).to_have_attribute("href", "/javascript/quickstart")
                expect(page.get_by_role("heading", name="Guides and API reference")).to_be_visible()
                expect(page.get_by_role("heading", name="COSMol ecosystem")).to_be_visible()
                check_hero(page)
                page.screenshot(path=screenshots / "hero.png")
                check_docking(page)
                page.locator("#docking-demo").screenshot(path=screenshots / "docking.png")
                check_conformers(page, screenshots)
                page.locator("#conformer-panel").screenshot(path=screenshots / "conformers.png")
                check_interface(page)
                page.locator("#antibody-interface").screenshot(path=screenshots / "interface.png")
                check_alignment(page)
                page.locator("#structure-comparison").screenshot(path=screenshots / "alignment.png")
                for name in ("docking", "conformer", "interface", "alignment"):
                    check_fullscreen(page, name)
                assert not errors, errors
                assets = [urlsplit(url).path for url in requests]
                assert any("cosmol-viewer-docs-web" in path and path.endswith(".wasm") for path in assets)
                assert not any(path.startswith("/gallery/") for path in assets)
                intro.get_by_role("link", name="Python quick start").click()
                expect(page).to_have_url(f"{origin}/python/quickstart")
                expect(page.locator("#hero-canvas")).to_have_count(0)
                page.go_back()
                expect(page.locator("#hero-canvas")).to_have_attribute("data-live", "true", timeout=60_000)
                assert not errors, errors
                page.close()

                mobile = browser.new_page(viewport={"width": 390, "height": 844}, is_mobile=True, has_touch=True)
                mobile.goto(origin)
                expect(mobile.locator("#hero-canvas")).to_have_attribute("data-live", "true", timeout=60_000)
                assert mobile.locator("#docking-canvas").get_attribute("data-live") is None
                assert mobile.evaluate("document.documentElement.scrollWidth <= innerWidth")
                assert mobile.evaluate("scrollY === 0")
                mobile.screenshot(path=screenshots / "mobile.png")
                mobile.locator("#conformer-panel").scroll_into_view_if_needed()
                expect(mobile.locator("#conformer-play")).to_be_enabled()
                check_fullscreen(mobile, "conformer")
                mobile.close()

                failed = browser.new_page()
                failed.add_init_script("""
                    const getContext = HTMLCanvasElement.prototype.getContext;
                    HTMLCanvasElement.prototype.getContext = function (kind, ...options) {
                        if (kind === 'webgl' || kind === 'webgl2') return null;
                        return getContext.call(this, kind, ...options);
                    };
                """)
                failed.goto(origin)
                expect(failed.locator("#hero-loading")).to_contain_text("Could not display", timeout=60_000)
                failed.locator("#conformer-canvas").scroll_into_view_if_needed()
                expect(failed.locator("#conformer-loading")).to_contain_text("Could not display", timeout=60_000)
                print(f"Hero, four live applications, mobile and load-error checks passed. Screenshots: {screenshots}")
            finally:
                browser.close()
    finally:
        server.shutdown()
        server.server_close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("public", type=Path)
    parser.add_argument("--channel", default=None)
    args = parser.parse_args()
    check(args.public.resolve(), args.channel)
