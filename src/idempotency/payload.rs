//! Canonical payload hashing helpers.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Validation and hashing errors for [`PayloadHash`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PayloadHashError {
    /// The byte slice had the wrong length.
    #[error("payload hash must be {expected} bytes, got {actual}")]
    InvalidLength {
        /// Expected number of bytes.
        expected: usize,
        /// Actual number of bytes.
        actual: usize,
    },
    /// Canonical JSON serialization failed.
    #[error("failed to serialize canonical JSON payload: {message}")]
    Serialization {
        /// Human-readable serialization failure details.
        message: String,
    },
}

/// SHA-256 hash of a canonicalized request payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PayloadHash([u8; 32]);

impl PayloadHash {
    /// Construct a hash from a fixed-size byte array.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self { Self(bytes) }

    /// Construct a hash from a byte slice.
    ///
    /// # Errors
    ///
    /// Returns [`PayloadHashError::InvalidLength`] when the slice is not
    /// exactly 32 bytes.
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, PayloadHashError> {
        let digest_bytes: [u8; 32] =
            bytes
                .try_into()
                .map_err(|_| PayloadHashError::InvalidLength {
                    expected: 32,
                    actual: bytes.len(),
                })?;
        Ok(Self(digest_bytes))
    }

    /// Access the raw digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] { &self.0 }

    /// Encode the digest as lowercase hexadecimal.
    #[must_use]
    pub fn to_hex(&self) -> String { hex::encode(self.0) }
}

impl std::fmt::Display for PayloadHash {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.to_hex())
    }
}

/// Canonicalize a JSON payload and compute its SHA-256 hash.
///
/// Object keys are sorted recursively before compact serialization so
/// semantically identical objects produce identical hashes regardless of input
/// key order.
///
/// # Errors
///
/// Returns [`PayloadHashError::Serialization`] when the canonical JSON payload
/// cannot be serialized.
pub fn canonicalize_and_hash(value: &Value) -> Result<PayloadHash, PayloadHashError> {
    let canonical = canonicalize(value);
    let bytes =
        serde_json::to_vec(&canonical).map_err(|error| PayloadHashError::Serialization {
            message: error.to_string(),
        })?;
    let digest = Sha256::digest(bytes);
    let digest_bytes: [u8; 32] = digest.into();
    Ok(PayloadHash::from_bytes(digest_bytes))
}

fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries = map.iter().collect::<Vec<_>>();
            entries.sort_by_key(|(key, _)| *key);

            let canonical = entries
                .into_iter()
                .fold(Map::new(), |mut acc, (key, entry_value)| {
                    acc.insert(key.clone(), canonicalize(entry_value));
                    acc
                });
            Value::Object(canonical)
        }
        Value::Array(values) => Value::Array(values.iter().map(canonicalize).collect()),
        scalar => scalar.clone(),
    }
}

#[cfg(test)]
mod tests {
    //! Regression coverage for payload hashing.

    use serde_json::json;

    use super::{PayloadHash, PayloadHashError, canonicalize_and_hash};

    #[test]
    fn canonicalize_and_hash_matches_for_equivalent_objects() {
        let left = json!({"b": 2, "a": {"d": 4, "c": 3}});
        let right = json!({"a": {"c": 3, "d": 4}, "b": 2});

        let left_hash = canonicalize_and_hash(&left).expect("left payload should hash");
        let right_hash = canonicalize_and_hash(&right).expect("right payload should hash");

        assert_eq!(left_hash, right_hash);
    }

    #[test]
    fn canonicalize_and_hash_differs_for_distinct_payloads() {
        let left = json!({"routeId": "a"});
        let right = json!({"routeId": "b"});

        let left_hash = canonicalize_and_hash(&left).expect("left payload should hash");
        let right_hash = canonicalize_and_hash(&right).expect("right payload should hash");

        assert_ne!(left_hash, right_hash);
    }

    #[test]
    fn try_from_bytes_rejects_wrong_length() {
        let result = PayloadHash::try_from_bytes(&[0_u8; 4]);

        assert_eq!(
            result,
            Err(PayloadHashError::InvalidLength {
                expected: 32,
                actual: 4,
            })
        );
    }
}
