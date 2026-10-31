//! Property tests for the documented-example parser and identifier grammar.
//!
//! The parser is a small state machine over lines: it recognizes markers,
//! opening and closing fences, and body text. These properties state the
//! contract that state machine must satisfy for arbitrary documents:
//! a document it accepts round-trips to the examples it was built from,
//! unmarked or duplicate fences are always rejected, and the identifier
//! grammar rejects every disallowed character. Generated bodies exclude fence
//! characters, so each property isolates one part of the contract.

use proptest::{prelude::*, test_runner::TestCaseError};

use crate::documentation_examples::{is_valid_example_id, parse_document};

/// One example the parser should recover.
#[derive(Clone, Debug)]
struct Example {
    id: String,
    language: String,
    body: String,
}

/// An identifier the grammar accepts: a lowercase letter, then letters,
/// digits, and single hyphens, with no trailing hyphen.
fn identifier() -> impl Strategy<Value = String> {
    ("[a-z]", prop::collection::vec("[a-z0-9]{1,6}", 0..3)).prop_map(|(first, parts)| {
        parts.into_iter().fold(first, |mut id, part| {
            id.push('-');
            id.push_str(&part);
            id
        })
    })
}

/// A fence language: a lowercase letter followed by lowercase letters or
/// digits.
fn language() -> impl Strategy<Value = String> { "[a-z][a-z0-9]*".prop_map(String::from) }

/// One body line, excluding the characters that could close a fence.
fn body_line() -> impl Strategy<Value = String> {
    "[^\\x60\\x7e\\r\\n]{0,40}".prop_map(String::from)
}

/// An example whose rendered fence the parser must recover exactly.
fn example() -> impl Strategy<Value = Example> {
    (
        identifier(),
        language(),
        prop::collection::vec(body_line(), 0..5),
    )
        .prop_map(|(id, language, lines)| {
            let body = lines.into_iter().fold(String::new(), |mut body, line| {
                body.push_str(&line);
                body.push('\n');
                body
            });
            Example { id, language, body }
        })
}

/// Prose that cannot contain a marker or a fence.
fn prose() -> impl Strategy<Value = String> {
    prop::collection::vec("[^\\x60\\x7e<\\r\\n]{0,40}", 0..3).prop_map(|lines| {
        lines.into_iter().fold(String::new(), |mut prose, line| {
            prose.push_str(&line);
            prose.push('\n');
            prose
        })
    })
}

/// A line the parser treats as a fence: at most three spaces, then a run of
/// at least three backticks or tildes.
fn fence_line() -> impl Strategy<Value = String> {
    ("[ ]{0,3}", prop_oneof![Just('`'), Just('~')], 3_usize..7).prop_map(
        |(indentation, delimiter, length)| {
            format!("{indentation}{}", delimiter.to_string().repeat(length))
        },
    )
}

/// Renders one example as a marker followed by its fenced block.
fn render_example(example: &Example) -> String {
    format!(
        "<!-- tested-example: {} -->\n```{}\n{}```\n",
        example.id, example.language, example.body
    )
}

/// Renders `examples` as a document the parser should accept.
fn render(examples: &[Example]) -> String {
    let mut document = String::new();
    for example in examples {
        document.push_str(&render_example(example));
    }
    document
}

/// Keeps the first example for each identifier, so the document is valid.
fn with_unique_identifiers(examples: Vec<Example>) -> Vec<Example> {
    let mut seen = std::collections::HashSet::new();
    examples
        .into_iter()
        .filter(|example| seen.insert(example.id.clone()))
        .collect()
}

proptest! {
    /// A generated document parses back to exactly the examples it renders.
    #[test]
    fn generated_documents_round_trip(examples in prop::collection::vec(example(), 0..6)) {
        let unique_examples = with_unique_identifiers(examples);
        let parsed = parse_document("generated.md", &render(&unique_examples))
            .map_err(|error| TestCaseError::fail(format!("generated document rejected: {error:#}")))?;
        prop_assert_eq!(parsed.len(), unique_examples.len());
        for (actual, expected) in parsed.iter().zip(&unique_examples) {
            prop_assert_eq!(&actual.id, &expected.id);
            prop_assert_eq!(&actual.language, &expected.language);
            prop_assert_eq!(&actual.body, &expected.body);
        }
    }

    /// An unmarked fence is rejected wherever it appears in prose.
    #[test]
    fn unmarked_fences_are_rejected(
        before in prose(),
        fence in fence_line(),
        after in prose(),
    ) {
        let contents = format!("{before}{fence}\n{after}");
        prop_assert!(parse_document("generated.md", &contents).is_err());
    }

    /// Two markers naming the same identifier are always rejected.
    #[test]
    fn duplicate_identifiers_are_rejected(first in example(), second in example()) {
        let identifier = first.id.clone();
        let contents = render(&[
            Example { id: identifier.clone(), ..first },
            Example { id: identifier, ..second },
        ]);
        let error = match parse_document("generated.md", &contents) {
            Ok(_) => String::new(),
            Err(error) => format!("{error:#}"),
        };
        prop_assert!(
            error.contains("duplicate tested-example identifier"),
            "expected a duplicate-identifier error, got {error:?}"
        );
    }

    /// A marker whose identifier contains a disallowed character is rejected
    /// by both the grammar and the parser.
    #[test]
    fn disallowed_identifier_characters_are_rejected(
        prefix in "[a-z]{0,5}",
        disallowed in "[A-Z_/ .]",
        suffix in "[a-z0-9-]{0,5}",
    ) {
        let id = format!("{prefix}{disallowed}{suffix}");
        prop_assert!(!is_valid_example_id(&id));
        let contents = render(&[Example {
            id: id.clone(),
            language: "rust".to_owned(),
            body: "fn main() {}\n".to_owned(),
        }]);
        prop_assert!(
            parse_document("generated.md", &contents).is_err(),
            "marker with identifier {id:?} should be rejected"
        );
    }
}
