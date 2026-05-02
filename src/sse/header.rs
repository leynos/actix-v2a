//! Framework-agnostic HTTP header values used by SSE helpers.

/// Plain HTTP header name and value pair for SSE domain operations.
///
/// This type keeps the SSE domain helpers independent of any concrete web
/// framework while still modelling repeated header fields when stored in a
/// `Vec<SseHeader>`.
///
/// # Examples
///
/// ```
/// use actix_v2a::SseHeader;
///
/// let header = SseHeader::new("Last-Event-ID", "evt-123");
/// assert_eq!(header.name(), "Last-Event-ID");
/// assert_eq!(header.value(), "evt-123");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseHeader {
    name: String,
    value: String,
}

impl SseHeader {
    /// Construct a framework-agnostic HTTP header pair.
    #[must_use]
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }

    /// Borrow the header name.
    #[must_use]
    pub fn name(&self) -> &str { &self.name }

    /// Borrow the header value.
    #[must_use]
    pub fn value(&self) -> &str { &self.value }

    pub(crate) fn has_name(&self, expected: &str) -> bool {
        self.name.eq_ignore_ascii_case(expected)
    }
}
