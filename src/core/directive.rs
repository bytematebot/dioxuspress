//! Rewrites `:::tip` container directives into `<Callout>` tags before parsing.

use crate::core::ast::CalloutKind;

pub fn expand(source: &str) -> String {
    let mut out = String::with_capacity(source.len() + 64);
    let mut fence: Option<String> = None;
    let mut depth = 0usize;

    for line in source.split_inclusive('\n') {
        let trimmed = line.trim();

        match &fence {
            Some(marker) => {
                if trimmed.starts_with(marker.as_str()) {
                    fence = None;
                }
                out.push_str(line);
                continue;
            }
            None => {
                if let Some(marker) = fence_marker(trimmed) {
                    fence = Some(marker);
                    out.push_str(line);
                    continue;
                }
            }
        }

        if let Some(rest) = trimmed.strip_prefix(":::") {
            let rest = rest.trim();
            if rest.is_empty() {
                if depth > 0 {
                    depth -= 1;
                    out.push_str("\n</Callout>\n\n");
                    continue;
                }
            } else if let Some((kind, title)) = parse_open(rest) {
                depth += 1;
                out.push_str("\n<Callout kind=\"");
                out.push_str(kind.as_str());
                out.push('"');
                if let Some(title) = title {
                    out.push_str(" title=\"");
                    out.push_str(&escape_attr(&title));
                    out.push('"');
                }
                out.push_str(">\n\n");
                continue;
            }
        }

        out.push_str(line);
    }

    for _ in 0..depth {
        out.push_str("\n</Callout>\n");
    }
    out
}

fn parse_open(rest: &str) -> Option<(CalloutKind, Option<String>)> {
    let (name, title) = match rest.split_once(char::is_whitespace) {
        Some((name, title)) => (name, Some(title.trim().to_string())),
        None => (rest, None),
    };
    let kind = CalloutKind::from_name(&name.to_ascii_lowercase())?;
    Some((kind, title.filter(|t| !t.is_empty())))
}

fn fence_marker(trimmed: &str) -> Option<String> {
    for ch in ['`', '~'] {
        let run = trimmed.chars().take_while(|c| *c == ch).count();
        if run >= 3 {
            return Some(std::iter::repeat_n(ch, run).collect());
        }
    }
    None
}

fn escape_attr(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_tip_with_title() {
        let out = expand(":::tip Heads up\nbody\n:::\n");
        assert!(out.contains("<Callout kind=\"tip\" title=\"Heads up\">"));
        assert!(out.contains("</Callout>"));
        assert!(out.contains("body"));
    }

    #[test]
    fn leaves_code_blocks_alone() {
        let src = "```\n:::tip\n```\n";
        assert_eq!(expand(src), src);
    }

    #[test]
    fn closes_unbalanced_directive() {
        let out = expand(":::note\nbody\n");
        assert_eq!(out.matches("</Callout>").count(), 1);
    }
}
