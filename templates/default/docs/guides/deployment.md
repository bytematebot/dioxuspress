---
title: Deployment
order: 2
---

# Deployment

Build a static site:

```sh
dxpress build
```

The output lands in `dist/` and is plain files, so any static host will serve it.

## GitHub Pages

```yaml
name: docs
on:
  push:
    branches: [main]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      - run: cargo install dioxus-cli dioxuspress
      - run: dxpress build
      - uses: actions/upload-pages-artifact@v3
        with:
          path: dist
```

:::info
Serving from a subpath? Set the base path in `Dioxus.toml` so asset URLs resolve.
:::
