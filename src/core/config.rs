//! `dioxus-press.toml`, the only configuration file a site needs, and it is optional.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

pub const CONFIG_FILE: &str = "dioxus-press.toml";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    /// Site title, shown in the navbar and appended to page titles.
    pub title: String,
    pub description: Option<String>,
    /// Repository URL for the navbar link.
    pub repository: Option<String>,
    /// Markdown root, relative to the project.
    pub docs_dir: PathBuf,
    /// URL prefix for every documentation route. Empty serves the docs at `/`.
    pub base_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            title: "Documentation".to_string(),
            description: None,
            repository: None,
            docs_dir: PathBuf::from("docs"),
            base_path: String::new(),
        }
    }
}

impl Config {
    /// Loads `dioxus-press.toml` from `root`, falling back to defaults when absent.
    pub fn load(root: &Path) -> Result<Self> {
        let path = root.join(CONFIG_FILE);
        if !path.exists() {
            return Ok(Self::default());
        }
        let source = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&source).with_context(|| format!("parsing {}", path.display()))
    }

    pub fn docs_path(&self, root: &Path) -> PathBuf {
        root.join(&self.docs_dir)
    }

    /// The base path with a leading slash and no trailing one. Empty means the root.
    pub fn normalized_base(&self) -> String {
        let trimmed = self.base_path.trim().trim_matches('/');
        if trimmed.is_empty() {
            String::new()
        } else {
            format!("/{trimmed}")
        }
    }

    /// Where the documentation index lives, for links back to it.
    pub fn docs_root(&self) -> String {
        let base = self.normalized_base();
        if base.is_empty() {
            "/".to_string()
        } else {
            base
        }
    }

    /// Whether the root is free for a landing page.
    pub fn has_landing(&self) -> bool {
        !self.normalized_base().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_base_paths() {
        let base = |value: &str| Config {
            base_path: value.to_string(),
            ..Config::default()
        };
        assert_eq!(base("").normalized_base(), "");
        assert_eq!(base("/docs").normalized_base(), "/docs");
        assert_eq!(base("docs/").normalized_base(), "/docs");
        assert_eq!(base("/").normalized_base(), "");
        assert_eq!(base("").docs_root(), "/");
        assert_eq!(base("/docs").docs_root(), "/docs");
        assert!(!base("").has_landing());
        assert!(base("/docs").has_landing());
    }
}
