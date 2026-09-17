//! Contract coverage ownership between pull requests and the main workflow.

const CI: &str = include_str!("../.github/workflows/ci.yml");
const COVERAGE_MAIN: &str = include_str!("../.github/workflows/coverage-main.yml");

#[test]
fn pull_request_ci_keeps_codescene_out_of_coverage() {
    assert!(CI.contains("with-ratchet: 'true'"));
    assert!(!CI.contains("upload-codescene-coverage"));
    assert!(!CI.contains("CS_ACCESS_TOKEN"));
    assert!(!CI.contains("cs-coverage"));
    assert!(!CI.contains("project-url:"));
    assert!(!CI.contains("fetch-depth: 0"));
}

#[test]
fn main_coverage_publishes_with_explicit_upload_mode() {
    let main = COVERAGE_MAIN;
    assert!(main.contains("branches: [main]"));
    assert!(main.contains("upload-codescene-coverage@152d9c4784d0ae5877938a984fe6d1f04d718fd8"));
    assert!(main.contains("mode: upload"));
}
