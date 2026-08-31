//! Regenerates `generated/docs.rs` whenever the markdown changes.

use anyhow::{Context, Result};
use notify::{Event, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

/// Editors save in bursts (write, rename, chmod); this collapses them into one run.
const DEBOUNCE: Duration = Duration::from_millis(120);

/// Watches `docs/` and the config file. Dropping the returned watcher stops it.
pub fn spawn(root: &Path, docs: &Path, config: &Path) -> Result<Box<dyn Watcher + Send>> {
    let languages = root.join(dioxuspress::core::lang::LANGUAGES_DIR);
    let (tx, rx) = mpsc::channel::<()>();

    let mut watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
        if let Ok(event) = event {
            if !event.kind.is_access() {
                let _ = tx.send(());
            }
        }
    })
    .context("starting the docs watcher")?;

    watcher
        .watch(docs, RecursiveMode::Recursive)
        .with_context(|| format!("watching {}", docs.display()))?;
    if languages.is_dir() {
        let _ = watcher.watch(&languages, RecursiveMode::Recursive);
    }
    if config.exists() {
        let _ = watcher.watch(config, RecursiveMode::NonRecursive);
    }

    let root = root.to_path_buf();
    std::thread::spawn(move || {
        while rx.recv().is_ok() {
            std::thread::sleep(DEBOUNCE);
            while rx.try_recv().is_ok() {}
            regenerate(&root);
        }
    });

    Ok(Box::new(watcher))
}

/// Writes the module and reports what happened.
fn regenerate(root: &PathBuf) {
    match dioxuspress::build::generate_to_file(root) {
        Ok((_, false)) => {}
        Ok((path, true)) => {
            let shown = path.strip_prefix(root).unwrap_or(&path);
            println!("dxpress: regenerated {}", shown.display());
        }
        Err(error) => eprintln!("dxpress: {error:#}"),
    }
}
