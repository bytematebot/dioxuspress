//! `dxpress check`: parses the docs and reports problems without a Rust build.

use anyhow::Result;
use dioxuspress::core::ast::Node;
use dioxuspress::core::config::Config;
use dioxuspress::core::site::{self, SidebarNode, Site};
use std::collections::HashSet;
use std::path::Path;

pub fn run(root: &Path) -> Result<()> {
    let config = Config::load(root)?;
    let languages = dioxuspress::core::lang::load(root)?;
    let site = site::load(
        &config.docs_path(root),
        &config.normalized_base(),
        &languages,
    )?;
    if !languages.is_empty() {
        let names: Vec<_> = languages.iter().map(|l| l.name.as_str()).collect();
        println!("languages: {}\n", names.join(", "));
    }

    let routes: HashSet<&str> = site.pages.iter().map(|page| page.route.as_str()).collect();
    let mut problems = 0;

    for page in &site.pages {
        let anchors: HashSet<&str> = page.doc.toc.iter().map(|entry| entry.id.as_str()).collect();
        let mut links = Vec::new();
        collect_links(&page.doc.nodes, &mut links);

        for href in links {
            let (path, anchor) = match href.split_once('#') {
                Some((path, anchor)) => (path, Some(anchor)),
                None => (href.as_str(), None),
            };

            if path.is_empty() {
                if let Some(anchor) = anchor {
                    if !anchors.contains(anchor) {
                        problems += 1;
                        println!("  {}: unknown anchor `#{anchor}`", page.file.display());
                    }
                }
                continue;
            }
            if !path.starts_with('/') || path.starts_with("//") {
                continue; // external or relative; not ours to resolve
            }
            let normalized = path.trim_end_matches('/');
            let normalized = if normalized.is_empty() {
                "/"
            } else {
                normalized
            };
            if config.has_landing() && normalized == "/" {
                continue; // the landing page, which is not markdown
            }
            if !routes.contains(normalized) {
                problems += 1;
                println!("  {}: link to unknown route `{path}`", page.file.display());
            }
        }
    }

    print_tree(&site);
    println!("\n{} page(s), {} problem(s)", site.pages.len(), problems);

    anyhow::ensure!(problems == 0, "check failed");
    Ok(())
}

fn print_tree(site: &Site) {
    println!("routes:");
    for page in &site.pages {
        println!("  {:<40} {}", page.route, page.title);
    }
    println!("\nsidebar:");
    print_nodes(&site.sidebar, 1);
}

fn print_nodes(nodes: &[SidebarNode], depth: usize) {
    let indent = "  ".repeat(depth);
    for node in nodes {
        match node {
            SidebarNode::Link { title, route } => println!("{indent}{title} -> {route}"),
            SidebarNode::Group {
                title,
                route,
                items,
            } => {
                match route {
                    Some(route) => println!("{indent}{title}/ -> {route}"),
                    None => println!("{indent}{title}/"),
                }
                print_nodes(items, depth + 1);
            }
        }
    }
}

fn collect_links(nodes: &[Node], out: &mut Vec<String>) {
    for node in nodes {
        match node {
            Node::Link { href, children, .. } => {
                out.push(href.clone());
                collect_links(children, out);
            }
            Node::Paragraph(children)
            | Node::Emphasis(children)
            | Node::Strong(children)
            | Node::Strikethrough(children)
            | Node::Blockquote(children)
            | Node::Heading { children, .. }
            | Node::Component { children, .. } => collect_links(children, out),
            Node::List { items, .. } => {
                for item in items {
                    collect_links(&item.children, out);
                }
            }
            Node::Table { head, rows } => {
                for cell in head.iter().chain(rows.iter().flatten()) {
                    collect_links(cell, out);
                }
            }
            _ => {}
        }
    }
}
