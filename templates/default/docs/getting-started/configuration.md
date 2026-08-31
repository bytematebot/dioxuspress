---
title: Configuration
order: 2
---

# Configuration

All settings live in `dioxus-press.toml`, and every one of them is optional.

```toml
title = "My Docs"
description = "Everything about my project"
repository = "https://github.com/you/my-project"
docs_dir = "docs"
```

## Frontmatter

Per-page settings go at the top of the file:

```markdown
---
title: Signals
description: Reactive state in Dioxus
order: 2
hidden: false
pager: true
---
```

- `title`: overrides the first `#` heading
- `description`: used for the meta tag and search snippets
- `order`: sidebar position; pages without it sort alphabetically after those with it
- `hidden`: keeps the page routable but out of the sidebar
- `pager`: set to `false` on a section index so prev/next steps over it

## Callouts

Container directives become callouts:

```markdown
:::tip Worth knowing
Signals implement `Copy`.
:::
```

:::note
`note`, `tip`, `info`, `warning`, and `danger` are all available.
:::
