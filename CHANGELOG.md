# Changelog

## 0.1.0 (unreleased)

First release.

- `dxpress new`, `dev`, `build`, `generate`, `check`
- Markdown compiled to `rsx!` at build time, with embedded Dioxus components keeping
  typed props, closures, and signals
- File-based routing, sidebar, breadcrumbs, prev/next, and table of contents
- Container directives (`:::tip`, `:::warning`, …) as callouts
- Build-time syntax highlighting for light and dark, plus project-defined languages
  declared as data in `languages/*.toml`
- One crate: `types` with no dependencies by default, `core`, `build` and `cli` behind
  features
- Task lists counted at generation time into `PageMeta::tasks`, which the default theme
  renders as a header progress card and an in-page `<PageProgress />`
- Client-side search over a build-time index
- Dark and light mode, applied before the first paint
- Static output, optionally pre-rendered with `dxpress build --ssg`
