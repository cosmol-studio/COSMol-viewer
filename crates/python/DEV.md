## Generate `.pyi` Stubs (dev / abi3-py310)

From repo root:

```bash
cargo run -p cosmol_viewer_python --no-default-features --features dev-stub --bin stub_gen
```

Generated file:


```text
./crates/python/cosmol_viewer.pyi
```

## Build/Install Extension with maturin

### Dev install (editable)

From repo root:

```bash
maturin develop --uv --manifest-path crates/python/Cargo.toml
```
