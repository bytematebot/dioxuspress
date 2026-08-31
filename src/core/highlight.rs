//! Build-time syntax highlighting, resolved for a light and a dark theme.

use crate::core::ast::HlToken;
use crate::core::lang::Language;
use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

const LIGHT_THEME: &str = "InspiredGitHub";
const DARK_THEME: &str = "base16-ocean.dark";

struct Assets {
    syntaxes: SyntaxSet,
    light: Theme,
    dark: Theme,
}

fn assets() -> &'static Assets {
    static ASSETS: OnceLock<Assets> = OnceLock::new();
    ASSETS.get_or_init(|| {
        let mut themes = ThemeSet::load_defaults();
        let light = themes
            .themes
            .remove(LIGHT_THEME)
            .expect("syntect ships the light theme");
        let dark = themes
            .themes
            .remove(DARK_THEME)
            .expect("syntect ships the dark theme");
        Assets {
            syntaxes: SyntaxSet::load_defaults_newlines(),
            light,
            dark,
        }
    })
}

/// Highlights `code` as `lang`, falling back to plain text one token per line.
pub fn highlight(code: &str, lang: Option<&str>, custom: &[Language]) -> Vec<HlToken> {
    if let Some(name) = lang.map(str::trim) {
        if let Some(language) = custom.iter().find(|language| language.matches(name)) {
            return crate::core::lang::highlight(language, code);
        }
    }

    let assets = assets();
    let syntax = lang
        .map(|lang| lang.trim())
        .filter(|lang| !lang.is_empty())
        .and_then(|lang| {
            assets
                .syntaxes
                .find_syntax_by_token(lang)
                .or_else(|| assets.syntaxes.find_syntax_by_extension(lang))
        })
        .unwrap_or_else(|| assets.syntaxes.find_syntax_plain_text());

    let mut light = HighlightLines::new(syntax, &assets.light);
    let mut dark = HighlightLines::new(syntax, &assets.dark);
    let mut tokens: Vec<HlToken> = Vec::new();

    for line in LinesWithEndings::from(code) {
        let Ok(light_runs) = light.highlight_line(line, &assets.syntaxes) else {
            return vec![plain(code)];
        };
        let Ok(dark_runs) = dark.highlight_line(line, &assets.syntaxes) else {
            return vec![plain(code)];
        };
        merge_line(line, &light_runs, &dark_runs, &mut tokens);
    }

    tokens
}

fn plain(code: &str) -> HlToken {
    HlToken {
        text: code.to_string(),
        light: "#24292e".to_string(),
        dark: "#c0c5ce".to_string(),
    }
}

type Runs<'a> = [(syntect::highlighting::Style, &'a str)];

/// Cuts on the union of both themes' boundaries, so each token has one colour pair.
fn merge_line(line: &str, light: &Runs, dark: &Runs, out: &mut Vec<HlToken>) {
    let mut cuts: Vec<usize> = Vec::new();
    for runs in [light, dark] {
        let mut offset = 0;
        for (_, text) in runs {
            cuts.push(offset);
            offset += text.len();
        }
    }
    cuts.push(line.len());
    cuts.sort_unstable();
    cuts.dedup();

    for pair in cuts.windows(2) {
        let (start, end) = (pair[0], pair[1]);
        if start >= end || !line.is_char_boundary(start) || !line.is_char_boundary(end) {
            continue;
        }
        let text = &line[start..end];
        let token = HlToken {
            text: text.to_string(),
            light: color_at(light, start),
            dark: color_at(dark, start),
        };
        match out.last_mut() {
            Some(prev) if prev.light == token.light && prev.dark == token.dark => {
                prev.text.push_str(&token.text)
            }
            _ => out.push(token),
        }
    }
}

fn color_at(runs: &Runs, offset: usize) -> String {
    let mut cursor = 0;
    for (style, text) in runs {
        if offset < cursor + text.len() {
            let c = style.foreground;
            return format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b);
        }
        cursor += text.len();
    }
    "inherit".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlights_rust_into_multiple_colors() {
        let tokens = highlight("let x = 1;\n", Some("rust"), &[]);
        assert!(tokens.len() > 1, "expected several tokens: {tokens:?}");
        let rebuilt: String = tokens.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(rebuilt, "let x = 1;\n");
    }

    #[test]
    fn unknown_language_round_trips_text() {
        let tokens = highlight("hello\nworld\n", Some("not-a-language"), &[]);
        let rebuilt: String = tokens.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(rebuilt, "hello\nworld\n");
    }
}

#[cfg(test)]
mod routing_tests {
    use super::*;

    fn byteorm_like() -> Language {
        toml::from_str(
            r##"
name = "bo"
line_comment = "//"
declaration_after = ["keyword"]

[words]
keyword = ["model"]

[colours]
keyword = { light = "#1d4ed8", dark = "#3b82f6" }
declaration = { light = "#047857", dark = "#86efac" }
identifier = { light = "#1e293b", dark = "#f8fafc" }
"##,
        )
        .expect("the definition parses")
    }

    #[test]
    fn a_custom_language_takes_over_from_syntect() {
        let tokens = highlight("model User {}", Some("bo"), &[byteorm_like()]);
        assert_eq!(tokens[0].dark, "#3b82f6", "keyword: {tokens:?}");
        assert_eq!(tokens[2].dark, "#86efac", "declared name: {tokens:?}");
    }

    #[test]
    fn an_unknown_language_still_falls_back() {
        let tokens = highlight("model User {}", Some("bo"), &[]);
        let rebuilt: String = tokens.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(rebuilt, "model User {}");
    }
}
