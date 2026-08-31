# Dioxus Press

Write your project's documentation in markdown and get a [Dioxus](https://dioxuslabs.com)
app out. Markdown is compiled to `rsx!` at build time, so every page is an ordinary
Dioxus component.

```sh
cargo install dioxus-cli dioxus-press
dxpress new my-docs
cd my-docs
dxpress dev
```

## Pages

```markdown
---
title: Rate limits
description: How many requests you get, and what happens past that
---

# Rate limits

:::tip
Limits are per API key, not per IP.
:::

<Counter initial={5} on_change={move |v| tracing::info!("{v}")} />
```

`initial` and `on_change` are checked by `cargo build`. Closures, signals, and
non-serializable types all work, because nothing crosses a string boundary.

## Routing

```
docs/
├── index.md                     ->  /
├── getting-started/
│   ├── index.md                 ->  /getting-started
│   └── installation.md          ->  /getting-started/installation
└── guides/
    └── deployment.md            ->  /guides/deployment
```

Sidebar, breadcrumbs, prev/next, and the table of contents follow from that structure.
`order` in a page's frontmatter overrides sidebar position.

## Commands

| Command            | What it does                                              |
| ------------------ | --------------------------------------------------------- |
| `dxpress new`      | Scaffold a site                                            |
| `dxpress dev`      | Dev server with hot reloading (wraps `dx serve`)           |
| `dxpress build`    | Production build into `dist/` (`--ssg` to pre-render, `--base-path` to serve from a subdirectory) |
| `dxpress generate` | Rewrite `generated/docs.rs` without building               |
| `dxpress check`    | Parse docs, validate internal links, print the site tree   |

`check` needs no Rust build.

## Project layout

```
dioxus-press.toml     config
docs/                your content
languages/*.toml     optional syntax definitions for highlighting
src/                 main.rs, components.rs, landing.rs, theme/
tailwind.css         the design system, compiled to assets/tailwind.css
generated/           dxpress writes this
```

`dxpress new` writes the whole theme into `src/theme/`, as ordinary Dioxus components.
Edit or replace any of it. The contract is `dioxus_press::types`: `SidebarItem`,
`PageMeta`, `TocItem`, `Tasks`, `Token`, `SiteConfig`.

`generated/` is git-ignored and rewritten on every run. It stays in the crate root
because `dx` ignores dotted directories and gives them no hot reload.

## Task lists

`- [ ]` and `- [x]` items are counted per page at build time into `PageMeta::tasks`:

```markdown
- [x] Query API
- [ ] Migrations
```

The default theme draws a progress card in the header (`PROGRESS_PAGE` in
`src/theme/mod.rs`) and `<PageProgress pages={pages()} />` in the page. Both render
nothing on a page with no task list.

## Development

```sh
cargo test
cargo run --bin dxpress -- new /tmp/demo --local .
```

`--local` points the generated site at this checkout. The template lives in
`templates/default/`, inside the crate so `cargo package` ships it.
