//! Temporary Cargo workspaces for compiling and running documented Rust.
//!
//! Each workspace owns a private package under the repository's `target/`
//! directory. The package's binaries are exact Markdown fence bodies wrapped in
//! a `main` function, so the published examples are compiled and executed
//! without copying them into a fixture.
//!
//! Reuse this helper only from documentation tests that need to exercise a
//! Rust fence. It does not rewrite example source, and it must not be used by
//! production code.

use std::{
    path::Path,
    process::{Command, Output},
};

use anyhow::{Context, Result, ensure};
use cap_std::{ambient_authority, fs::Dir};

use crate::documentation_examples::{DocumentedExample, is_valid_example_id};

/// An isolated package whose binaries are exact Rust fence bodies.
pub struct ExampleWorkspace {
    root: tempfile::TempDir,
    directory: Dir,
}

impl ExampleWorkspace {
    /// Create an empty package with the dependencies the guide examples use.
    ///
    /// # Errors
    ///
    /// Returns an error when the workspace directory or manifest cannot be
    /// created.
    pub fn new() -> Result<Self> {
        let base = repository_root().join("target/documentation-examples");
        Dir::create_ambient_dir_all(&base, ambient_authority())
            .with_context(|| format!("create {}", base.display()))?;
        let root = tempfile::Builder::new()
            .prefix("workspace-")
            .tempdir_in(&base)
            .with_context(|| format!("create example workspace in {}", base.display()))?;
        let directory = Dir::open_ambient_dir(root.path(), ambient_authority())
            .context("open the example workspace")?;
        directory
            .create_dir_all("src/bin")
            .context("create the binary source directory")?;
        let workspace = Self { root, directory };
        workspace
            .directory
            .write("Cargo.toml", manifest())
            .context("write the example manifest")?;
        workspace
            .install_repository_lockfile()
            .context("seed the example lockfile")?;
        Ok(workspace)
    }

    /// Seed the workspace with the repository's own lockfile.
    ///
    /// Cargo accepts versions named in a lockfile even after crates.io yanks
    /// them, but a package without a lockfile resolves afresh. Seeding makes
    /// the example package reuse the versions the repository already builds,
    /// so an offline resolution cannot fail on a yanked transitive crate.
    fn install_repository_lockfile(&self) -> Result<()> {
        let lockfile = Dir::open_ambient_dir(repository_root(), ambient_authority())
            .context("open the repository root")?
            .read("Cargo.lock")
            .context("read the repository lockfile")?;
        self.directory
            .write("Cargo.lock", lockfile)
            .context("write the example lockfile")
    }

    /// Add one Rust example as a binary without altering its published source.
    ///
    /// # Errors
    ///
    /// Returns an error when the example is not Rust, its identifier is unsafe
    /// for a path, or the source cannot be written.
    pub fn add_binary(&self, example: &DocumentedExample) -> Result<()> {
        ensure!(
            example.language == "rust",
            "documented example '{}' should be Rust, got '{}'",
            example.id,
            example.language
        );
        ensure!(
            is_valid_example_id(&example.id),
            "documented example '{}' is not a safe binary name",
            example.id
        );
        self.directory
            .write(
                format!("src/bin/{}.rs", example.id),
                wrapped_source(&example.body),
            )
            .with_context(|| format!("write documented example '{}'", example.id))
    }

    /// Build every binary added to the workspace.
    ///
    /// # Errors
    ///
    /// Returns an error containing Cargo's output when compilation fails.
    pub fn build(&self) -> Result<()> {
        let output = self
            .cargo()
            .args(["build", "--bins", "--offline"])
            .output()
            .context("build documented examples")?;
        ensure_success("documented examples should compile", &output)
    }

    /// Run one built example and require a successful exit.
    ///
    /// # Errors
    ///
    /// Returns an error when the example cannot be started or exits non-zero.
    pub fn run(&self, id: &str) -> Result<()> {
        ensure!(
            is_valid_example_id(id),
            "documented example '{id}' is not a safe binary name"
        );
        let output = self
            .cargo()
            .args(["run", "--offline", "--quiet", "--bin", id])
            .output()
            .with_context(|| format!("run documented example '{id}'"))?;
        ensure_success(&format!("documented example '{id}' should run"), &output)
    }

    fn cargo(&self) -> Command {
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| std::ffi::OsString::from("cargo"));
        let mut command = Command::new(cargo);
        command
            .current_dir(self.root.path())
            // Keep the build cache outside the temporary package so repeated
            // runs reuse compiled dependencies. Cargo's target-directory lock
            // serializes concurrent users.
            .env("CARGO_TARGET_DIR", target_directory())
            .env("CARGO_TERM_COLOR", "never")
            // The repository gate denies warnings for its own targets. The
            // generated examples are illustrative fragments, so inherited
            // `-D warnings` would fail them for unused imports and helpers.
            .env_remove("RUSTFLAGS")
            .env_remove("RUSTDOCFLAGS")
            .env_remove("CARGO_ENCODED_RUSTFLAGS");
        command
    }
}

fn ensure_success(context: &str, output: &Output) -> Result<()> {
    ensure!(
        output.status.success(),
        "{context}; stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn wrapped_source(body: &str) -> String {
    format!(
        concat!(
            "fn main() -> Result<(), Box<dyn std::error::Error>> {{\n",
            "{body}",
            "    Ok(())\n",
            "}}\n",
        ),
        body = body
    )
}

fn manifest() -> String {
    format!(
        concat!(
            "[package]\n",
            "name = \"documentation-examples\"\n",
            "version = \"0.0.0\"\n",
            "edition = \"2024\"\n",
            "\n",
            "[dependencies]\n",
            "actix_v2a = {{ path = \"{}\" }}\n",
            "actix-web = \"4\"\n",
            "serde = {{ version = \"1.0.228\", features = [\"derive\"] }}\n",
            "url = \"2.5.7\"\n",
            "utoipa = {{ version = \"6\", features = [\"macros\"] }}\n",
        ),
        escape_manifest_path(repository_root())
    )
}

fn escape_manifest_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}

fn repository_root() -> &'static Path { Path::new(env!("CARGO_MANIFEST_DIR")) }

/// The shared build cache for the generated example package.
fn target_directory() -> std::path::PathBuf {
    repository_root().join("target/documentation-examples/target")
}
