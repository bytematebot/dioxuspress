//! The document AST.

/// A value passed to an embedded Dioxus component.
#[derive(Debug, Clone, PartialEq)]
pub enum PropValue {
    /// `foo="bar"`: a plain string literal.
    Str(String),
    /// `foo={expr}`: raw Rust, parsed as a `syn::Expr` at codegen time.
    Expr(String),
    /// `foo`: a bare attribute, lowered to `true`.
    Flag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalloutKind {
    Note,
    Tip,
    Info,
    Warning,
    Danger,
}

impl CalloutKind {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "note" => Self::Note,
            "tip" => Self::Tip,
            "info" => Self::Info,
            "warning" | "caution" => Self::Warning,
            "danger" | "error" => Self::Danger,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Note => "note",
            Self::Tip => "tip",
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Danger => "danger",
        }
    }
}

/// One highlighted run of source code, coloured for both themes at build time.
#[derive(Debug, Clone, PartialEq)]
pub struct HlToken {
    pub text: String,
    /// `#rrggbb` for the light theme.
    pub light: String,
    /// `#rrggbb` for the dark theme.
    pub dark: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Text(String),
    /// A hard or soft line break.
    Break,
    Emphasis(Vec<Node>),
    Strong(Vec<Node>),
    Strikethrough(Vec<Node>),
    InlineCode(String),
    Link {
        href: String,
        title: Option<String>,
        children: Vec<Node>,
    },
    Image {
        src: String,
        alt: String,
    },
    Paragraph(Vec<Node>),
    Heading {
        level: u8,
        id: String,
        children: Vec<Node>,
    },
    CodeBlock {
        lang: Option<String>,
        /// The raw source, kept for the copy button.
        code: String,
        tokens: Vec<HlToken>,
    },
    Blockquote(Vec<Node>),
    List {
        ordered: bool,
        start: u64,
        items: Vec<ListItem>,
    },
    Table {
        head: Vec<Vec<Node>>,
        rows: Vec<Vec<Vec<Node>>>,
    },
    Rule,
    /// An embedded Dioxus component: `<Counter initial={5} />`.
    Component {
        name: String,
        props: Vec<(String, PropValue)>,
        children: Vec<Node>,
    },
}

/// One entry in a list. `task` is `Some` for a GFM task list item.
#[derive(Debug, Clone, PartialEq)]
pub struct ListItem {
    pub task: Option<bool>,
    pub children: Vec<Node>,
}

impl ListItem {
    pub fn new(children: Vec<Node>) -> Self {
        Self {
            task: None,
            children,
        }
    }
}

/// How many task list items a page has, and how many are ticked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Tasks {
    pub done: u32,
    pub total: u32,
}

impl Tasks {
    /// Completion as a whole percentage, and `0` for a page with no tasks.
    pub fn percent(self) -> u8 {
        if self.total == 0 {
            return 0;
        }
        ((self.done as u64 * 100) / self.total as u64) as u8
    }

    /// Walks a node tree, counting task list items at any depth.
    pub fn count(nodes: &[Node]) -> Self {
        let mut tasks = Self::default();
        tasks.add(nodes);
        tasks
    }

    fn add(&mut self, nodes: &[Node]) {
        for node in nodes {
            match node {
                Node::List { items, .. } => {
                    for item in items {
                        if let Some(checked) = item.task {
                            self.total += 1;
                            self.done += u32::from(checked);
                        }
                        self.add(&item.children);
                    }
                }
                Node::Emphasis(children)
                | Node::Strong(children)
                | Node::Strikethrough(children)
                | Node::Paragraph(children)
                | Node::Blockquote(children)
                | Node::Component { children, .. }
                | Node::Heading { children, .. }
                | Node::Link { children, .. } => self.add(children),
                Node::Table { head, rows } => {
                    for cell in head {
                        self.add(cell);
                    }
                    for row in rows {
                        for cell in row {
                            self.add(cell);
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

/// A `##`/`###` heading collected for the on-page table of contents.
#[derive(Debug, Clone, PartialEq)]
pub struct TocEntry {
    pub id: String,
    pub title: String,
    pub level: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Frontmatter {
    pub title: Option<String>,
    pub description: Option<String>,
    /// Explicit sidebar ordering; lower sorts first, untitled pages sort last.
    pub order: Option<i64>,
    /// Hide the page from the sidebar (it stays routable).
    pub hidden: bool,
    /// Whether the page takes part in prev/next. Set `false` on a section index.
    pub pager: bool,
}

impl Default for Frontmatter {
    fn default() -> Self {
        Self {
            title: None,
            description: None,
            order: None,
            hidden: false,
            pager: true,
        }
    }
}

/// A fully parsed page: metadata, body, and the derived TOC and search text.
#[derive(Debug, Clone)]
pub struct Document {
    pub frontmatter: Frontmatter,
    pub nodes: Vec<Node>,
    pub toc: Vec<TocEntry>,
    /// The first `#` heading, used when frontmatter carries no title.
    pub first_heading: Option<String>,
    /// Prose with markup stripped, for the search index.
    pub plain_text: String,
    /// Task list totals for the whole page.
    pub tasks: Tasks,
}

impl Document {
    /// Frontmatter title, else the first `#` heading, else the caller's fallback.
    pub fn title<'a>(&'a self, fallback: &'a str) -> &'a str {
        self.frontmatter
            .title
            .as_deref()
            .or(self.first_heading.as_deref())
            .unwrap_or(fallback)
    }
}
