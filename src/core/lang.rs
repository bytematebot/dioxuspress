//! User-defined languages for code highlighting, declared in `languages/*.toml`.

use crate::core::ast::HlToken;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Where language definitions live, relative to the project root.
pub const LANGUAGES_DIR: &str = "languages";

/// A light/dark colour pair, both `#rrggbb`.
#[derive(Debug, Clone, Deserialize)]
pub struct Colour {
    pub light: String,
    pub dark: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Language {
    /// The fence info string this applies to, e.g. ```` ```bo ````.
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,

    /// Everything from this marker to end of line is a comment.
    #[serde(default)]
    pub line_comment: Option<String>,
    /// A sigil that turns the following word into an `attribute` token, e.g. `@`.
    #[serde(default)]
    pub attribute_prefix: Option<String>,
    /// Token classes whose word makes the *next* word a `declaration`.
    #[serde(default)]
    pub declaration_after: Vec<String>,
    /// Characters the tokenizer emits as `operator`.
    #[serde(default)]
    pub operators: String,

    /// Literal words per token class: `keyword = ["model", "enum"]`.
    #[serde(default)]
    pub words: HashMap<String, Vec<String>>,
    /// Colour per token class: `comment`, `string`, `attribute`, `number`, `brace`,
    /// `paren`, `operator`, `punctuation`, `identifier`, `declaration`.
    pub colours: HashMap<String, Colour>,
}

impl Language {
    pub fn matches(&self, lang: &str) -> bool {
        self.name == lang || self.aliases.iter().any(|alias| alias == lang)
    }

    fn colour(&self, class: &str) -> (&str, &str) {
        let entry = self
            .colours
            .get(class)
            .or_else(|| self.colours.get("identifier"));
        match entry {
            Some(colour) => (&colour.light, &colour.dark),
            None => ("#24292e", "#c0c5ce"),
        }
    }

    fn class_of(&self, word: &str) -> Option<&str> {
        self.words.iter().find_map(|(class, words)| {
            words
                .iter()
                .any(|candidate| candidate == word)
                .then_some(class.as_str())
        })
    }
}

/// Loads every `languages/*.toml` under `root`. A missing directory is not an error.
pub fn load(root: &Path) -> Result<Vec<Language>> {
    let dir = root.join(LANGUAGES_DIR);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut languages = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    entries.sort();

    for path in entries {
        let source = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let language: Language =
            toml::from_str(&source).with_context(|| format!("parsing {}", path.display()))?;
        languages.push(language);
    }
    Ok(languages)
}

/// Tokenizes with a user-defined language.
pub fn highlight(language: &Language, code: &str) -> Vec<HlToken> {
    let mut out: Vec<HlToken> = Vec::new();
    let bytes = code.as_bytes();
    let mut i = 0;
    let mut expect_declaration = false;

    while i < code.len() {
        let rest = &code[i..];
        let ch = bytes[i] as char;

        if ch.is_whitespace() {
            let len = rest
                .find(|c: char| !c.is_whitespace())
                .unwrap_or(rest.len());
            push(&mut out, &rest[..len], language.colour("identifier"));
            i += len;
            continue;
        }

        if let Some(marker) = &language.line_comment {
            if rest.starts_with(marker.as_str()) {
                let len = rest.find('\n').unwrap_or(rest.len());
                push(&mut out, &rest[..len], language.colour("comment"));
                i += len;
                continue;
            }
        }

        if ch == '"' {
            let len = string_len(rest);
            push(&mut out, &rest[..len], language.colour("string"));
            i += len;
            continue;
        }

        if let Some(prefix) = &language.attribute_prefix {
            if rest.starts_with(prefix.as_str()) {
                let len = prefix.len() + word_len(&rest[prefix.len()..]);
                push(&mut out, &rest[..len], language.colour("attribute"));
                i += len;
                continue;
            }
        }

        if ch.is_ascii_digit() {
            let len = rest
                .find(|c: char| !c.is_ascii_digit() && c != '.')
                .unwrap_or(rest.len());
            push(&mut out, &rest[..len], language.colour("number"));
            i += len;
            continue;
        }

        if ch.is_alphabetic() || ch == '_' {
            let len = word_len(rest);
            let word = &rest[..len];

            let class = if std::mem::take(&mut expect_declaration) {
                "declaration"
            } else {
                match language.class_of(word) {
                    Some(class) => {
                        expect_declaration = language
                            .declaration_after
                            .iter()
                            .any(|after| after == class);
                        class
                    }
                    None => "identifier",
                }
            };

            push(&mut out, word, language.colour(class));
            i += len;
            continue;
        }

        let class = match ch {
            '{' | '}' => "brace",
            '(' | ')' => "paren",
            _ if language.operators.contains(ch) => "operator",
            _ => "punctuation",
        };
        push(&mut out, &rest[..ch.len_utf8()], language.colour(class));
        i += ch.len_utf8();
    }

    out
}

fn word_len(rest: &str) -> usize {
    rest.find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(rest.len())
}

/// Length of a double-quoted string including both quotes, honouring backslash escapes.
fn string_len(rest: &str) -> usize {
    let mut escaped = false;
    for (index, ch) in rest.char_indices().skip(1) {
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return index + 1;
        } else if ch == '\n' {
            return index;
        }
    }
    rest.len()
}

/// Appends a run, merging with the previous one when the colour is unchanged.
fn push(out: &mut Vec<HlToken>, text: &str, (light, dark): (&str, &str)) {
    match out.last_mut() {
        Some(prev) if prev.light == light && prev.dark == dark => prev.text.push_str(text),
        _ => out.push(HlToken {
            text: text.to_string(),
            light: light.to_string(),
            dark: dark.to_string(),
        }),
    }
}
