//! The CV-005 judgements on the push-to-`main` publisher.
//!
//! Split from `rules.rs` under the repository's 400-line cap. As there, each
//! function returns the reasons a workflow fails, so a fixture case can assert
//! which clause fired.

use serde_yaml::{Mapping, Value};

use super::{
    reader::{self, get},
    rules::{
        ACCESS_TOKEN,
        COVERAGE_CLI,
        input_is,
        input_str,
        is_coverage,
        is_upload,
        is_upload_action,
        normalized,
        runs_the_cli,
    },
    text::{computes_a_secret, rendered, rendered_mapping},
};

/// The expression that hands a step the secret itself.
///
/// The publisher's placement clauses look for this rather than the bare name,
/// because the upload's own guard names `env.CS_ACCESS_TOKEN` without holding
/// anything: a step whose `env` lost the secret would otherwise still read as
/// receiving it through its `if:`.
const SECRET_REFERENCE: &str = "secrets.CS_ACCESS_TOKEN";
/// The upload step's `env` binding of the token, whitespace normalized.
const TOKEN_BINDING: &str = "${{ secrets.CS_ACCESS_TOKEN }}";
/// The upload action's `access-token` input, whitespace normalized.
const TOKEN_INPUT: &str = "${{ env.CS_ACCESS_TOKEN }}";
/// The conjunct that restricts the publisher's upload to the trunk.
const MAIN_REF_GUARD: &str = "github.ref == 'refs/heads/main'";

/// Returns whether a workflow is triggered by a push restricted to `main`.
///
/// A push with no branch filter is not a main publisher: it fires on every
/// branch, so the baseline it writes would be whichever branch pushed last.
/// A tag filter fails too, since it names no branch at all.
pub fn publishes_from_main(workflow: &Value) -> bool {
    reader::trigger(workflow, "push")
        .and_then(Value::as_mapping)
        .and_then(|push| get(push, "branches"))
        .and_then(Value::as_sequence)
        .is_some_and(|branches| {
            !branches.is_empty()
                && branches
                    .iter()
                    .all(|branch| branch.as_str() == Some("main"))
        })
}

/// Blanks every character inside a single-quoted literal, keeping the quotes.
///
/// Blanked character for character, so the result has as many characters as
/// the input and a position found in one is the same position in the other.
fn unquoted(body: &str) -> String {
    let mut in_quote = false;
    body.chars()
        .map(|character| {
            if character == '\'' {
                in_quote = !in_quote;
                character
            } else if in_quote {
                ' '
            } else {
                character
            }
        })
        .collect()
}

/// Returns the conjuncts of an `if:` condition, or `None` if it has a `||`.
///
/// Quoted literals are blanked before the operators are looked for, so a
/// `||` inside a string does not count and an `&&` inside one does not
/// split. A disjunction anywhere makes every conjunct optional, which is why
/// it is refused rather than parsed: `... && ref == main || dispatch` passes
/// any substring search for the ref and uploads a dispatch from any branch.
pub fn conjuncts(condition: &str) -> Option<Vec<String>> {
    let trimmed = condition.trim();
    let body = trimmed
        .strip_prefix("${{")
        .and_then(|inner| inner.strip_suffix("}}"))
        .unwrap_or(trimmed);
    let blanked = unquoted(body);
    if blanked.contains("||") {
        return None;
    }
    let mut parts = vec![String::new()];
    let mut pending_ampersand = false;
    for (original, seen) in body.chars().zip(blanked.chars()) {
        if seen == '&' && !pending_ampersand {
            pending_ampersand = true;
            continue;
        }
        if seen == '&' {
            parts.push(String::new());
        } else if let Some(part) = parts.last_mut() {
            if pending_ampersand {
                part.push('&');
            }
            part.push(original);
        }
        pending_ampersand = false;
    }
    Some(parts.iter().map(|part| normalized(part)).collect())
}

/// Returns whether an upload step's condition confines it to `main`.
fn guarded_to_main(step: &Mapping) -> bool {
    get(step, "if")
        .and_then(Value::as_str)
        .and_then(conjuncts)
        .is_some_and(|parts| parts.iter().any(|part| part == MAIN_REF_GUARD))
}

/// Returns whether a step binds the token in its own `env`, as the secret.
///
/// Asserted positively because the upload's guard, `env.CS_ACCESS_TOKEN !=
/// ''`, reads a missing binding as an empty string: with the binding deleted
/// the guard is simply false, the upload skips forever, and nothing fails.
fn binds_the_token(step: &Mapping) -> bool {
    get(step, "env")
        .and_then(Value::as_mapping)
        .and_then(|env| get(env, ACCESS_TOKEN))
        .and_then(Value::as_str)
        .is_some_and(|value| normalized(value) == TOKEN_BINDING)
}

/// Returns the reasons the token is declared in a scope wider than one step.
fn wide_token_findings(workflow: &Value) -> Vec<String> {
    let mut findings = Vec::new();
    if workflow
        .as_mapping()
        .and_then(|root| get(root, "env"))
        .is_some_and(|env| rendered(env).contains(SECRET_REFERENCE))
    {
        findings.push(format!(
            "the publisher declares {ACCESS_TOKEN} for every job"
        ));
    }
    for (id, job) in reader::jobs(workflow) {
        if get(job, "env").is_some_and(|env| rendered(env).contains(SECRET_REFERENCE)) {
            findings.push(format!("job {id} declares {ACCESS_TOKEN} for every step"));
        }
        if forwards_the_token(job) {
            findings.push(format!(
                "job {id} forwards {ACCESS_TOKEN} to a reusable workflow"
            ));
        }
    }
    findings
}

/// Returns the reasons the token reaches somewhere other than the upload.
///
/// "Some step has the token" proves nothing: moving it to the coverage step
/// satisfies that while the upload's own guard goes false and publishing
/// silently stops. So the uploading step must bind it, no other step may
/// hold it, and no wider scope may declare it.
fn token_findings(workflow: &Value) -> Vec<String> {
    let mut findings = wide_token_findings(workflow);
    if computes_a_secret(&rendered(workflow)) {
        findings.push("the publisher reaches a secret by a computed name".to_owned());
    }
    for step in reader::steps(workflow) {
        let holds = rendered_mapping(step).contains(SECRET_REFERENCE);
        if is_upload(step) && !binds_the_token(step) {
            findings.push(format!(
                "the upload step does not bind {ACCESS_TOKEN} in its env"
            ));
        }
        if !is_upload(step) && holds {
            findings.push(format!(
                "a step other than the upload receives {ACCESS_TOKEN}"
            ));
        }
    }
    findings
}

/// Returns whether a job calling a reusable workflow hands it the token.
///
/// Such a job has no steps, so the step clauses never see it: the token can
/// travel through its `with:` inputs, a named `secrets:` entry, or
/// `secrets: inherit`, and the called workflow then holds it outside the one
/// upload step the publisher is allowed.
fn forwards_the_token(job: &Mapping) -> bool {
    if get(job, "uses").is_none() {
        return false;
    }
    let inherits = get(job, "secrets").and_then(Value::as_str) == Some("inherit");
    let names_it = ["with", "secrets"]
        .iter()
        .filter_map(|key| get(job, key))
        .any(|value| rendered(value).contains(SECRET_REFERENCE));
    inherits || names_it
}

/// Returns the reasons the publisher's runs could cancel one another.
///
/// A cancelled publisher abandons both its upload and its baseline write, so
/// runs share a group that never cancels the run in progress: a newer push
/// replaces a pending run rather than queueing behind it, and the newest
/// baseline wins. Any `cancel-in-progress` other than an absent key or a
/// literal `false` is refused, an expression included: the question is
/// whether a push to `main` can ever be cancelled, and only the literal
/// answers it without evaluation.
fn concurrency_findings(workflow: &Value) -> Vec<String> {
    let mut findings = Vec::new();
    let group = workflow
        .as_mapping()
        .and_then(|root| get(root, "concurrency"));
    if group.is_none() {
        findings.push("the publisher declares no concurrency group".to_owned());
    }
    let job_groups = reader::jobs(workflow)
        .into_iter()
        .filter_map(|(_, job)| get(job, "concurrency"));
    for concurrency in group.into_iter().chain(job_groups) {
        let cancel = concurrency
            .as_mapping()
            .and_then(|mapping| get(mapping, "cancel-in-progress"));
        if cancel.is_some_and(|value| value.as_bool() != Some(false)) {
            findings.push("the publisher may cancel a run in progress".to_owned());
        }
    }
    findings
}

/// Returns the reasons the publisher's required work might never run.
///
/// A requirement met by a step that cannot run is not met: a ratcheted
/// coverage step or an upload inside a job carrying `if: false` reads as
/// present to every other clause. So no publisher job may carry a condition,
/// the ratcheted coverage step must carry none, and the upload is read only
/// from the action, never from a `run` body, where `false && cs-coverage
/// upload` still contains the command.
fn reachability_findings(workflow: &Value) -> Vec<String> {
    let mut findings: Vec<String> = reader::jobs(workflow)
        .into_iter()
        .filter(|(_, job)| get(job, "if").is_some())
        .map(|(id, _)| format!("publisher job {id} carries an `if:`"))
        .collect();
    let steps = reader::steps(workflow);
    if !steps.iter().any(|step| {
        is_coverage(step) && input_is(step, "with-ratchet", true) && get(step, "if").is_none()
    }) {
        findings.push("the main publisher generates no ratcheted coverage".to_owned());
    }
    if steps.iter().any(|step| runs_the_cli(step)) {
        findings.push(format!(
            "the publisher runs {COVERAGE_CLI} directly rather than through the action"
        ));
    }
    findings
}

/// Returns the reasons a main publisher fails to publish what CV-005 requires.
pub fn publisher_findings(workflow: &Value) -> Vec<String> {
    let mut findings = reachability_findings(workflow);
    let steps = reader::steps(workflow);
    let uploads: Vec<_> = steps.iter().filter(|step| is_upload(step)).collect();
    if uploads.is_empty() {
        findings.push("the main publisher uploads nothing to CodeScene".to_owned());
    }
    if uploads.iter().any(|step| !guarded_to_main(step)) {
        findings.push(format!(
            "an upload step is not guarded by `{MAIN_REF_GUARD}`"
        ));
    }
    findings.extend(token_findings(workflow));
    findings.extend(concurrency_findings(workflow));
    findings
}

/// Returns the reasons the publisher's upload would not send what it measured.
///
/// Kept apart from [`publisher_findings`] because it compares two steps of a
/// complete publisher: each upload must read the file, in the format, that a
/// coverage step writes, or it uploads nothing useful while every other
/// clause passes; and it must pass the token its step binds as its
/// `access-token`, or its own guard holds while the action runs
/// unauthenticated.
pub fn wiring_findings(workflow: &Value) -> Vec<String> {
    let steps = reader::steps(workflow);
    let written: Vec<(Option<&str>, Option<&str>)> = steps
        .iter()
        .filter(|step| is_coverage(step))
        .map(|step| (input_str(step, "output-path"), input_str(step, "format")))
        .collect();
    let mut findings = Vec::new();
    for upload in steps.iter().filter(|step| is_upload_action(step)) {
        let read = (input_str(upload, "path"), input_str(upload, "format"));
        if !written.contains(&read) {
            findings.push(format!(
                "the upload reads {read:?}, which no coverage step writes; written: {written:?}"
            ));
        }
        let token = input_str(upload, "access-token").map(normalized);
        if token.as_deref() != Some(TOKEN_INPUT) {
            findings.push(format!(
                "the upload's access-token is {token:?}, not the token its step binds"
            ));
        }
    }
    findings
}
