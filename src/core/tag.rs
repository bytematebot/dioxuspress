//! Parses component tags from raw HTML events. Only capitalized names count.

use crate::core::ast::PropValue;

#[derive(Debug, Clone, PartialEq)]
pub enum Tag {
    Open {
        name: String,
        props: Vec<(String, PropValue)>,
    },
    SelfClosing {
        name: String,
        props: Vec<(String, PropValue)>,
    },
    Close {
        name: String,
    },
}

/// Parses a single component tag. `None` for plain HTML, comments, or malformed input.
pub fn parse(input: &str) -> Option<Tag> {
    let input = input.trim();
    let body = input.strip_prefix('<')?.strip_suffix('>')?;

    if let Some(name) = body.strip_prefix('/') {
        let name = name.trim();
        return is_component(name).then(|| Tag::Close {
            name: name.to_string(),
        });
    }

    let (body, self_closing) = match body.strip_suffix('/') {
        Some(body) => (body, true),
        None => (body, false),
    };

    let mut scanner = Scanner::new(body);
    let name = scanner.ident()?;
    if !is_component(&name) {
        return None;
    }
    let props = scanner.props()?;

    Some(if self_closing {
        Tag::SelfClosing { name, props }
    } else {
        Tag::Open { name, props }
    })
}

/// Components are capitalized; everything else is left to plain HTML.
fn is_component(name: &str) -> bool {
    name.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        && name.chars().all(|c| c.is_alphanumeric() || c == '_')
}

struct Scanner<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Scanner<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn rest(&self) -> &'a str {
        &self.input[self.pos..]
    }

    fn skip_whitespace(&mut self) {
        let trimmed = self.rest().trim_start();
        self.pos = self.input.len() - trimmed.len();
    }

    fn ident(&mut self) -> Option<String> {
        self.skip_whitespace();
        let rest = self.rest();
        let len = rest
            .char_indices()
            .take_while(|(i, c)| c.is_alphanumeric() || *c == '_' || (*i > 0 && *c == '-'))
            .map(|(i, c)| i + c.len_utf8())
            .last()?;
        self.pos += len;
        Some(rest[..len].replace('-', "_"))
    }

    fn props(&mut self) -> Option<Vec<(String, PropValue)>> {
        let mut props = Vec::new();
        loop {
            self.skip_whitespace();
            if self.rest().is_empty() {
                return Some(props);
            }
            let name = self.ident()?;
            self.skip_whitespace();
            if !self.rest().starts_with('=') {
                props.push((name, PropValue::Flag));
                continue;
            }
            self.pos += 1;
            self.skip_whitespace();
            let value = match self.rest().chars().next()? {
                '"' => PropValue::Str(self.quoted('"')?),
                '\'' => PropValue::Str(self.quoted('\'')?),
                '{' => PropValue::Expr(self.braced()?),
                _ => return None,
            };
            props.push((name, value));
        }
    }

    fn quoted(&mut self, quote: char) -> Option<String> {
        let mut out = String::new();
        let mut chars = self.rest().char_indices();
        chars.next()?; // opening quote
        let mut escaped = false;
        for (i, c) in chars {
            if escaped {
                out.push(c);
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == quote {
                self.pos += i + c.len_utf8();
                return Some(out);
            } else {
                out.push(c);
            }
        }
        None
    }

    /// Captures a `{...}` value, tracking nesting and Rust string literals.
    fn braced(&mut self) -> Option<String> {
        let rest = self.rest();
        let mut depth = 0usize;
        let mut quote: Option<char> = None;
        let mut escaped = false;

        for (i, c) in rest.char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            match (quote, c) {
                (Some(_), '\\') => escaped = true,
                (Some(q), c) if c == q => quote = None,
                (Some(_), _) => {}
                (None, '"') => quote = Some('"'),
                (None, '{') => depth += 1,
                (None, '}') => {
                    depth -= 1;
                    if depth == 0 {
                        self.pos += i + 1;
                        return Some(rest[1..i].trim().to_string());
                    }
                }
                (None, _) => {}
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_self_closing_with_mixed_props() {
        let tag = parse(r#"<Counter initial={5} label="hits" wide />"#).unwrap();
        let Tag::SelfClosing { name, props } = tag else {
            panic!("expected self-closing");
        };
        assert_eq!(name, "Counter");
        assert_eq!(props[0], ("initial".into(), PropValue::Expr("5".into())));
        assert_eq!(props[1], ("label".into(), PropValue::Str("hits".into())));
        assert_eq!(props[2], ("wide".into(), PropValue::Flag));
    }

    #[test]
    fn keeps_braces_inside_string_literals() {
        let tag = parse(r#"<Fmt f={|x| format!("{x} ms")} />"#).unwrap();
        let Tag::SelfClosing { props, .. } = tag else {
            panic!("expected self-closing");
        };
        assert_eq!(
            props[0].1,
            PropValue::Expr(r#"|x| format!("{x} ms")"#.into())
        );
    }

    #[test]
    fn parses_open_and_close() {
        assert_eq!(
            parse("<Tabs>"),
            Some(Tag::Open {
                name: "Tabs".into(),
                props: vec![]
            })
        );
        assert_eq!(
            parse("</Tabs>"),
            Some(Tag::Close {
                name: "Tabs".into()
            })
        );
    }

    #[test]
    fn ignores_plain_html() {
        assert_eq!(parse("<div class=\"x\">"), None);
        assert_eq!(parse("<!-- comment -->"), None);
    }
}
