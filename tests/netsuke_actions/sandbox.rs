//! Runs the repository `Netsukefile` with the real `netsuke` against fake tools.
//!
//! Each sandbox owns a private working directory, home directory, and tool
//! directory, so the generated Ninja graph, Netsuke's `.netsuke/` state, and the
//! command log never touch the checkout. The child process starts from an empty
//! environment: `HOME` points into the sandbox, so the manifest's
//! `$HOME/.cargo/bin` prepend cannot reach the host's real Cargo or nextest, and
//! `PATH` holds only the fake tools and the system directories that supply
//! `/bin/sh` and `ninja`.

use std::{env, process::Command};

use anyhow::{Context, Result};
use cap_std::{
    ambient_authority,
    fs::PermissionsExt,
    fs_utf8::{
        Dir,
        camino::{Utf8Path, Utf8PathBuf},
    },
};
use tempfile::TempDir;

/// The manifest under test, read at compile time so a test always runs the
/// committed recipes rather than a copy.
const MANIFEST: &str = include_str!("../../Netsukefile");

/// Logs `tool<TAB>arg...` and the flags a gate depends on, then fails when the
/// invocation matches `FAILING_COMMAND`, either by tool name or by tool name
/// and first argument (for example `cargo clippy`).
const FAKE_TOOL: &str = concat!(
    "#!/bin/sh\n",
    "tool=${0##*/}\n",
    "line=$tool\n",
    "for argument in \"$@\"; do line=$(printf '%s\\t%s' \"$line\" \"$argument\"); done\n",
    "printf '%s\\n' \"$line\" >> \"$ACTION_LOG\"\n",
    "printf '%s\\tRUSTFLAGS=%s\\tRUSTDOCFLAGS=%s\\tUV_CACHE_DIR=%s\\n' \\\n",
    "  \"$tool ${1:-}\" \"${RUSTFLAGS:-}\" \"${RUSTDOCFLAGS:-}\" \"${UV_CACHE_DIR:-}\" \\\n",
    "  >> \"$ENV_LOG\"\n",
    "case \"${FAILING_COMMAND:-}\" in\n",
    "  \"$tool\"|\"$tool ${1:-}\") exit 23 ;;\n",
    "esac\n",
);

/// Tools every gate in the manifest may call. Individual tests remove or
/// relocate entries to exercise missing-tool and `PATH`-prepend behaviour.
pub const DEFAULT_TOOLS: [&str; 6] = [
    "cargo",
    "whitaker",
    "mdtablefix",
    "markdownlint-cli2",
    "uv",
    "nixie",
];

/// Where a fake tool is installed inside the sandbox.
#[derive(Clone, Copy, Debug)]
pub enum ToolHome {
    /// The directory placed on the child's `PATH`.
    Path,
    /// `$HOME/.local/bin`, which only the manifest's prepend puts on `PATH`.
    LocalBin,
}

/// One isolated Netsuke invocation environment.
pub struct ActionSandbox {
    root: TempDir,
    directory: Dir,
    netsuke: Utf8PathBuf,
    variables: Vec<(String, String)>,
}

impl ActionSandbox {
    /// Creates a sandbox holding the manifest and no tools.
    ///
    /// # Errors
    ///
    /// Returns an error when the temporary tree cannot be created or no
    /// `netsuke` executable is on the test process's `PATH`.
    pub fn empty() -> Result<Self> {
        let netsuke = netsuke_binary()?;
        let root = tempfile::tempdir().context("create sandbox root")?;
        let root_path = Utf8Path::from_path(root.path()).context("sandbox path is not UTF-8")?;
        let directory =
            Dir::open_ambient_dir(root_path, ambient_authority()).context("open sandbox root")?;
        for path in ["work", "tools", "home/.local/bin"] {
            directory
                .create_dir_all(path)
                .with_context(|| format!("create {path}"))?;
        }
        directory
            .write("work/Netsukefile", MANIFEST)
            .context("write Netsukefile")?;
        Ok(Self {
            root,
            directory,
            netsuke,
            variables: Vec::new(),
        })
    }

    /// Creates a sandbox with every default tool on `PATH`.
    ///
    /// # Errors
    ///
    /// Returns an error when [`ActionSandbox::empty`] or tool creation fails.
    pub fn with_default_tools() -> Result<Self> {
        let sandbox = Self::empty()?;
        for tool in DEFAULT_TOOLS {
            sandbox.install(tool, ToolHome::Path)?;
        }
        Ok(sandbox)
    }

    /// Installs one executable fake tool.
    ///
    /// # Errors
    ///
    /// Returns an error when the script cannot be written or made executable.
    pub fn install(&self, tool: &str, home: ToolHome) -> Result<()> {
        let path = match home {
            ToolHome::Path => format!("tools/{tool}"),
            ToolHome::LocalBin => format!("home/.local/bin/{tool}"),
        };
        self.directory
            .write(&path, FAKE_TOOL)
            .with_context(|| format!("write {path}"))?;
        let mut permissions = self.directory.metadata(&path)?.permissions();
        permissions.set_mode(0o755);
        self.directory
            .set_permissions(&path, permissions)
            .with_context(|| format!("make {path} executable"))
    }

    /// Adds an environment variable to the Netsuke child process.
    pub fn set_var(&mut self, name: &str, value: &str) {
        self.variables.push((name.to_owned(), value.to_owned()));
    }

    /// Runs `netsuke` with `arguments` in the sandbox working directory.
    ///
    /// # Errors
    ///
    /// Returns an error when the process cannot be started.
    pub fn netsuke(&self, arguments: &[&str]) -> Result<std::process::Output> {
        let root = self.root_path()?;
        let path = env::join_paths([root.join("tools").as_str(), "/usr/bin", "/bin"])
            .context("join sandbox PATH")?;
        Command::new(&self.netsuke)
            .args(arguments)
            .current_dir(root.join("work"))
            .env_clear()
            .env("PATH", path)
            .env("HOME", root.join("home"))
            .env("ACTION_LOG", root.join("commands.log"))
            .env("ENV_LOG", root.join("environment.log"))
            .envs(self.variables.iter().map(|(name, value)| (name, value)))
            .output()
            .context("run netsuke")
    }

    /// Returns each logged invocation as `tool<TAB>arg...`, in execution order.
    ///
    /// # Errors
    ///
    /// Returns an error when the log exists but cannot be read.
    pub fn commands(&self) -> Result<Vec<String>> { self.log_lines("commands.log") }

    /// Returns each logged `tool first-arg<TAB>NAME=value...` environment record.
    ///
    /// # Errors
    ///
    /// Returns an error when the log exists but cannot be read.
    pub fn environment(&self) -> Result<Vec<String>> { self.log_lines("environment.log") }

    fn log_lines(&self, name: &str) -> Result<Vec<String>> {
        if !self.directory.exists(name) {
            return Ok(Vec::new());
        }
        let text = self
            .directory
            .read_to_string(name)
            .with_context(|| format!("read {name}"))?;
        Ok(text.lines().map(str::to_owned).collect())
    }

    fn root_path(&self) -> Result<&Utf8Path> {
        Utf8Path::from_path(self.root.path()).context("sandbox path is not UTF-8")
    }
}

/// Finds the installed `netsuke` before the sandbox narrows `PATH`.
fn netsuke_binary() -> Result<Utf8PathBuf> {
    let path = env::var_os("PATH").unwrap_or_default();
    env::split_paths(&path)
        .map(|directory| directory.join("netsuke"))
        .find(|candidate| candidate.is_file())
        .and_then(|candidate| Utf8PathBuf::from_path_buf(candidate).ok())
        .context("netsuke must be on PATH to run the Netsukefile action tests")
}
