//! Names the `Netsukefile` gates and the command lines each one issues.
//!
//! Tests speak in these types rather than raw action names and log lines, so a
//! case states which gate or failure it exercises and the expected commands
//! live beside the gate that owns them.

/// A gate action declared in the `Netsukefile`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gate {
    All,
    Build,
    CheckFmt,
    Clean,
    DocumentationContracts,
    Fmt,
    Lint,
    Markdownlint,
    Nixie,
    Release,
    Spelling,
    Test,
    Typecheck,
    WorkflowContracts,
}

impl Gate {
    /// The action name passed to `netsuke build`.
    pub const fn action(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Build => "build",
            Self::CheckFmt => "check-fmt",
            Self::Clean => "clean",
            Self::DocumentationContracts => "test-documentation-contracts",
            Self::Fmt => "fmt",
            Self::Lint => "lint",
            Self::Markdownlint => "markdownlint",
            Self::Nixie => "nixie",
            Self::Release => "release",
            Self::Spelling => "spelling",
            Self::Test => "test",
            Self::Typecheck => "typecheck",
            Self::WorkflowContracts => "test-workflow-contracts",
        }
    }

    /// Identifies the gate that issued a logged `tool<TAB>arg...` line.
    pub fn issuing(line: &str) -> Option<Self> {
        let mut words = line.split('\t');
        match (words.next(), words.next()) {
            (Some("uv"), _) if line.contains("typos-config-builder") => Some(Self::Spelling),
            (Some("uv"), _) if line.contains("cv005-contracts") => Some(Self::WorkflowContracts),
            (Some("cargo"), Some("fmt")) | (Some("mdtablefix"), _) => Some(Self::CheckFmt),
            (Some("cargo"), Some("doc" | "clippy")) | (Some("whitaker"), _) => Some(Self::Lint),
            (Some("cargo"), Some("nextest" | "test"))
                if line.contains("--test\tdocumentation_examples") =>
            {
                Some(Self::DocumentationContracts)
            }
            (Some("cargo"), Some("nextest" | "test")) => Some(Self::Test),
            _ => None,
        }
    }
}

/// The commands `check-fmt` runs, in order.
pub const CHECK_FMT: [&str; 2] = [
    "cargo\tfmt\t--all\t--\t--check",
    concat!(
        "mdtablefix\t--check\t--git\t--include-untracked",
        "\t--wrap\t--renumber\t--breaks\t--ellipsis\t--fences",
    ),
];

/// The commands `lint` runs, in order. Whitaker names the manifest by absolute path, because it
/// runs from the checkout's parent; `{manifest}` stands for that path.
pub const LINT: [&str; 3] = [
    "cargo\tdoc\t--no-deps",
    "cargo\tclippy\t--all-targets\t--all-features\t--\t-D\twarnings",
    "whitaker\t--manifest-path\t{manifest}\t--all\t--\t--all-targets\t--all-features",
];

/// The commands `lint` runs for a sandbox whose checkout holds the manifest at `manifest`.
pub fn lint_commands(manifest: &str) -> Vec<String> {
    LINT.iter()
        .map(|command| command.replace("{manifest}", manifest))
        .collect()
}

/// A gate whose commands do not depend on which tools are installed.
#[derive(Clone, Copy, Debug)]
pub struct Recipe {
    pub gate: Gate,
    /// The commands the gate runs, in order.
    pub commands: &'static [&'static str],
}

impl Recipe {
    /// `build` compiles debug artefacts.
    pub const BUILD: Self = Self {
        gate: Gate::Build,
        commands: &["cargo\tbuild"],
    };

    /// `release` compiles release artefacts.
    pub const RELEASE: Self = Self {
        gate: Gate::Release,
        commands: &["cargo\tbuild\t--release"],
    };

    /// `clean` removes build artefacts.
    pub const CLEAN: Self = Self {
        gate: Gate::Clean,
        commands: &["cargo\tclean"],
    };

    /// `fmt` formats Rust, then Markdown tables, then Markdown style.
    pub const FMT: Self = Self {
        gate: Gate::Fmt,
        commands: &[
            "cargo\tfmt\t--all",
            concat!(
                "mdtablefix\t--in-place\t--git\t--include-untracked",
                "\t--wrap\t--renumber\t--breaks\t--ellipsis\t--fences",
            ),
            "markdownlint-cli2\t--fix\t**/*.md",
        ],
    };
}

/// The command `markdownlint` runs after its `spelling` dependency.
pub const MARKDOWNLINT: &str = "markdownlint-cli2\t**/*.md";

/// The test runner the `test` action should select.
#[derive(Clone, Copy, Debug)]
pub enum Runner {
    /// `cargo-nextest` is installed: nextest, then doctests separately.
    Nextest,
    /// `cargo-nextest` is absent: `cargo test`, then doctests separately,
    /// because `--all-targets` excludes them.
    CargoTest,
}

impl Runner {
    /// Whether the sandbox should install a fake `cargo-nextest`.
    pub const fn has_nextest(self) -> bool { matches!(self, Self::Nextest) }

    /// The commands `test` should run with this runner, in order.
    pub const fn commands(self) -> &'static [&'static str] {
        match self {
            Self::Nextest => &[
                "cargo\tnextest\trun\t--all-targets\t--all-features",
                "cargo\ttest\t--doc\t--workspace\t--all-features",
            ],
            Self::CargoTest => &[
                "cargo\ttest\t--all-targets\t--all-features",
                "cargo\ttest\t--doc\t--workspace\t--all-features",
            ],
        }
    }

    /// The command `test-documentation-contracts` should run with this runner.
    ///
    /// The gate names both documentation targets, so it runs exactly those
    /// tests and reports them as a documentation failure rather than a
    /// generic test failure.
    pub const fn documentation_contracts_commands(self) -> &'static [&'static str] {
        match self {
            Self::Nextest => &[concat!(
                "cargo\tnextest\trun\t--test\tdocumentation_examples_tests",
                "\t--test\tdocumentation_examples_bdd",
            )],
            Self::CargoTest => &[concat!(
                "cargo\ttest\t--test\tdocumentation_examples_tests",
                "\t--test\tdocumentation_examples_bdd",
            )],
        }
    }
}

/// A `lint` step made to fail, and the steps that should have run by then.
#[derive(Clone, Copy, Debug)]
pub enum LintFailure {
    Rustdoc,
    Clippy,
}

impl LintFailure {
    /// The `FAILING_COMMAND` value that makes the fake tool fail this step.
    pub const fn failing_command(self) -> &'static str {
        match self {
            Self::Rustdoc => "cargo doc",
            Self::Clippy => "cargo clippy",
        }
    }

    /// The lint commands that run up to and including the failing step.
    pub const fn attempted(self) -> &'static [&'static str] {
        match self {
            Self::Rustdoc => LINT.split_at(1).0,
            Self::Clippy => LINT.split_at(2).0,
        }
    }
}

/// A gate that runs one pinned tool through `uv tool run`.
#[derive(Clone, Copy, Debug)]
pub struct UvGate {
    pub gate: Gate,
    /// Everything up to the pinned revision, which may move.
    pub prefix: &'static str,
    /// Everything after the pinned revision.
    pub suffix: &'static str,
}

impl UvGate {
    /// The spelling gate, run by `typos-config-builder`.
    pub const SPELLING: Self = Self {
        gate: Gate::Spelling,
        prefix: "uv\ttool\trun\t--python\t3.14\t--from\tgit+https://github.com/leynos/typos-config-builder.git@",
        suffix: "\ttypos-config-builder\tgate\t--repository\t.",
    };

    /// The CV-005 workflow contracts, run by `cv005-contracts`.
    pub const WORKFLOW_CONTRACTS: Self = Self {
        gate: Gate::WorkflowContracts,
        prefix:
            "uv\ttool\trun\t--python\t3.13\t--from\tgit+https://github.com/leynos/shared-actions@",
        suffix: "#subdirectory=packages/cv005-contracts\tcv005-contracts\tcheck\t--repository\t.",
    };

    /// Whether `commands` is exactly this gate's single pinned invocation.
    pub fn is_sole_invocation(self, commands: &[String]) -> bool {
        matches!(commands, [only] if only.starts_with(self.prefix) && only.ends_with(self.suffix))
    }
}
