//! The DioxusPress CLI.

mod check;
mod dx;
mod template;
mod watch;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "dxpress",
    version,
    about = "A Dioxus-native documentation framework"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new documentation site.
    New {
        /// Directory to create. Its final segment becomes the crate name.
        name: PathBuf,
        /// Depend on a local DioxusPress checkout instead of the published crates.
        #[arg(long, value_name = "DIR")]
        local: Option<PathBuf>,
        /// Force dependencies on the published crates.
        #[arg(long, conflicts_with = "local")]
        registry: bool,
    },
    /// Start the dev server with hot reloading.
    Dev {
        #[arg(long)]
        port: Option<u16>,
        /// Do not open a browser window.
        #[arg(long)]
        no_open: bool,
    },
    /// Build the site for production.
    Build {
        /// Pre-render every route to static HTML. Requires the `server` feature.
        #[arg(long)]
        ssg: bool,
        /// Where to copy the finished site.
        #[arg(long, default_value = "dist")]
        out: PathBuf,
    },
    /// Regenerate `generated/docs.rs` from the markdown without building.
    Generate,
    /// Parse the docs and report problems without compiling Rust.
    Check,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let cwd = std::env::current_dir().context("reading the current directory")?;

    match cli.command {
        Commands::New {
            name,
            local,
            registry,
        } => new(&cwd, &name, local, registry),
        Commands::Dev { port, no_open } => dev(&cwd, port, no_open),
        Commands::Build { ssg, out } => build(&cwd, ssg, &out),
        Commands::Generate => {
            let root = project_root(&cwd)?;
            let (path, changed) = dioxuspress::build::generate_to_file(&root)?;
            let shown = path.strip_prefix(&root).unwrap_or(&path);
            println!(
                "{} {}",
                if changed { "wrote" } else { "unchanged" },
                shown.display()
            );
            Ok(())
        }
        Commands::Check => check::run(&project_root(&cwd)?),
    }
}

fn new(cwd: &Path, name: &Path, local: Option<PathBuf>, registry: bool) -> Result<()> {
    let dir = cwd.join(name);
    let crate_name = name
        .file_name()
        .and_then(|n| n.to_str())
        .context("the project name must be valid UTF-8")?
        .to_string();
    anyhow::ensure!(
        crate_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
        "`{crate_name}` is not a valid crate name"
    );

    let deps = match (local, registry) {
        (Some(root), _) => template::Deps::Local(
            root.canonicalize()
                .with_context(|| format!("resolving --local {}", root.display()))?,
        ),
        (None, true) => template::Deps::Registry,
        (None, false) => template::Deps::detect(),
    };

    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    template::scaffold(&dir, &crate_name, &deps)?;
    dioxuspress::build::generate_to_file(&dir)?;

    println!("created {}", dir.display());
    println!("\nnext:");
    println!("  cd {}", name.display());
    println!("  dxpress dev");
    Ok(())
}

fn dev(cwd: &Path, port: Option<u16>, no_open: bool) -> Result<()> {
    let root = project_root(cwd)?;
    dx::ensure_available()?;

    let config = dioxuspress::core::config::Config::load(&root)?;
    dioxuspress::build::generate_to_file(&root)?;
    let _watcher = watch::spawn(
        &root,
        &config.docs_path(&root),
        &root.join(dioxuspress::core::config::CONFIG_FILE),
    )?;

    let mut args = vec!["serve".to_string(), "--web".to_string()];
    if let Some(port) = port {
        args.push("--port".to_string());
        args.push(port.to_string());
    }
    if no_open {
        args.push("--open".to_string());
        args.push("false".to_string());
    }
    dx::run(&root, &args)
}

fn build(cwd: &Path, ssg: bool, out: &Path) -> Result<()> {
    let root = project_root(cwd)?;
    dx::ensure_available()?;
    dioxuspress::build::generate_to_file(&root)?;

    let mut args = vec![
        "build".to_string(),
        "--web".to_string(),
        "--release".to_string(),
    ];
    if ssg {
        args.push("--fullstack".to_string());
        args.push("--ssg".to_string());
    }
    dx::run(&root, &args)?;

    let destination = root.join(out);
    match find_output(&root)? {
        Some(source) => {
            copy_dir(&source, &destination)?;
            println!("site written to {}", destination.display());
        }
        None => println!(
            "build finished; dx wrote the output under {}",
            root.join("target/dx").display()
        ),
    }
    Ok(())
}

/// Locates `target/dx/<app>/<profile>/web/public`, which is where dx puts a web build.
fn find_output(root: &Path) -> Result<Option<PathBuf>> {
    let dx_dir = root.join("target").join("dx");
    if !dx_dir.is_dir() {
        return Ok(None);
    }
    for entry in std::fs::read_dir(&dx_dir)? {
        let app = entry?.path();
        for profile in ["release", "debug"] {
            let candidate = app.join(profile).join("web").join("public");
            if candidate.is_dir() {
                return Ok(Some(candidate));
            }
        }
    }
    Ok(None)
}

fn copy_dir(source: &Path, destination: &Path) -> Result<()> {
    if destination.exists() {
        std::fs::remove_dir_all(destination)
            .with_context(|| format!("clearing {}", destination.display()))?;
    }
    std::fs::create_dir_all(destination)
        .with_context(|| format!("creating {}", destination.display()))?;

    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)
                .with_context(|| format!("copying {}", entry.path().display()))?;
        }
    }
    Ok(())
}

/// Walks up from `cwd` looking for a `dioxuspress.toml`.
fn project_root(cwd: &Path) -> Result<PathBuf> {
    for dir in cwd.ancestors() {
        if dir.join(dioxuspress::core::config::CONFIG_FILE).exists() {
            return Ok(dir.to_path_buf());
        }
    }
    anyhow::bail!(
        "no `{}` found in `{}` or any parent directory.\n\
         Run `dxpress new <name>` to create a site.",
        dioxuspress::core::config::CONFIG_FILE,
        cwd.display()
    )
}
