"""Regression checks for preserving Rust async and borrowing in Alef output."""

import unittest

from compat import preserve_source_semantics


def api_method(name, *, static=False, asynchronous=False, params=()):
    return {"name": name, "is_static": static, "is_async": asynchronous,
            "params": list(params)}


def parameter(name, rust_type, *, borrowed=True):
    return {"name": name, "ty": {"Named": rust_type}, "is_ref": borrowed,
            "is_mut": False, "optional": False}


def api(*methods):
    return {"types": [
        {"name": "Scene", "is_opaque": True, "methods": []},
        {"name": "Viewer", "is_opaque": True, "methods": list(methods)},
    ]}


class SourceSemanticsTests(unittest.TestCase):
    def test_static_async_and_borrow_are_preserved_together(self):
        generated = '''impl Scene {}
impl Viewer {
    pub fn render(scene: Scene) -> Result<Viewer, JsValue> {
        core::Viewer::render(&scene.inner).map_err(error).map(Viewer::from)
    }
}'''
        metadata = api(api_method("render", static=True, asynchronous=True,
                                  params=[parameter("scene", "Scene")]))
        corrected = preserve_source_semantics(generated, metadata)
        self.assertIn("pub async fn render(scene: &Scene)", corrected)
        self.assertIn("render(&scene.inner).await.map_err", corrected)
        self.assertEqual(preserve_source_semantics(corrected, metadata), corrected)

    def test_only_borrowed_opaque_arguments_change(self):
        generated = '''impl Scene {}
impl Viewer {
    pub fn update(&self, scene: Scene, owned: Scene, scale: f32) -> Result<(), JsValue> {
        self.inner.update(&scene.inner, owned.inner, scale)
    }
}'''
        metadata = api(api_method("update", params=[parameter("scene", "Scene"),
                           parameter("owned", "Scene", borrowed=False),
                           parameter("scale", "f32")]))
        corrected = preserve_source_semantics(generated, metadata)
        self.assertIn("scene: &Scene, owned: Scene, scale: f32", corrected)
        self.assertNotIn("async", corrected)
        self.assertNotIn(".await", corrected)

    def test_unexpected_delegation_fails_instead_of_silently_changing_semantics(self):
        generated = '''impl Scene {}
impl Viewer {
    pub fn render() -> Result<Viewer, JsValue> { core::Viewer::render() }
}'''
        with self.assertRaisesRegex(ValueError, "async delegation"):
            preserve_source_semantics(generated, api(api_method(
                "render", static=True, asynchronous=True)))

    def test_missing_method_or_changed_argument_fails(self):
        metadata = api(api_method("update", params=[parameter("scene", "Scene")]))
        with self.assertRaisesRegex(ValueError, "Missing generated method"):
            preserve_source_semantics("impl Scene {}\nimpl Viewer {}", metadata)
        with self.assertRaisesRegex(ValueError, "Unexpected opaque argument"):
            preserve_source_semantics('''impl Scene {}
impl Viewer {
    pub fn update(&self, scene: String) -> Result<(), JsValue> { Ok(()) }
}''', metadata)

    def test_comments_and_strings_do_not_end_the_method_block(self):
        generated = '''impl Scene {}
impl Viewer {
    pub fn update(&self, scene: Scene) -> Result<(), JsValue> {
        // }
        /* } */
        println!("}");
        self.inner.update(&scene.inner)
    }
}'''
        corrected = preserve_source_semantics(generated, api(api_method(
            "update", params=[parameter("scene", "Scene")])))
        self.assertIn("scene: &Scene", corrected)
        self.assertIn("self.inner.update(&scene.inner)", corrected)


if __name__ == "__main__":
    unittest.main()
