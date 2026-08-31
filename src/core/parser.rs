//! Markdown -> [`Document`].

use crate::core::ast::*;
use crate::core::lang::Language;
use crate::core::slug::SlugAllocator;
use crate::core::{directive, frontmatter, highlight, tag};
use anyhow::Result;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

/// Parses one markdown source with no project-defined languages.
pub fn parse(source: &str) -> Result<Document> {
    parse_with(source, &[])
}

/// Parses one markdown source, expanding directives, highlighting code, and collecting
/// the TOC and search text in the same pass. `languages` are consulted before syntect.
pub fn parse_with(source: &str, languages: &[Language]) -> Result<Document> {
    let (frontmatter, body) = frontmatter::split(source)?;
    let expanded = directive::expand(body);

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);

    let mut builder = Builder::new(languages);
    for event in Parser::new_ext(&expanded, options) {
        builder.event(event);
    }
    let nodes = builder.finish();

    let plain_text = plain_text(&nodes);
    let tasks = Tasks::count(&nodes);
    Ok(Document {
        frontmatter,
        toc: builder.toc,
        first_heading: builder.first_heading,
        plain_text,
        tasks,
        nodes,
    })
}

#[derive(Debug)]
enum Kind {
    Root,
    Paragraph,
    Heading {
        level: u8,
        id: Option<String>,
    },
    Emphasis,
    Strong,
    Strikethrough,
    Link {
        href: String,
        title: Option<String>,
    },
    Blockquote,
    List {
        ordered: bool,
        start: u64,
        items: Vec<ListItem>,
    },
    Item {
        task: Option<bool>,
    },
    Table {
        head: Vec<Vec<Node>>,
        rows: Vec<Vec<Vec<Node>>>,
        row: Vec<Vec<Node>>,
        in_head: bool,
    },
    TableCell,
    CodeBlock {
        lang: Option<String>,
        code: String,
    },
    Component {
        name: String,
        props: Vec<(String, PropValue)>,
    },
    /// A container we do not model; its children are spliced into the parent.
    Transparent,
}

struct Frame {
    kind: Kind,
    children: Vec<Node>,
}

struct Builder<'a> {
    stack: Vec<Frame>,
    slugs: SlugAllocator,
    toc: Vec<TocEntry>,
    first_heading: Option<String>,
    languages: &'a [Language],
}

impl<'a> Builder<'a> {
    fn new(languages: &'a [Language]) -> Self {
        Self {
            stack: vec![Frame {
                kind: Kind::Root,
                children: Vec::new(),
            }],
            slugs: SlugAllocator::default(),
            toc: Vec::new(),
            first_heading: None,
            languages,
        }
    }
}

impl Builder<'_> {
    fn finish(&mut self) -> Vec<Node> {
        while self.stack.len() > 1 {
            self.pop();
        }
        std::mem::take(&mut self.stack[0].children)
    }

    fn push(&mut self, kind: Kind) {
        self.stack.push(Frame {
            kind,
            children: Vec::new(),
        });
    }

    fn emit(&mut self, node: Node) {
        self.stack
            .last_mut()
            .expect("root frame is never popped")
            .children
            .push(node);
    }

    fn event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => {
                if let Some(Frame {
                    kind: Kind::CodeBlock { code, .. },
                    ..
                }) = self.stack.last_mut()
                {
                    code.push_str(&text);
                } else {
                    self.emit(Node::Text(text.to_string()));
                }
            }
            Event::Code(text) => self.emit(Node::InlineCode(text.to_string())),
            Event::SoftBreak => self.emit(Node::Text(" ".to_string())),
            Event::HardBreak => self.emit(Node::Break),
            Event::Rule => self.emit(Node::Rule),
            Event::Html(html) | Event::InlineHtml(html) => self.html(&html),
            Event::TaskListMarker(checked) => {
                if let Some(Frame {
                    kind: Kind::Item { task },
                    ..
                }) = self.stack.last_mut()
                {
                    *task = Some(checked);
                }
            }
            _ => {}
        }
    }

    fn html(&mut self, html: &str) {
        for chunk in html.split_inclusive('>') {
            let chunk = chunk.trim();
            if chunk.is_empty() {
                continue;
            }
            match tag::parse(chunk) {
                Some(tag::Tag::SelfClosing { name, props }) => self.emit(Node::Component {
                    name,
                    props,
                    children: Vec::new(),
                }),
                Some(tag::Tag::Open { name, props }) => self.push(Kind::Component { name, props }),
                Some(tag::Tag::Close { name }) => self.close_component(&name),
                None => {}
            }
        }
    }

    /// Unwinds to the matching open component, closing any frames left dangling inside.
    fn close_component(&mut self, name: &str) {
        let target = self.stack.iter().rposition(
            |frame| matches!(&frame.kind, Kind::Component { name: open, .. } if open == name),
        );
        let Some(target) = target else { return };
        while self.stack.len() > target {
            self.pop();
        }
    }

    fn start(&mut self, tag: Tag) {
        match tag {
            Tag::Paragraph => self.push(Kind::Paragraph),
            Tag::Heading { level, id, .. } => self.push(Kind::Heading {
                level: level as u8,
                id: id.map(|id| id.to_string()),
            }),
            Tag::Emphasis => self.push(Kind::Emphasis),
            Tag::Strong => self.push(Kind::Strong),
            Tag::Strikethrough => self.push(Kind::Strikethrough),
            Tag::BlockQuote(_) => self.push(Kind::Blockquote),
            Tag::Link {
                dest_url, title, ..
            } => self.push(Kind::Link {
                href: dest_url.to_string(),
                title: Some(title.to_string()).filter(|t| !t.is_empty()),
            }),
            Tag::Image {
                dest_url, title, ..
            } => self.push(Kind::Link {
                href: dest_url.to_string(),
                title: Some(format!("\0image\0{title}")),
            }),
            Tag::List(start) => self.push(Kind::List {
                ordered: start.is_some(),
                start: start.unwrap_or(1),
                items: Vec::new(),
            }),
            Tag::Item => self.push(Kind::Item { task: None }),
            Tag::Table(_) => self.push(Kind::Table {
                head: Vec::new(),
                rows: Vec::new(),
                row: Vec::new(),
                in_head: false,
            }),
            Tag::TableHead => {
                if let Some(Frame {
                    kind: Kind::Table { in_head, .. },
                    ..
                }) = self.stack.last_mut()
                {
                    *in_head = true;
                }
            }
            Tag::TableRow => {}
            Tag::TableCell => self.push(Kind::TableCell),
            Tag::CodeBlock(kind) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(lang) => Some(lang.to_string()).filter(|l| !l.is_empty()),
                    CodeBlockKind::Indented => None,
                };
                self.push(Kind::CodeBlock {
                    lang,
                    code: String::new(),
                });
            }
            Tag::HtmlBlock => {}
            _ => self.push(Kind::Transparent),
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::HtmlBlock => {}
            TagEnd::TableHead => {
                if let Some(Frame {
                    kind:
                        Kind::Table {
                            in_head, row, head, ..
                        },
                    ..
                }) = self.stack.last_mut()
                {
                    *head = std::mem::take(row);
                    *in_head = false;
                }
            }
            TagEnd::TableRow => {
                if let Some(Frame {
                    kind:
                        Kind::Table {
                            rows, row, in_head, ..
                        },
                    ..
                }) = self.stack.last_mut()
                {
                    if !*in_head {
                        let finished = std::mem::take(row);
                        rows.push(finished);
                    }
                }
            }
            _ => self.pop(),
        }
    }

    fn pop(&mut self) {
        if self.stack.len() <= 1 {
            return;
        }
        let frame = self.stack.pop().expect("checked length above");
        let children = frame.children;

        let node = match frame.kind {
            Kind::Root => return,
            Kind::Transparent => {
                self.stack
                    .last_mut()
                    .expect("root frame is never popped")
                    .children
                    .extend(children);
                return;
            }
            Kind::Paragraph => Node::Paragraph(children),
            Kind::Emphasis => Node::Emphasis(children),
            Kind::Strong => Node::Strong(children),
            Kind::Strikethrough => Node::Strikethrough(children),
            Kind::Blockquote => Node::Blockquote(children),
            Kind::Heading { level, id } => {
                let title = plain_text(&children);
                let id = id.unwrap_or_else(|| self.slugs.allocate(&title));
                if level == 1 && self.first_heading.is_none() {
                    self.first_heading = Some(title.clone());
                }
                if (2..=3).contains(&level) {
                    self.toc.push(TocEntry {
                        id: id.clone(),
                        title,
                        level,
                    });
                }
                Node::Heading {
                    level,
                    id,
                    children,
                }
            }
            Kind::Link { href, title } => match title.as_deref().and_then(image_title) {
                Some(_) => Node::Image {
                    src: href,
                    alt: plain_text(&children),
                },
                None => Node::Link {
                    href,
                    title,
                    children,
                },
            },
            Kind::CodeBlock { lang, code } => {
                let tokens = highlight::highlight(&code, lang.as_deref(), self.languages);
                Node::CodeBlock { lang, code, tokens }
            }
            Kind::Component { name, props } => Node::Component {
                name,
                props,
                children,
            },
            Kind::List {
                ordered,
                start,
                items,
            } => Node::List {
                ordered,
                start,
                items,
            },
            Kind::Item { task } => {
                if let Some(Frame {
                    kind: Kind::List { items, .. },
                    ..
                }) = self.stack.last_mut()
                {
                    items.push(ListItem { task, children });
                }
                return;
            }
            Kind::TableCell => {
                if let Some(Frame {
                    kind: Kind::Table { row, .. },
                    ..
                }) = self.stack.last_mut()
                {
                    row.push(children);
                }
                return;
            }
            Kind::Table { head, rows, .. } => Node::Table { head, rows },
        };

        self.emit(node);
    }
}

/// Images ride in on a `Link` frame with a sentinel title so alt text can be collected.
fn image_title(title: &str) -> Option<&str> {
    title.strip_prefix("\0image\0")
}

/// Flattens a node tree to plain text, for headings, alt text, and the search index.
pub fn plain_text(nodes: &[Node]) -> String {
    let mut out = String::new();
    collect_text(nodes, &mut out);
    out.trim().to_string()
}

fn collect_text(nodes: &[Node], out: &mut String) {
    for node in nodes {
        match node {
            Node::Text(text) | Node::InlineCode(text) => out.push_str(text),
            Node::Break | Node::Rule => out.push(' '),
            Node::Emphasis(children)
            | Node::Strong(children)
            | Node::Strikethrough(children)
            | Node::Paragraph(children)
            | Node::Blockquote(children) => {
                collect_text(children, out);
                out.push(' ');
            }
            Node::Heading { children, .. } | Node::Link { children, .. } => {
                collect_text(children, out);
                out.push(' ');
            }
            Node::Component { children, .. } => collect_text(children, out),
            Node::Image { alt, .. } => out.push_str(alt),
            Node::CodeBlock { code, .. } => {
                out.push_str(code);
                out.push(' ');
            }
            Node::List { items, .. } => {
                for item in items {
                    collect_text(&item.children, out);
                    out.push(' ');
                }
            }
            Node::Table { head, rows } => {
                for cell in head.iter().chain(rows.iter().flatten()) {
                    collect_text(cell, out);
                    out.push(' ');
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn task_list_items_carry_their_state() {
        let doc = parse("- [x] Done\n- [ ] Not done\n- Plain\n").unwrap();
        let Node::List { items, .. } = &doc.nodes[0] else {
            panic!("expected a list, got {:?}", doc.nodes[0]);
        };
        assert_eq!(
            items.iter().map(|item| item.task).collect::<Vec<_>>(),
            vec![Some(true), Some(false), None]
        );
    }

    #[test]
    fn tasks_are_counted_across_the_page() {
        let doc = parse(
            "# Roadmap\n\n## Now\n\n- [x] One\n- [x] Two\n\n## Later\n\n- [ ] Three\n- [ ] Four\n",
        )
        .unwrap();
        assert_eq!(doc.tasks.total, 4);
        assert_eq!(doc.tasks.done, 2);
        assert_eq!(doc.tasks.percent(), 50);
    }

    #[test]
    fn nested_tasks_are_counted_too() {
        let doc = parse("- [x] Parent\n  - [ ] Child\n  - [x] Other child\n").unwrap();
        assert_eq!((doc.tasks.done, doc.tasks.total), (2, 3));
    }

    #[test]
    fn a_page_without_tasks_reports_no_progress() {
        let doc = parse("- One\n- Two\n").unwrap();
        assert_eq!(doc.tasks.total, 0);
        assert_eq!(doc.tasks.percent(), 0);
    }
    use super::*;

    #[test]
    fn directive_becomes_a_component_with_children() {
        let doc = parse("# Hi\n\n:::tip Note\nBody **text**.\n:::\n\nAfter.\n").unwrap();
        let Node::Component {
            name,
            props,
            children,
        } = &doc.nodes[1]
        else {
            panic!("expected a component, got {:?}", doc.nodes[1]);
        };
        assert_eq!(name, "Callout");
        assert_eq!(props[0], ("kind".into(), PropValue::Str("tip".into())));
        assert_eq!(props[1], ("title".into(), PropValue::Str("Note".into())));
        assert_eq!(children.len(), 1, "directive body should be nested inside");
        assert!(
            matches!(doc.nodes[2], Node::Paragraph(_)),
            "sibling after close"
        );
    }

    #[test]
    fn inline_component_keeps_expression_props() {
        let doc = parse("<Counter initial={5} />\n").unwrap();
        let text = format!("{:?}", doc.nodes);
        assert!(text.contains("Counter"), "{text}");
        assert!(text.contains("Expr(\"5\")"), "{text}");
    }

    #[test]
    fn headings_get_slugs_and_toc_entries() {
        let doc = parse("# Title\n\n## First Section\n\n### Deep\n").unwrap();
        assert_eq!(doc.first_heading.as_deref(), Some("Title"));
        assert_eq!(doc.toc.len(), 2);
        assert_eq!(doc.toc[0].id, "first-section");
        assert_eq!(doc.toc[1].level, 3);
    }

    #[test]
    fn code_blocks_keep_their_source() {
        let doc = parse("```rust\nfn main() {}\n```\n").unwrap();
        let Node::CodeBlock { lang, code, tokens } = &doc.nodes[0] else {
            panic!("expected a code block");
        };
        assert_eq!(lang.as_deref(), Some("rust"));
        assert_eq!(code, "fn main() {}\n");
        assert!(!tokens.is_empty());
    }
}
