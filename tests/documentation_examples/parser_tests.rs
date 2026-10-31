//! Parser rejection tests for the documentation-example loader.
//!
//! These live beside the loader but are declared only by the integration test
//! target, so the loader's parser invariants are exercised once rather than in
//! every test binary that shares the loader.

use anyhow::Result;
use rstest::rstest;

use crate::documentation_examples::{DocumentedExample, is_valid_example_id, parse_document};

const MARKED_FENCE: &str = "<!-- tested-example: example-id -->\n```rust\nfn main() {}\n```\n";

/// Two marked fences that reuse the same identifier.
const DUPLICATED_FENCES: &str = concat!(
    "<!-- tested-example: example-id -->\n",
    "```rust\nfn main() {}\n```\n",
    "<!-- tested-example: example-id -->\n",
    "```rust\nfn main() {}\n```\n",
);

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

#[rstest]
#[case::unmarked_fence("```rust\nfn main() {}\n```\n", "missing a tested-example marker")]
#[case::marker_without_a_fence("<!-- tested-example: example-id -->\n", "marker has no fence")]
#[case::fence_without_a_language(
    "<!-- tested-example: example-id -->\n```\nbody\n```\n",
    "declare a language"
)]
#[case::unterminated_fence(
    "<!-- tested-example: example-id -->\n```rust\nfn main() {}\n",
    "fence is not terminated"
)]
#[case::empty_identifier(
    "<!-- tested-example: -->\n```rust\nfn main() {}\n```\n",
    "identifier must not be empty"
)]
#[case::whitespace_identifier(
    "<!-- tested-example:  -->\n```rust\nfn main() {}\n```\n",
    "identifier must not be empty"
)]
#[case::duplicate_identifier(DUPLICATED_FENCES, "duplicate tested-example identifier 'example-id'")]
fn malformed_documents_are_rejected(#[case] contents: &str, #[case] expected: &str) {
    let error = error_message(contents);
    assert!(error.contains(expected), "unexpected error: {error}");
}

#[test]
fn longer_closing_fence_terminates_the_block() {
    let contents = "<!-- tested-example: example-id -->\n````rust\nfn main() {}\n`````\n";
    let examples = parse(contents).expect("a longer closing fence should terminate the block");
    let example = examples.first().expect("fixture has one example");
    assert_eq!(example.body, "fn main() {}\n");
}

#[test]
fn shorter_inner_fence_does_not_close_the_block() {
    let contents = "<!-- tested-example: example-id -->\n````rust\n```\nfn main() {}\n````\n";
    let examples = parse(contents).expect("an inner shorter fence should remain body text");
    let example = examples.first().expect("fixture has one example");
    assert_eq!(example.body, "```\nfn main() {}\n");
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
