---
title: Installation
order: 1
---

# Installation

DioxusPress builds on the Dioxus CLI. Install both:

```sh
cargo install dioxus-cli
cargo install dioxuspress
```

Then start the dev server:

```sh
dxpress dev
```

:::warning
The WebAssembly target is required. Add it once with
`rustup target add wasm32-unknown-unknown`.
:::

## Project layout

| Path                | Purpose                                  |
| ------------------- | ---------------------------------------- |
| `docs/`             | Your markdown. Structure becomes routing. |
| `src/components.rs` | Components you embed in markdown.         |
| `dioxuspress.toml`  | Site title, description, docs directory.  |
