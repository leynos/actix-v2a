//! Opaque cursor encoding and decoding helpers.
//!
//! A cursor is a base64url-encoded JSON representation of one ordering key and
//! a pagination direction. The token is URL-safe and opaque to clients, but it
//! is not signed or encrypted. Do not place secrets in cursor keys, and treat
//! decoded cursor data as caller-controlled input.

use base64::{
    Engine as _,
    engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;

const MAX_CURSOR_TOKEN_LEN: usize = 8 * 1024;

/// Direction of pagination relative to the cursor.
///
/// Indicates whether the cursor represents a position for fetching
/// the next page (forward in sort order) or the previous page
/// (backward in sort order).
///
/// # Examples
///
/// ```
/// use actix_v2a::pagination::Direction;
///
/// let forward = Direction::Next;
/// let backward = Direction::Prev;
///
/// assert_ne!(forward, backward);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    /// Forward in the sort order (for example, newer items if sorting ascending).
    #[default]
    Next,
    /// Backward in the sort order (for example, older items).
    Prev,
}

/// Cursor wrapper for an ordered boundary key with direction.
///
/// The encoded representation is base64url JSON and must be treated as opaque
/// by clients. The direction indicates whether this cursor is meant for
/// fetching the next page (forward) or previous page (backward).
///
/// # Example
///
/// ```
/// use actix_v2a::pagination::{Cursor, Direction};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// struct UserKey {
///     created_at: String,
///     id: String,
/// }
///
/// let cursor = Cursor::new(UserKey {
///     created_at: "2026-03-22T10:30:00Z".to_owned(),
///     id: "8b116c56-0a58-4c55-b7d7-06ee6bbddb8c".to_owned(),
/// });
/// let encoded = cursor.encode().expect("cursor encoding succeeds");
/// let decoded = Cursor::<UserKey>::decode(&encoded).expect("cursor decoding succeeds");
///
/// assert_eq!(decoded.key(), cursor.key());
/// assert_eq!(decoded.direction(), Direction::Next);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor<Key> {
    key: Key,
    #[serde(default)]
    dir: Direction,
}

/// Errors raised while encoding or decoding opaque cursors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CursorError {
    /// The cursor key could not be serialized into JSON.
    #[error("cursor JSON serialization failed: {message}")]
    Serialize {
        /// Human-readable serialization failure details.
        message: String,
    },
    /// The encoded token was not valid base64url.
    #[error("cursor is not valid base64url: {message}")]
    InvalidBase64 {
        /// Human-readable base64 decoding failure details.
        message: String,
    },
    /// The encoded token exceeded the accepted input size.
    #[error("cursor token exceeds maximum length of {max_len} bytes")]
    TokenTooLong {
        /// The maximum accepted cursor token length in bytes.
        max_len: usize,
    },
    /// The decoded JSON payload did not match the expected key shape.
    #[error("cursor JSON deserialization failed: {message}")]
    Deserialize {
        /// Human-readable deserialization failure details.
        message: String,
    },
}

impl<Key> Cursor<Key> {
    /// Construct a cursor from one ordering key with the default direction (`Next`).
    ///
    /// # Examples
    ///
    /// ```
    /// use actix_v2a::pagination::{Cursor, Direction};
    ///
    /// let cursor = Cursor::new("my-key");
    /// assert_eq!(cursor.direction(), Direction::Next);
    /// ```
    #[must_use]
    pub const fn new(key: Key) -> Self {
        Self {
            key,
            dir: Direction::Next,
        }
    }

    /// Construct a cursor from one ordering key with an explicit direction.
    ///
    /// # Examples
    ///
    /// ```
    /// use actix_v2a::pagination::{Cursor, Direction};
    ///
    /// let cursor = Cursor::with_direction("my-key", Direction::Prev);
    /// assert_eq!(cursor.direction(), Direction::Prev);
    /// ```
    #[must_use]
    pub const fn with_direction(key: Key, dir: Direction) -> Self { Self { key, dir } }

    /// Borrow the cursor key.
    #[must_use]
    pub const fn key(&self) -> &Key { &self.key }

    /// Access the pagination direction.
    #[must_use]
    pub const fn direction(&self) -> Direction { self.dir }

    /// Consume the cursor and return the inner key.
    #[must_use]
    pub fn into_inner(self) -> Key { self.key }

    /// Decompose the cursor into its constituent parts (key and direction).
    ///
    /// # Examples
    ///
    /// ```
    /// use actix_v2a::pagination::{Cursor, Direction};
    ///
    /// let cursor = Cursor::with_direction("my-key", Direction::Prev);
    /// let (key, dir) = cursor.into_parts();
    /// assert_eq!(key, "my-key");
    /// assert_eq!(dir, Direction::Prev);
    /// ```
    #[must_use]
    pub fn into_parts(self) -> (Key, Direction) { (self.key, self.dir) }
}

impl<Key> Cursor<Key>
where
    Key: Serialize,
{
    /// Encode the cursor to an opaque base64url JSON token.
    ///
    /// # Errors
    ///
    /// Returns [`CursorError::Serialize`] when the cursor key cannot be
    /// serialized into JSON.
    pub fn encode(&self) -> Result<String, CursorError> {
        let payload = serde_json::to_vec(self).map_err(|error| CursorError::Serialize {
            message: error.to_string(),
        })?;
        Ok(URL_SAFE_NO_PAD.encode(payload))
    }
}

impl<Key> Cursor<Key>
where
    Key: DeserializeOwned,
{
    /// Decode a cursor from an opaque base64url JSON token.
    ///
    /// # Errors
    ///
    /// Returns [`CursorError::InvalidBase64`] when `value` is not valid
    /// base64url and [`CursorError::Deserialize`] when the decoded JSON does
    /// not match the expected cursor shape.
    pub fn decode(value: &str) -> Result<Self, CursorError> {
        if value.len() > MAX_CURSOR_TOKEN_LEN {
            return Err(CursorError::TokenTooLong {
                max_len: MAX_CURSOR_TOKEN_LEN,
            });
        }

        let payload = URL_SAFE_NO_PAD
            .decode(value)
            .or_else(|_| URL_SAFE.decode(value))
            .map_err(|error| CursorError::InvalidBase64 {
                message: error.to_string(),
            })?;
        serde_json::from_slice(&payload).map_err(|error| CursorError::Deserialize {
            message: error.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for opaque cursor encoding and decoding.

    use base64::Engine as _;
    use rstest::{fixture, rstest};
    use serde::{Deserialize, Serialize, Serializer};

    use super::{Cursor, CursorError, Direction, MAX_CURSOR_TOKEN_LEN};

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct FixtureKey {
        created_at: String,
        id: String,
    }

    struct FailingKey;

    impl Serialize for FailingKey {
        fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            Err(serde::ser::Error::custom("fixture serialization failed"))
        }
    }

    #[fixture]
    fn fixture_key() -> FixtureKey {
        FixtureKey {
            created_at: "2026-03-22T10:30:00Z".to_owned(),
            id: "8b116c56-0a58-4c55-b7d7-06ee6bbddb8c".to_owned(),
        }
    }

    const _CONST_CURSOR: Cursor<&str> = Cursor::new("compile-time-test");
    const _CONST_DIRECTIONAL_CURSOR: Cursor<&str> =
        Cursor::with_direction("compile-time-test", Direction::Prev);

    fn encode_without_padding(cursor: &Cursor<FixtureKey>) -> String {
        cursor.encode().expect("cursor encoding should succeed")
    }

    fn encode_with_padding(cursor: &Cursor<FixtureKey>) -> String {
        let payload = serde_json::to_vec(cursor).expect("cursor should serialize");
        base64::engine::general_purpose::URL_SAFE.encode(payload)
    }

    #[rstest]
    #[case::unpadded(encode_without_padding)]
    #[case::padded(encode_with_padding)]
    fn cursor_decodes_successfully(
        fixture_key: FixtureKey,
        #[case] encode: for<'a> fn(&'a Cursor<FixtureKey>) -> String,
    ) {
        let cursor = Cursor::new(fixture_key);
        let encoded = encode(&cursor);

        let decoded =
            Cursor::<FixtureKey>::decode(&encoded).expect("cursor decoding should succeed");

        assert_eq!(decoded, cursor);
    }

    #[test]
    fn cursor_encode_reports_serialization_failure() {
        let cursor = Cursor::new(FailingKey);

        let result = cursor.encode();

        assert_eq!(
            result,
            Err(CursorError::Serialize {
                message: "fixture serialization failed".to_owned(),
            })
        );
    }

    #[rstest]
    #[case("!!!")]
    fn invalid_base64_cursor_fails_decode(#[case] raw_input: &str) {
        let result = Cursor::<FixtureKey>::decode(raw_input);

        assert!(matches!(result, Err(CursorError::InvalidBase64 { .. })));
    }

    #[test]
    fn oversized_cursor_fails_decode_before_base64_decoding() {
        let oversized = "a".repeat(MAX_CURSOR_TOKEN_LEN + 1);

        let result = Cursor::<FixtureKey>::decode(&oversized);

        assert_eq!(
            result,
            Err(CursorError::TokenTooLong {
                max_len: MAX_CURSOR_TOKEN_LEN,
            })
        );
    }
}
