//! Loads fenced examples from the published user-facing documentation.
//!
//! Every fence in a registered document must sit immediately below a stable
//! `tested-example` marker. Loading fails on an unmarked fence, an
//! unterminated fence, an empty identifier, or a duplicate identifier, so a
//! documented example cannot escape the test suite. Integration tests and
//! behaviour-driven scenarios share this loader and therefore exercise the
//! shipped Markdown rather than a copied fixture.

use std::{collections::HashSet, sync::LazyLock};

use anyhow::{Context, Result, ensure};
use cap_std::{ambient_authority, fs::Dir};

const DOCUMENT_PATHS: &[&str] = &["README.md", "docs/users-guide.md"];
const MARKER_PREFIX: &str = "<!-- tested-example: ";
const MARKER_SUFFIX: &str = " -->";

static DOCUMENTED_EXAMPLES: LazyLock<Result<Vec<DocumentedExample>, String>> =
    LazyLock::new(|| read_documented_examples().map_err(|error| format!("{error:#}")));

#[derive(Clone, Copy, Eq, PartialEq)]
struct Fence {
    delimiter: u8,
    length: usize,
}

#[derive(Clone, Copy)]
struct Cursor {
    source: &'static str,
    line_index: usize,
}

impl Cursor {
    fn error(self, message: &str) -> String {
        format!("{}:{} {message}", self.source, self.line_index + 1)
    }
}

/// One marked fenced example loaded from user-facing documentation.
#[derive(Debug, Eq, PartialEq)]
pub struct DocumentedExample {
    /// Stable identifier declared by the `tested-example` marker.
    pub id: String,
    /// Markdown fence language, such as `rust`, `sh`, or `plaintext`.
    pub language: String,
    /// Exact text inside the fence, including a trailing newline.
    pub body: String,
}

/// Load every registered example once and return the cached registry.
///
/// # Errors
///
/// Returns an error when a document cannot be read, a marker is malformed, a
/// fence is unmarked or unterminated, or an identifier is duplicated.
pub fn load_documented_examples() -> Result<&'static [DocumentedExample]> {
    DOCUMENTED_EXAMPLES
        .as_ref()
        .map(Vec::as_slice)
        .map_err(|message| anyhow::anyhow!(message.clone()))
}

/// Borrow the cached documented example identified by `id`.
///
/// # Errors
///
/// Returns an error when the documents are invalid or `id` is absent.
pub fn documented_example(id: &str) -> Result<&'static DocumentedExample> {
    load_documented_examples()?
        .iter()
        .find(|example| example.id == id)
        .with_context(|| format!("documented example '{id}' should exist"))
}

/// Return whether an identifier is safe for documentation-workspace paths.
///
/// Identifiers start with a lowercase ASCII letter, then use lowercase ASCII
/// letters, digits, or single hyphens. Validate before interpolating an
/// identifier into a path.
pub(crate) fn is_valid_example_id(id: &str) -> bool {
    id.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && !id.ends_with('-')
        && !id.contains("--")
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

/// Parse one document, rejecting any fence that is not correctly marked.
///
/// # Errors
///
/// Returns an error when a marker is malformed, a fence is unmarked or
/// unterminated, a fence omits its language, or an identifier is duplicated.
pub(crate) fn parse_document(
    source: &'static str,
    contents: &str,
) -> Result<Vec<DocumentedExample>> {
    // `lines()` drops source terminators. `read_fence_body` restores one LF
    // per body line, keeping the registry's canonical representation.
    let mut lines = contents.lines().enumerate();
    let mut examples = Vec::new();
    let mut ids = HashSet::new();

    while let Some((line_index, line)) = lines.next() {
        let cursor = Cursor { source, line_index };
        if let Some(id) = parse_marker(line) {
            validate_identifier(cursor, id)?;
            ensure!(ids.insert(id), "duplicate tested-example identifier '{id}'");
            examples.push(read_marked_example(&cursor, id, &mut lines)?);
        } else {
            reject_invalid_example_line(cursor, line)?;
        }
    }

    Ok(examples)
}

fn validate_identifier(cursor: Cursor, id: &str) -> Result<()> {
    ensure!(
        !id.trim().is_empty(),
        "{}",
        cursor.error("tested-example identifier must not be empty")
    );
    ensure!(
        is_valid_example_id(id),
        "{}",
        cursor.error(
            "tested-example identifier must use lowercase letters, digits, and single hyphens"
        )
    );
    Ok(())
}

fn read_documented_examples() -> Result<Vec<DocumentedExample>> {
    let repository = repository_directory()?;
    let mut examples = Vec::new();
    for path in DOCUMENT_PATHS {
        let contents = repository
            .read_to_string(path)
            .with_context(|| format!("read {path}"))?;
        examples.extend(parse_document(path, &contents)?);
    }

    let mut ids = HashSet::new();
    for example in &examples {
        ensure!(
            ids.insert(example.id.as_str()),
            "duplicate tested-example identifier '{}'",
            example.id
        );
    }
    Ok(examples)
}

fn repository_directory() -> Result<Dir> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    Dir::open_ambient_dir(root, ambient_authority()).context("open the repository root")
}

fn reject_invalid_example_line(cursor: Cursor, line: &str) -> Result<()> {
    ensure!(
        line != format!("{MARKER_PREFIX}{}", MARKER_SUFFIX.trim_start()),
        "{}",
        cursor.error("tested-example identifier must not be empty")
    );
    ensure!(
        parse_fence(line).is_none(),
        "{}",
        cursor.error("fence is missing a tested-example marker")
    );
    Ok(())
}

fn read_marked_example<'a>(
    cursor: &Cursor,
    id: &str,
    lines: &mut impl Iterator<Item = (usize, &'a str)>,
) -> Result<DocumentedExample> {
    let (fence_index, fence) = lines
        .next()
        .with_context(|| cursor.error("marker has no fence"))?;
    let fence_cursor = Cursor {
        source: cursor.source,
        line_index: fence_index,
    };
    let (opening_fence, language) = parse_fence(fence)
        .with_context(|| fence_cursor.error("expected an opening fence after marker"))?;
    ensure!(
        !language.is_empty(),
        "{}",
        fence_cursor.error("fence should declare a language")
    );
    let body = read_fence_body(cursor.source, fence_index, opening_fence, lines)?;
    Ok(DocumentedExample {
        id: id.to_owned(),
        language: language.to_owned(),
        body,
    })
}

fn parse_marker(line: &str) -> Option<&str> {
    line.strip_prefix(MARKER_PREFIX)
        .and_then(|value| value.strip_suffix(MARKER_SUFFIX))
}

fn parse_fence(line: &str) -> Option<(Fence, &str)> {
    let indentation = line.bytes().take_while(|byte| *byte == b' ').count();
    if indentation > 3 {
        return None;
    }
    let remainder = line.get(indentation..)?;
    let delimiter = *remainder.as_bytes().first()?;
    if !matches!(delimiter, b'`' | b'~') {
        return None;
    }
    let length = remainder
        .bytes()
        .take_while(|candidate| *candidate == delimiter)
        .count();
    let language = remainder.get(length..)?;
    (length >= 3).then_some((Fence { delimiter, length }, language))
}

fn is_matching_closing_fence(line: &str, opening_fence: Fence) -> bool {
    let Some((closing_fence, suffix)) = parse_fence(line) else {
        return false;
    };
    can_close_fence(closing_fence, opening_fence) && has_no_info_string(suffix)
}

/// A closing fence uses the opening delimiter and is at least as long.
const fn can_close_fence(closing_fence: Fence, opening_fence: Fence) -> bool {
    closing_fence.delimiter == opening_fence.delimiter
        && closing_fence.length >= opening_fence.length
}

/// Only spaces and tabs may follow a closing fence's delimiter run.
fn has_no_info_string(suffix: &str) -> bool {
    suffix.bytes().all(|byte| matches!(byte, b' ' | b'\t'))
}

fn read_fence_body<'a>(
    source: &'static str,
    fence_index: usize,
    opening_fence: Fence,
    lines: &mut impl Iterator<Item = (usize, &'a str)>,
) -> Result<String> {
    let mut body = String::new();
    for (_, line) in lines {
        if is_matching_closing_fence(line, opening_fence) {
            return Ok(body);
        }
        body.push_str(line);
        body.push('\n');
    }
    anyhow::bail!("{source}:{} fence is not terminated", fence_index + 1)
}
