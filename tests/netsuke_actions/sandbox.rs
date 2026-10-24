//! Runs the repository `Netsukefile` with the real `netsuke` against fake tools.
//!
//! Each sandbox owns a private working directory, home directory, and tool
//! directory, so the generated Ninja graph, Netsuke's `.netsuke/` state, and the
//! command log never touch the checkout. The child process starts from an empty
//! environment: `HOME` points into the sandbox, so the manifest's
//! `$HOME/.cargo/bin` prepend cannot reach the host's real Cargo or nextest, and
//! `PATH` holds only the fake tools and the system directories that supply
//! `/bin/sh` and the POSIX utilities the fake tools use. Netsuke finds Ninja
//! through `NETSUKE_NINJA`, resolved from the host before `PATH` is narrowed,
//! so a Ninja installed outside `/usr/bin` still works without widening the
//! recipes' `PATH`.

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

/// A fake tool the sandbox can install.
#[derive(Clone, Copy, Debug)]
pub enum FakeTool {
    Cargo,
    CargoNextest,
    MarkdownlintCli2,
    Mdtablefix,
    Nixie,
    /// A Cargo stand-in with a different name, to test the `CARGO` override.
    PinnedCargo,
    Uv,
    Whitaker,
}

impl FakeTool {
    /// The executable name the manifest invokes.
    const fn executable(self) -> &'static str {
        match self {
            Self::Cargo => "cargo",
            Self::CargoNextest => "cargo-nextest",
            Self::MarkdownlintCli2 => "markdownlint-cli2",
            Self::Mdtablefix => "mdtablefix",
            Self::Nixie => "nixie",
            Self::PinnedCargo => "pinned-cargo",
            Self::Uv => "uv",
            Self::Whitaker => "whitaker",
        }
    }
}

/// Tools every gate in the manifest may call. Individual tests remove or
/// relocate entries to exercise missing-tool and `PATH`-prepend behaviour.
pub const DEFAULT_TOOLS: [FakeTool; 6] = [
    FakeTool::Cargo,
    FakeTool::Whitaker,
    FakeTool::Mdtablefix,
    FakeTool::MarkdownlintCli2,
    FakeTool::Uv,
    FakeTool::Nixie,
];

/// An environment variable a test may set for the Netsuke child.
#[derive(Clone, Copy, Debug)]
pub enum ChildVar {
    /// Extra Cargo arguments the build actions pass through.
    BuildJobs,
    /// Read by the manifest in place of `cargo`.
    Cargo,
    /// Makes a matching fake-tool invocation fail; see `FAKE_TOOL`.
    FailingCommand,
    /// Extra flags the manifest appends to `RUSTFLAGS`.
    RustFlags,
}

impl ChildVar {
    /// The variable's name in the child environment.
    const fn name(self) -> &'static str {
        match self {
            Self::BuildJobs => "BUILD_JOBS",
            Self::Cargo => "CARGO",
            Self::FailingCommand => "FAILING_COMMAND",
            Self::RustFlags => "RUST_FLAGS",
        }
    }
}

/// A log the fake tools append to.
#[derive(Clone, Copy, Debug)]
enum Log {
    Commands,
    Environment,
}

impl Log {
    /// The log's file name in the sandbox root.
    const fn file_name(self) -> &'static str {
        match self {
            Self::Commands => "commands.log",
            Self::Environment => "environment.log",
        }
    }
}

/// A real executable the tests take from the host.
#[derive(Clone, Copy, Debug)]
enum HostTool {
    Netsuke,
    Ninja,
}

impl HostTool {
    /// The executable name to look for on the host `PATH`.
    const fn executable(self) -> &'static str {
        match self {
            Self::Netsuke => "netsuke",
            Self::Ninja => "ninja",
        }
    }
}

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
    ninja: Utf8PathBuf,
    variables: Vec<(String, String)>,
}

impl ActionSandbox {
    /// Creates a sandbox holding the manifest and no tools.
    ///
    /// # Errors
    ///
    /// Returns an error when the temporary tree cannot be created or no
    /// `netsuke` or `ninja` executable is on the test process's `PATH`.
    pub fn empty() -> Result<Self> {
        let netsuke = host_executable(HostTool::Netsuke)?;
        let ninja = host_executable(HostTool::Ninja)?;
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
            ninja,
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
    pub fn install(&self, tool: FakeTool, home: ToolHome) -> Result<()> {
        let executable = tool.executable();
        let path = match home {
            ToolHome::Path => format!("tools/{executable}"),
            ToolHome::LocalBin => format!("home/.local/bin/{executable}"),
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
    pub fn set_var(&mut self, variable: ChildVar, value: &str) {
        self.variables
            .push((variable.name().to_owned(), value.to_owned()));
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
            .env("NETSUKE_NINJA", &self.ninja)
            .env("HOME", root.join("home"))
            .env("ACTION_LOG", root.join(Log::Commands.file_name()))
            .env("ENV_LOG", root.join(Log::Environment.file_name()))
            .envs(self.variables.iter().map(|(name, value)| (name, value)))
            .output()
            .context("run netsuke")
    }

    /// Returns each logged invocation as `tool<TAB>arg...`, in execution order.
    ///
    /// # Errors
    ///
    /// Returns an error when the log exists but cannot be read.
    pub fn commands(&self) -> Result<Vec<String>> { self.log_lines(Log::Commands) }

    /// Returns each logged `tool first-arg<TAB>NAME=value...` environment record.
    ///
    /// # Errors
    ///
    /// Returns an error when the log exists but cannot be read.
    pub fn environment(&self) -> Result<Vec<String>> { self.log_lines(Log::Environment) }

    /// Reads one sandbox log as lines; a log never written reads as empty.
    fn log_lines(&self, log: Log) -> Result<Vec<String>> {
        let name = log.file_name();
        if !self.directory.exists(name) {
            return Ok(Vec::new());
        }
        let text = self
            .directory
            .read_to_string(name)
            .with_context(|| format!("read {name}"))?;
        Ok(text.lines().map(str::to_owned).collect())
    }

    /// The sandbox root as a UTF-8 path.
    fn root_path(&self) -> Result<&Utf8Path> {
        Utf8Path::from_path(self.root.path()).context("sandbox path is not UTF-8")
    }
}

/// Finds an installed executable on the host `PATH` before the sandbox
/// narrows it.
///
/// # Errors
///
/// Returns an error naming the executable when no `PATH` entry holds it, since
/// these tests drive the real Netsuke and Ninja.
fn host_executable(tool: HostTool) -> Result<Utf8PathBuf> {
    let name = tool.executable();
    let path = env::var_os("PATH").unwrap_or_default();
    env::split_paths(&path)
        .map(|directory| directory.join(name))
        .find(|candidate| candidate.is_file())
        .and_then(|candidate| Utf8PathBuf::from_path_buf(candidate).ok())
        .with_context(|| format!("{name} must be on PATH to run the Netsukefile action tests"))
}
