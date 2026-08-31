//! Lowers the document AST into `rsx!` tokens.

use crate::core::ast::{HlToken, ListItem, Node, PropValue};
use anyhow::{Context, Result};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Ident, LitStr};

/// A string literal safe to hand to `rsx!` as a format string.
pub fn lit(value: &str) -> LitStr {
    LitStr::new(
        &value.replace('{', "{{").replace('}', "}}"),
        Span::call_site(),
    )
}

/// A plain literal for contexts `rsx!` does not format, such as `const` initializers.
pub fn raw_lit(value: &str) -> LitStr {
    LitStr::new(value, Span::call_site())
}

pub fn children(nodes: &[Node]) -> Result<TokenStream> {
    let mut out = TokenStream::new();
    for node in nodes {
        out.extend(child(node)?);
    }
    Ok(out)
}

fn child(node: &Node) -> Result<TokenStream> {
    Ok(match node {
        Node::Text(text) => {
            let text = lit(text);
            quote! { #text }
        }
        Node::Break => quote! { br {} },
        Node::Rule => quote! { hr {} },
        Node::Emphasis(inner) => {
            let inner = children(inner)?;
            quote! { em { #inner } }
        }
        Node::Strong(inner) => {
            let inner = children(inner)?;
            quote! { strong { #inner } }
        }
        Node::Strikethrough(inner) => {
            let inner = children(inner)?;
            quote! { s { #inner } }
        }
        Node::InlineCode(text) => {
            let text = lit(text);
            quote! { code { class: "dp-inline-code", #text } }
        }
        Node::Paragraph(inner) => {
            let inner = children(inner)?;
            quote! { p { #inner } }
        }
        Node::Blockquote(inner) => {
            let inner = children(inner)?;
            quote! { blockquote { #inner } }
        }
        Node::Heading {
            level,
            id,
            children: inner,
        } => heading(*level, id, inner)?,
        Node::Link {
            href,
            title,
            children: inner,
        } => link(href, title.as_deref(), inner)?,
        Node::Image { src, alt } => {
            let src = lit(src);
            let alt = lit(alt);
            quote! { img { src: #src, alt: #alt, loading: "lazy" } }
        }
        Node::CodeBlock { lang, code, tokens } => code_block(lang.as_deref(), code, tokens),
        Node::List {
            ordered,
            start,
            items,
        } => list(*ordered, *start, items)?,
        Node::Table { head, rows } => table(head, rows)?,
        Node::Component {
            name,
            props,
            children: inner,
        } => component(name, props, inner)?,
    })
}

fn heading(level: u8, id: &str, inner: &[Node]) -> Result<TokenStream> {
    let inner = children(inner)?;
    let id_lit = lit(id);
    let href = lit(&format!("#{id}"));
    let anchor = if (2..=3).contains(&level) {
        quote! {
            a { class: "dp-heading-anchor", href: #href, aria_hidden: "true", "#" }
        }
    } else {
        TokenStream::new()
    };
    let tag = Ident::new(&format!("h{}", level.clamp(1, 6)), Span::call_site());
    Ok(quote! { #tag { id: #id_lit, #inner #anchor } })
}

fn link(href: &str, title: Option<&str>, inner: &[Node]) -> Result<TokenStream> {
    let inner = children(inner)?;
    let title_attr = match title {
        Some(title) => {
            let title = lit(title);
            quote! { title: #title, }
        }
        None => TokenStream::new(),
    };

    if is_internal(href) {
        let route = lit(href);
        return Ok(quote! {
            DocLink { route: #route, class: "", #inner }
        });
    }

    let href = lit(href);
    Ok(quote! {
        a { href: #href, #title_attr rel: "noreferrer", #inner }
    })
}

fn is_internal(href: &str) -> bool {
    href.starts_with('/') && !href.starts_with("//")
}

fn code_block(lang: Option<&str>, code: &str, tokens: &[HlToken]) -> TokenStream {
    let lang_prop = match lang {
        Some(lang) => {
            let lang = lit(lang);
            quote! { lang: #lang, }
        }
        None => TokenStream::new(),
    };
    let code_lit = lit(code);
    let tokens = tokens.iter().map(|token| {
        let (text, light, dark) = (
            raw_lit(&token.text),
            raw_lit(&token.light),
            raw_lit(&token.dark),
        );
        quote! { Token { text: #text, light: #light, dark: #dark } }
    });
    quote! {
        CodeBlock { #lang_prop code: #code_lit, tokens: &[ #(#tokens),* ] }
    }
}

fn list(ordered: bool, start: u64, items: &[ListItem]) -> Result<TokenStream> {
    let has_tasks = items.iter().any(|item| item.task.is_some());

    let items = items
        .iter()
        .map(|item| {
            let inner = children(&item.children)?;
            Ok(match item.task {
                Some(checked) => quote! {
                    li { class: "dp-task",
                        input {
                            r#type: "checkbox",
                            checked: #checked,
                            disabled: true,
                            tabindex: "-1",
                        }
                        span { class: "dp-task-body", #inner }
                    }
                },
                None => quote! { li { #inner } },
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let class = if has_tasks {
        quote! { class: "dp-task-list", }
    } else {
        TokenStream::new()
    };

    Ok(if ordered {
        let start = raw_lit(&start.to_string());
        quote! { ol { #class start: #start, #(#items)* } }
    } else {
        quote! { ul { #class #(#items)* } }
    })
}

fn table(head: &[Vec<Node>], rows: &[Vec<Vec<Node>>]) -> Result<TokenStream> {
    let head = head
        .iter()
        .map(|cell| {
            let inner = children(cell)?;
            Ok(quote! { th { #inner } })
        })
        .collect::<Result<Vec<_>>>()?;

    let body = rows
        .iter()
        .map(|row| {
            let cells = row
                .iter()
                .map(|cell| {
                    let inner = children(cell)?;
                    Ok(quote! { td { #inner } })
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(quote! { tr { #(#cells)* } })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(quote! {
        div { class: "dp-table-wrap",
            table {
                thead { tr { #(#head)* } }
                tbody { #(#body)* }
            }
        }
    })
}

fn component(name: &str, props: &[(String, PropValue)], inner: &[Node]) -> Result<TokenStream> {
    let ident = Ident::new(name, Span::call_site());
    let mut attrs = TokenStream::new();

    for (key, value) in props {
        let key_ident = ident_for(key);
        let value = match value {
            PropValue::Str(text) => {
                let text = lit(text);
                quote! { #text }
            }
            PropValue::Flag => quote! { true },
            PropValue::Expr(source) => {
                let expr: syn::Expr = syn::parse_str(source).with_context(|| {
                    format!("invalid Rust expression in `<{name} {key}={{{source}}}>`")
                })?;
                quote! { #expr }
            }
        };
        attrs.extend(quote! { #key_ident: #value, });
    }

    let inner = children(inner)?;
    Ok(quote! { #ident { #attrs #inner } })
}

/// Prop names may collide with keywords (`type`, `for`), so fall back to a raw ident.
fn ident_for(name: &str) -> Ident {
    syn::parse_str::<Ident>(name).unwrap_or_else(|_| Ident::new_raw(name, Span::call_site()))
}
