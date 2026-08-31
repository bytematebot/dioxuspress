//! The `dxpress new` scaffold, embedded in the binary.

use anyhow::{Context, Result};
use std::path::Path;

/// A template file. `contents` may contain `{{name}}`, `{{ui_dep}}`, `{{build_dep}}`.
struct TemplateFile {
    path: &'static str,
    contents: &'static str,
}

macro_rules! template_files {
    ($($path:literal),* $(,)?) => {
        &[$(TemplateFile {
            path: $path,
            contents: include_str!(concat!("../templates/default/", $path)),
        }),*]
    };
}

const FILES: &[TemplateFile] = template_files![
    "dioxus-press.toml",
    "src/main.rs",
    "src/components.rs",
    "src/landing.rs",
    "src/theme/mod.rs",
    "src/theme/nav.rs",
    "src/theme/progress.rs",
    "src/theme/code.rs",
    "src/theme/callout.rs",
    "src/theme/toggle.rs",
    "assets/tailwind.css",
    "tailwind.css",
    "docs/index.md",
    "docs/getting-started/index.md",
    "docs/getting-started/installation.md",
    "docs/getting-started/configuration.md",
    "docs/guides/index.md",
    "docs/guides/deployment.md",
    "docs/roadmap.md",
];

const MANIFEST: &str = include_str!("../templates/default/Cargo.toml.tmpl");

/// Stored undotted: `cargo package` strips `.gitignore` from a published crate.
const GITIGNORE: &str = include_str!("../templates/default/gitignore");

/// The checkout this binary was compiled from, if it is still on disk.
///
/// `.git` is the marker because `cargo package` never ships it, so an install from
/// crates.io lands here with nothing to find. `--local` and `--registry` override it.
fn local_checkout() -> Option<std::path::PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    root.join(".git").exists().then(|| root.to_path_buf())
}

/// How the generated project should depend on the Dioxus Press crates.
pub enum Deps {
    /// Published crates, the normal case.
    Registry,
    /// Path dependencies into a checkout, for working on Dioxus Press itself.
    Local(std::path::PathBuf),
}

impl Deps {
    /// Prefers a live checkout; an install from crates.io yields `Registry`.
    pub fn detect() -> Self {
        match local_checkout() {
            Some(root) => Deps::Local(root),
            None => Deps::Registry,
        }
    }

    /// `default-features = false` keeps the CLI's parser and highlighter out of the
    /// site's build; all it needs is `dioxus_press::types`.
    fn entry(&self) -> String {
        match self {
            Deps::Registry => format!(
                "{{ version = \"{}\", default-features = false }}",
                env!("CARGO_PKG_VERSION")
            ),
            Deps::Local(root) => format!(
                "{{ path = \"{}\", default-features = false }}",
                root.display()
            ),
        }
    }
}

/// Writes the scaffold into `dir`, which must not already contain a project.
pub fn scaffold(dir: &Path, name: &str, deps: &Deps) -> Result<()> {
    anyhow::ensure!(
        !dir.join("Cargo.toml").exists(),
        "`{}` already contains a Cargo project",
        dir.display()
    );

    let manifest = MANIFEST
        .replace("{{name}}", name)
        .replace("{{types_dep}}", &deps.entry());
    write(dir, "Cargo.toml", &manifest)?;

    write(dir, ".gitignore", GITIGNORE)?;

    for file in FILES {
        write(dir, file.path, &file.contents.replace("{{name}}", name))?;
    }
    Ok(())
}

fn write(dir: &Path, relative: &str, contents: &str) -> Result<()> {
    let path = dir.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(&path, contents).with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn template_dir() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/default")
    }

    /// A file missing from `FILES` never reaches a scaffolded project.
    #[test]
    fn every_template_file_is_scaffolded() {
        let root = template_dir();
        let listed: std::collections::HashSet<&str> = FILES.iter().map(|file| file.path).collect();

        let mut missing = Vec::new();
        let mut stack = vec![root.clone()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).expect("the template is present") {
                let path = entry.expect("readable entry").path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                let relative = path
                    .strip_prefix(&root)
                    .expect("under the template root")
                    .to_string_lossy()
                    .replace('\\', "/");
                if matches!(relative.as_str(), "Cargo.toml.tmpl" | "gitignore")
                    || listed.contains(relative.as_str())
                {
                    continue;
                }
                missing.push(relative);
            }
        }

        missing.sort();
        assert!(missing.is_empty(), "not listed in FILES: {missing:?}");
    }

    /// No workspace build type-checks the theme, so at least parse it.
    #[test]
    fn scaffolded_sources_parse() {
        let dir = tempdir();
        scaffold(&dir, "demo", &Deps::Registry).expect("scaffolding succeeds");

        let mut checked = 0;
        let mut stack = vec![dir.join("src")];
        while let Some(current) = stack.pop() {
            for entry in std::fs::read_dir(&current).expect("readable directory") {
                let path = entry.expect("readable entry").path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().is_some_and(|ext| ext == "rs") {
                    let source = std::fs::read_to_string(&path).expect("readable source");
                    syn::parse_file(&source)
                        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
                    checked += 1;
                }
            }
        }

        assert!(
            checked >= 8,
            "expected the whole theme, parsed {checked} files"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    /// A registry scaffold must name published versions for `cargo build` to resolve.
    #[test]
    fn a_registry_scaffold_has_no_path_dependencies() {
        let dir = tempdir();
        scaffold(&dir, "demo", &Deps::Registry).expect("scaffolding succeeds");

        let manifest = std::fs::read_to_string(dir.join("Cargo.toml")).expect("a manifest");
        assert!(manifest.contains("name = \"demo\""), "{manifest}");
        assert!(
            manifest.contains(&format!(
                "dioxus-press = {{ version = \"{}\", default-features = false }}",
                env!("CARGO_PKG_VERSION")
            )),
            "{manifest}"
        );
        assert!(!manifest.contains("path ="), "{manifest}");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// The template stores it undotted; the scaffold has to restore the dot.
    #[test]
    fn a_scaffold_gets_a_dotted_gitignore() {
        let dir = tempdir();
        scaffold(&dir, "demo", &Deps::Registry).expect("scaffolding succeeds");

        let ignored = std::fs::read_to_string(dir.join(".gitignore")).expect("a .gitignore");
        assert!(ignored.contains("/generated"), "{ignored}");
        assert!(!dir.join("gitignore").exists(), "the undotted copy leaked");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// A site that picked up the default features would drag the parser, the
    /// highlighter and clap into its wasm build.
    #[test]
    fn a_scaffold_takes_the_types_only_dependency() {
        for deps in [Deps::Registry, Deps::Local("/checkout".into())] {
            let dir = tempdir();
            scaffold(&dir, "demo", &deps).expect("scaffolding succeeds");

            let manifest = std::fs::read_to_string(dir.join("Cargo.toml")).expect("a manifest");
            let line = manifest
                .lines()
                .find(|line| line.starts_with("dioxus-press ="))
                .expect("a dioxus-press dependency");
            assert!(line.contains("default-features = false"), "{line}");
            std::fs::remove_dir_all(&dir).ok();
        }
    }

    #[test]
    fn scaffolding_refuses_to_overwrite_a_project() {
        let dir = tempdir();
        std::fs::create_dir_all(&dir).expect("a directory");
        std::fs::write(dir.join("Cargo.toml"), "[package]").expect("a manifest");

        let error = scaffold(&dir, "demo", &Deps::Registry).expect_err("refuses to overwrite");
        assert!(error.to_string().contains("already contains"), "{error}");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// A unique scratch directory, without a temp-file dependency.
    fn tempdir() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);

        let unique = format!(
            "dxpress-test-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let dir = std::env::temp_dir().join(unique);
        std::fs::remove_dir_all(&dir).ok();
        dir
    }
}
