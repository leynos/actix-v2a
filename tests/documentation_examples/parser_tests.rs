//! Parser rejection tests for the documentation-example loader.
//!
//! These live beside the loader but are declared only by the integration test
//! target, so the loader's parser invariants are exercised once rather than in
//! every test binary that shares the loader.

use anyhow::Result;
use rstest::rstest;

use crate::documentation_examples::{DocumentedExample, is_valid_example_id, parse_document};

const MARKED_FENCE: &str = "<!-- tested-example: example-id -->\n```rust\nfn main() {}\n```\n";

fn parse(contents: &str) -> Result<Vec<DocumentedExample>> {
    parse_document("fixture.md", contents)
}

fn error_message(contents: &str) -> String {
    match parse(contents) {
        Ok(examples) => panic!(
            "fixture should be rejected, but parsed {} example(s)",
            examples.len()
        ),
        Err(error) => format!("{error:#}"),
    }
}

#[test]
fn marked_fence_is_loaded_with_its_exact_body() {
    let examples = parse(MARKED_FENCE).expect("fixture should be valid");
    let example = examples.first().expect("fixture has one example");
    assert_eq!(example.id, "example-id");
    assert_eq!(example.language, "rust");
    assert_eq!(example.body, "fn main() {}\n");
}

#[test]
fn unmarked_fence_is_rejected() {
    let error = error_message("```rust\nfn main() {}\n```\n");
    assert!(
        error.contains("missing a tested-example marker"),
        "unexpected error: {error}"
    );
}

#[test]
fn marker_without_a_fence_is_rejected() {
    let error = error_message("<!-- tested-example: example-id -->\n");
    assert!(
        error.contains("marker has no fence"),
        "unexpected error: {error}"
    );
}

#[test]
fn fence_without_a_language_is_rejected() {
    let error = error_message("<!-- tested-example: example-id -->\n```\nbody\n```\n");
    assert!(
        error.contains("declare a language"),
        "unexpected error: {error}"
    );
}

#[test]
fn unterminated_fence_is_rejected() {
    let error = error_message("<!-- tested-example: example-id -->\n```rust\nfn main() {}\n");
    assert!(
        error.contains("fence is not terminated"),
        "unexpected error: {error}"
    );
}

#[test]
fn empty_identifier_marker_is_rejected() {
    let error = error_message("<!-- tested-example: -->\n```rust\nfn main() {}\n```\n");
    assert!(
        error.contains("identifier must not be empty"),
        "unexpected error: {error}"
    );
}

#[test]
fn whitespace_identifier_marker_is_rejected() {
    let error = error_message("<!-- tested-example:  -->\n```rust\nfn main() {}\n```\n");
    assert!(
        error.contains("identifier must not be empty"),
        "unexpected error: {error}"
    );
}

#[test]
fn duplicate_identifiers_are_rejected() {
    let duplicated = format!("{MARKED_FENCE}{MARKED_FENCE}");
    let error = error_message(&duplicated);
    assert!(
        error.contains("duplicate tested-example identifier 'example-id'"),
        "unexpected error: {error}"
    );
}

#[test]
fn document_without_fences_has_no_examples() {
    let examples = parse("# Heading\n\nProse only.\n").expect("prose is valid");
    assert!(examples.is_empty());
}

#[rstest]
#[case("example-id", true)]
#[case("guide-operator-log-output", true)]
#[case("a", true)]
#[case("", false)]
#[case("-leading", false)]
#[case("trailing-", false)]
#[case("double--hyphen", false)]
#[case("Uppercase", false)]
#[case("with space", false)]
#[case("under_score", false)]
fn identifier_grammar(#[case] id: &str, #[case] expected: bool) {
    assert_eq!(is_valid_example_id(id), expected, "identifier: {id:?}");
}
