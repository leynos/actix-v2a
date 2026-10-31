//! Contract for the development build standard (the Concordat `rust-build-defaults` rule).
//!
//! Development, test, lint and typecheck builds use the parallel `rustc` frontend and, on Linux,
//! the `mold` linker; Cranelift is the development-profile backend; coverage and release take
//! neither flag. The `Netsukefile` recipes are proved by `netsuke_actions.rs`, which runs the real
//! `netsuke`; this contract holds the Cargo configuration, the pin and the workflows the recipes
//! do not reach.

/// Cargo configuration holding the flags, the backend and the footprint settings.
const CARGO_CONFIG: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/.cargo/config.toml"));
/// Toolchain file whose channel and components the standard constrains.
const TOOLCHAIN: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/rust-toolchain.toml"));
/// The pull-request workflow.
const CI: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/.github/workflows/ci.yml"
));
/// The `main` coverage workflow.
const COVERAGE_MAIN: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/.github/workflows/coverage-main.yml"
));

/// Returns the non-comment lines of the named TOML table, header included in the match.
fn table_lines<'a>(document: &'a str, header: &str) -> Vec<&'a str> {
    document
        .lines()
        .map(|line| line.split('#').next().unwrap_or("").trim())
        .skip_while(|line| *line != header)
        .skip(1)
        .take_while(|line| !line.starts_with('['))
        .filter(|line| !line.is_empty())
        .collect()
}

/// Returns the `rustflags` words of a table: every quoted string on its `rustflags` line.
fn rustflags(lines: &[&str]) -> Vec<String> {
    let Some(rest) = lines.iter().find_map(|line| line.strip_prefix("rustflags")) else {
        return Vec::new();
    };
    rest.split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

/// Turns a failed condition into the message that explains it.
fn require(holds: bool, message: impl FnOnce() -> String) -> Result<(), String> {
    if holds { Ok(()) } else { Err(message()) }
}

/// Scenario: the Cargo configuration's `rustflags` sources.
///
/// Invariant: `[build]` carries the frontend flag and no linker; the Linux table repeats the
/// frontend flag and adds `mold`, because Cargo selects one source rather than merging them.
#[test]
fn the_rustflags_sources_carry_the_frontend_everywhere_and_mold_on_linux_only() -> Result<(), String>
{
    let build = rustflags(&table_lines(CARGO_CONFIG, "[build]"));
    let linux = rustflags(&table_lines(
        CARGO_CONFIG,
        "[target.'cfg(target_os = \"linux\")']",
    ));
    require(build == ["-Zthreads=8"], || {
        format!("[build] rustflags must be exactly the frontend flag, found {build:?}")
    })?;
    require(linux == ["-Zthreads=8", "-Clink-arg=-fuse-ld=mold"], || {
        format!("the Linux table must repeat the frontend flag and add mold, found {linux:?}")
    })
}

/// Scenario: the development profile's backend.
///
/// Invariant: Cranelift is the default, and the unstable key it needs is enabled.
#[test]
fn cranelift_is_the_development_backend_behind_its_unstable_key() -> Result<(), String> {
    let unstable = table_lines(CARGO_CONFIG, "[unstable]");
    let profile = table_lines(CARGO_CONFIG, "[profile.dev]");
    require(unstable.contains(&"codegen-backend = true"), || {
        "[unstable] must enable codegen-backend".to_owned()
    })?;
    require(profile.contains(&"codegen-backend = \"cranelift\""), || {
        "[profile.dev] must select the Cranelift backend".to_owned()
    })
}

/// Scenario: the pinned toolchain.
///
/// Invariant: it carries the Cranelift component, which the development profile needs.
#[test]
fn the_pin_carries_the_cranelift_component() -> Result<(), String> {
    require(
        TOOLCHAIN.contains("rustc-codegen-cranelift-preview"),
        || "rust-toolchain.toml must list rustc-codegen-cranelift-preview".to_owned(),
    )
}

/// Returns the lines of every step in a workflow that uses an action.
fn steps_using<'a>(workflow: &'a str, action: &str) -> Vec<Vec<&'a str>> {
    let lines: Vec<&str> = workflow.lines().collect();
    let mut steps = Vec::new();
    for (at, line) in lines.iter().enumerate() {
        if line.trim_start().starts_with('#') || !line.contains(action) {
            continue;
        }
        let start = (0..=at)
            .rev()
            .find(|i| {
                lines
                    .get(*i)
                    .is_some_and(|l| l.trim_start().starts_with("- "))
            })
            .unwrap_or(at);
        let end = (at + 1..lines.len())
            .find(|i| {
                lines
                    .get(*i)
                    .is_some_and(|l| l.trim_start().starts_with("- "))
            })
            .unwrap_or(lines.len());
        steps.push(lines.get(start..end).unwrap_or_default().to_vec());
    }
    steps
}

/// Returns the complaints about workflow steps that lack a required line.
fn missing(workflow: &str, name: &str, action: &str, wanted: &str) -> Vec<String> {
    let steps = steps_using(workflow, action);
    if steps.is_empty() {
        return vec![format!("{name}: no {action} step to read")];
    }
    steps
        .iter()
        .filter(|step| !step.iter().any(|line| line.trim() == wanted))
        .map(|_| format!("{name}: a {action} step lacks `{wanted}`"))
        .collect()
}

/// Scenario: the workflows that build under the standard.
///
/// Invariant: every `setup-rust` step installs `mold`, every coverage step assigns a
/// `RUSTFLAGS` that names neither standard flag, and the Whitaker install provisions the
/// Cranelift component the lint suite's toolchain needs.
#[test]
fn the_workflows_install_the_linker_and_hold_coverage_out_of_the_standard() -> Result<(), String> {
    let mut problems = Vec::new();
    for (name, workflow) in [("ci.yml", CI), ("coverage-main.yml", COVERAGE_MAIN)] {
        problems.extend(missing(
            workflow,
            name,
            "setup-rust@",
            "install-mold: 'true'",
        ));
        problems.extend(missing(
            workflow,
            name,
            "generate-coverage@",
            "RUSTFLAGS: -D warnings",
        ));
    }
    problems.extend(missing(
        CI,
        "ci.yml",
        "install-whitaker@",
        "cranelift: 'true'",
    ));
    require(problems.is_empty(), || problems.join("\n"))
}

/// Scenario: steps that lack a required line.
///
/// Invariant: the reader reports a missing line and a missing step, so the check above cannot
/// pass by reading nothing.
#[test]
fn the_workflow_reader_reports_a_missing_line_and_a_missing_step() -> Result<(), String> {
    let workflow =
        "steps:\n  - uses: org/setup-rust@abc\n    with:\n      install-mold: 'true'\n  - uses: \
         org/setup-rust@def\n";
    let found = missing(workflow, "w.yml", "setup-rust@", "install-mold: 'true'");
    require(found.len() == 1, || {
        format!("one step lacks the line, found {found:?}")
    })?;
    let none = missing(
        workflow,
        "w.yml",
        "generate-coverage@",
        "RUSTFLAGS: -D warnings",
    );
    require(none.len() == 1, || {
        format!("a missing step must be reported, found {none:?}")
    })
}
