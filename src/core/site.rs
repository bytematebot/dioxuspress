//! Turns a `docs/` directory into routes and a sidebar.

use crate::core::ast::{Document, Node};
use crate::core::lang::Language;
use crate::core::parser;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Page {
    /// URL path, always leading-slashed: `/`, `/guides/deploy`.
    pub route: String,
    /// Rust identifier for the generated `Route` variant.
    pub ident: String,
    pub file: PathBuf,
    pub title: String,
    pub description: Option<String>,
    pub hidden: bool,
    pub doc: Document,
}

#[derive(Debug, Clone)]
pub enum SidebarNode {
    Link {
        title: String,
        route: String,
    },
    Group {
        title: String,
        /// Set when the directory has an `index.md` of its own.
        route: Option<String>,
        items: Vec<SidebarNode>,
    },
}

#[derive(Debug, Clone)]
pub struct Site {
    pub pages: Vec<Page>,
    pub sidebar: Vec<SidebarNode>,
}

/// Loads and parses every `.md` file under `root`. `base` prefixes every route; pass an
/// empty string to serve the docs at `/`.
pub fn load(root: &Path, base: &str, languages: &[Language]) -> Result<Site> {
    anyhow::ensure!(
        root.is_dir(),
        "docs directory `{}` does not exist",
        root.display()
    );
    let mut pages = Vec::new();
    let sidebar = walk(root, root, base, base, languages, &mut pages)?;
    anyhow::ensure!(
        !pages.is_empty(),
        "no markdown files found under `{}`",
        root.display()
    );
    rewrite_links(&mut pages, base);

    let mut ordered = Vec::with_capacity(pages.len());
    flatten(&sidebar, &pages, &mut ordered);
    for page in &pages {
        if !ordered.iter().any(|p: &Page| p.route == page.route) {
            ordered.push(page.clone());
        }
    }
    Ok(Site {
        pages: ordered,
        sidebar,
    })
}

/// Reads one directory, returning its sidebar entries and appending its pages.
fn walk(
    root: &Path,
    dir: &Path,
    base: &str,
    prefix: &str,
    languages: &[Language],
    pages: &mut Vec<Page>,
) -> Result<Vec<SidebarNode>> {
    let mut files: Vec<(PathBuf, Page)> = Vec::new();
    let mut dirs: Vec<PathBuf> = Vec::new();

    for entry in std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        if name.starts_with('.') || name.starts_with('_') {
            continue;
        }
        if path.is_dir() {
            dirs.push(path);
        } else if path.extension().is_some_and(|ext| ext == "md") {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            let route = if stem == "index" {
                route_for_index(prefix)
            } else {
                format!("{prefix}/{stem}")
            };
            files.push((path.clone(), read_page(&path, route, root, languages)?));
        }
    }

    let index_route = route_for_index(prefix);
    let has_index = files.iter().any(|(_, page)| page.route == index_route);

    let mut nodes: Vec<(Option<i64>, String, SidebarNode)> = Vec::new();
    for (_, page) in &files {
        if page.hidden || page.route == index_route {
            continue;
        }
        nodes.push((
            page.doc.frontmatter.order,
            page.title.clone(),
            SidebarNode::Link {
                title: page.title.clone(),
                route: page.route.clone(),
            },
        ));
    }

    for path in dirs {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        let child_prefix = format!("{prefix}/{name}");
        let items = walk(root, &path, base, &child_prefix, languages, pages)?;
        if items.is_empty() {
            continue;
        }
        let group_page = pages.iter().find(|p| p.route == child_prefix);
        nodes.push((
            group_page.and_then(|p| p.doc.frontmatter.order),
            group_page.map(|p| p.title.clone()).unwrap_or_default(),
            SidebarNode::Group {
                title: group_page
                    .map(|p| p.title.clone())
                    .unwrap_or_else(|| titleize(name)),
                route: group_page.map(|p| p.route.clone()),
                items,
            },
        ));
    }

    nodes.sort_by(|a, b| match (a.0, b.0) {
        (Some(x), Some(y)) => x.cmp(&y).then_with(|| a.1.cmp(&b.1)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.1.cmp(&b.1),
    });

    let mut result: Vec<SidebarNode> = nodes.into_iter().map(|(_, _, node)| node).collect();
    let _ = has_index;
    if prefix == base {
        if let Some((_, page)) = files.iter().find(|(_, page)| page.route == index_route) {
            if !page.hidden {
                result.insert(
                    0,
                    SidebarNode::Link {
                        title: page.title.clone(),
                        route: index_route.clone(),
                    },
                );
            }
        }
    }

    pages.extend(files.into_iter().map(|(_, page)| page));
    Ok(result)
}

/// An `index.md` takes its directory's own path; at the docs root that may be `/`.
fn route_for_index(prefix: &str) -> String {
    if prefix.is_empty() {
        "/".to_string()
    } else {
        prefix.to_string()
    }
}

/// Rewrites docs-relative links so authors can ignore `base_path`. A link that already
/// resolves is left alone.
fn rewrite_links(pages: &mut [Page], base: &str) {
    if base.is_empty() {
        return;
    }
    let routes: HashSet<String> = pages.iter().map(|page| page.route.clone()).collect();
    for page in pages.iter_mut() {
        rewrite_nodes(&mut page.doc.nodes, base, &routes);
    }
}

fn rewrite_nodes(nodes: &mut [Node], base: &str, routes: &HashSet<String>) {
    for node in nodes {
        match node {
            Node::Link { href, children, .. } => {
                if let Some(rewritten) = prefixed(href, base, routes) {
                    *href = rewritten;
                }
                rewrite_nodes(children, base, routes);
            }
            Node::Paragraph(children)
            | Node::Emphasis(children)
            | Node::Strong(children)
            | Node::Strikethrough(children)
            | Node::Blockquote(children)
            | Node::Heading { children, .. }
            | Node::Component { children, .. } => rewrite_nodes(children, base, routes),
            Node::List { items, .. } => {
                for item in items {
                    rewrite_nodes(&mut item.children, base, routes);
                }
            }
            Node::Table { head, rows } => {
                for cell in head.iter_mut().chain(rows.iter_mut().flatten()) {
                    rewrite_nodes(cell, base, routes);
                }
            }
            _ => {}
        }
    }
}

fn prefixed(href: &str, base: &str, routes: &HashSet<String>) -> Option<String> {
    if !href.starts_with('/') || href.starts_with("//") {
        return None;
    }
    let (path, fragment) = match href.split_once('#') {
        Some((path, fragment)) => (path, Some(fragment)),
        None => (href, None),
    };
    if routes.contains(path) {
        return None;
    }
    let candidate = format!("{base}{}", path.trim_end_matches('/'));
    if !routes.contains(&candidate) {
        return None;
    }
    Some(match fragment {
        Some(fragment) => format!("{candidate}#{fragment}"),
        None => candidate,
    })
}

fn read_page(path: &Path, route: String, root: &Path, languages: &[Language]) -> Result<Page> {
    let source =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let doc = parser::parse_with(&source, languages)
        .with_context(|| format!("parsing {}", path.display()))?;
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("page");
    let fallback = titleize(stem);
    let title = doc.title(&fallback).to_string();
    Ok(Page {
        ident: route_ident(&route),
        description: doc.frontmatter.description.clone(),
        hidden: doc.frontmatter.hidden,
        file: path.strip_prefix(root).unwrap_or(path).to_path_buf(),
        route,
        title,
        doc,
    })
}

fn flatten(nodes: &[SidebarNode], pages: &[Page], out: &mut Vec<Page>) {
    for node in nodes {
        match node {
            SidebarNode::Link { route, .. } => push_page(route, pages, out),
            SidebarNode::Group { route, items, .. } => {
                if let Some(route) = route {
                    push_page(route, pages, out);
                }
                flatten(items, pages, out);
            }
        }
    }
}

fn push_page(route: &str, pages: &[Page], out: &mut Vec<Page>) {
    if let Some(page) = pages.iter().find(|p| p.route == route) {
        if !out.iter().any(|p| p.route == page.route) {
            out.push(page.clone());
        }
    }
}

/// `getting-started` -> `Getting Started`.
pub fn titleize(name: &str) -> String {
    name.split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// `/getting-started/installation` -> `GettingStartedInstallation`.
pub fn route_ident(route: &str) -> String {
    let ident: String = route
        .split(['/', '-', '_', '.'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect();
    let ident: String = ident.chars().filter(|c| c.is_alphanumeric()).collect();
    if ident.is_empty() {
        "Home".to_string()
    } else if ident.chars().next().is_some_and(|c| c.is_numeric()) {
        format!("Page{ident}")
    } else {
        ident
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idents_and_titles() {
        assert_eq!(route_ident("/"), "Home");
        assert_eq!(
            route_ident("/getting-started/installation"),
            "GettingStartedInstallation"
        );
        assert_eq!(titleize("getting-started"), "Getting Started");
    }
}
