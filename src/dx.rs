//! Thin wrapper over the Dioxus CLI, which does the actual building and serving.

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

const DX: &str = "dx";

/// Fails with install instructions when the Dioxus CLI is missing.
pub fn ensure_available() -> Result<()> {
    let found = Command::new(DX)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success());

    anyhow::ensure!(
        found,
        "the Dioxus CLI (`dx`) was not found on PATH.\n\
         Install it with: cargo install dioxus-cli"
    );
    Ok(())
}

/// Runs `dx` in `dir`, streaming its output, and propagates a non-zero exit.
pub fn run(dir: &Path, args: &[String]) -> Result<()> {
    let status = Command::new(DX)
        .args(args)
        .current_dir(dir)
        .status()
        .with_context(|| format!("running `dx {}`", args.join(" ")))?;

    anyhow::ensure!(
        status.success(),
        "`dx {}` exited with {}",
        args.join(" "),
        status
    );
    Ok(())
}
