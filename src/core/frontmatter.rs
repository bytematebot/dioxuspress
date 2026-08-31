use crate::core::ast::Frontmatter;
use anyhow::{Context, Result};

/// Splits a leading `---` YAML block off the source, returning it and the remaining body.
pub fn split(source: &str) -> Result<(Frontmatter, &str)> {
    let rest = match source.strip_prefix("---") {
        Some(rest) => rest.trim_start_matches([' ', '\t']),
        None => return Ok((Frontmatter::default(), source)),
    };
    let rest = match rest.strip_prefix('\n') {
        Some(rest) => rest,
        None => return Ok((Frontmatter::default(), source)),
    };

    let Some((yaml, body)) = find_close(rest) else {
        return Ok((Frontmatter::default(), source));
    };

    let raw: RawFrontmatter =
        serde_yaml_ng::from_str(yaml).context("failed to parse YAML frontmatter")?;
    Ok((raw.into(), body))
}

/// Finds the closing `---` line, returning (yaml, body-after-it).
fn find_close(rest: &str) -> Option<(&str, &str)> {
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end() == "---" {
            return Some((&rest[..offset], &rest[offset + line.len()..]));
        }
        offset += line.len();
    }
    None
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFrontmatter {
    title: Option<String>,
    description: Option<String>,
    order: Option<i64>,
    #[serde(default)]
    hidden: bool,
    #[serde(default = "yes")]
    pager: bool,
}

/// Pages take part in prev/next unless they opt out.
fn yes() -> bool {
    true
}

impl From<RawFrontmatter> for Frontmatter {
    fn from(raw: RawFrontmatter) -> Self {
        Frontmatter {
            title: raw.title,
            description: raw.description,
            order: raw.order,
            hidden: raw.hidden,
            pager: raw.pager,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_frontmatter() {
        let (fm, body) = split("---\ntitle: Signals\norder: 2\n---\n# Hi\n").unwrap();
        assert_eq!(fm.title.as_deref(), Some("Signals"));
        assert_eq!(fm.order, Some(2));
        assert_eq!(body, "# Hi\n");
    }

    #[test]
    fn passes_through_without_frontmatter() {
        let (fm, body) = split("# Hi\n").unwrap();
        assert_eq!(fm, Frontmatter::default());
        assert_eq!(body, "# Hi\n");
    }

    #[test]
    fn leaves_thematic_break_alone() {
        let (_, body) = split("---\n").unwrap();
        assert_eq!(body, "---\n");
    }
}
