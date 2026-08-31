---
title: Introduction
description: A documentation site built with DioxusPress
order: 0
---

# Introduction

Welcome. This site is built with **DioxusPress**. The folder you are editing *is* the
site. Add a `.md` file and it becomes a route, a sidebar entry, and a search result.

:::tip
Every page is a real Dioxus component, so you can drop interactive examples in.
:::

<Counter initial={5} />

## How it works

Markdown is compiled to `rsx!` at build time. That means embedded components keep the
full Rust type system: props are checked by `cargo build`, not at runtime.

```rust
#[component]
pub fn Counter(initial: i32) -> Element {
    let mut value = use_signal(|| initial);

    rsx! {
        button {
            onclick: move |_| value += 1,
            "{value}"
        }
    }
}
```

## Where to go next

- [Installation](/getting-started/installation): set up the toolchain
- [Configuration](/getting-started/configuration): title, description, docs directory
- [Deployment](/guides/deployment): ship a static build
