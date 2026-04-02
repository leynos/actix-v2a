//! Client-provided idempotency key validation.

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Validation errors for [`IdempotencyKey`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IdempotencyKeyValidationError {
    /// The key string was empty once trimmed.
    #[error("idempotency key must not be empty")]
    EmptyKey,
    /// The key string was not a valid UUID.
    #[error("idempotency key must be a valid UUID")]
    InvalidKey,
}

/// Validated client-provided idempotency key.
///
/// # Examples
///
/// ```
/// use actix_v2a::IdempotencyKey;
///
/// let key = IdempotencyKey::new("550e8400-e29b-41d4-a716-446655440000").expect("valid UUID");
///
/// assert_eq!(key.as_ref(), "550e8400-e29b-41d4-a716-446655440000");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct IdempotencyKey(Uuid, String);

impl IdempotencyKey {
    /// Validate and construct an idempotency key.
    ///
    /// # Errors
    ///
    /// Returns [`IdempotencyKeyValidationError::EmptyKey`] when the input is
    /// blank once trimmed and [`IdempotencyKeyValidationError::InvalidKey`]
    /// when the input is not a UUID.
    pub fn new(value: impl AsRef<str>) -> Result<Self, IdempotencyKeyValidationError> {
        Self::from_str(value.as_ref())
    }

    /// Construct a key directly from an already validated UUID.
    #[must_use]
    pub fn from_uuid(uuid: Uuid) -> Self { Self(uuid, uuid.to_string()) }

    /// Generate a random key for testing or synthetic requests.
    #[must_use]
    pub fn random() -> Self {
        let uuid = Uuid::new_v4();
        Self::from_uuid(uuid)
    }

    /// Access the UUID representation.
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid { &self.0 }

    fn from_str(value: &str) -> Result<Self, IdempotencyKeyValidationError> {
        if value.trim().is_empty() {
            return Err(IdempotencyKeyValidationError::EmptyKey);
        }
        if value.trim() != value {
            return Err(IdempotencyKeyValidationError::InvalidKey);
        }

        let parsed =
            Uuid::parse_str(value).map_err(|_| IdempotencyKeyValidationError::InvalidKey)?;
        Ok(Self(parsed, parsed.to_string()))
    }
}

impl AsRef<str> for IdempotencyKey {
    fn as_ref(&self) -> &str { self.1.as_str() }
}

impl fmt::Display for IdempotencyKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_ref())
    }
}

impl From<IdempotencyKey> for String {
    fn from(value: IdempotencyKey) -> Self { value.1 }
}

impl TryFrom<String> for IdempotencyKey {
    type Error = IdempotencyKeyValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> { Self::from_str(&value) }
}

#[cfg(test)]
mod tests {
    //! Regression coverage for idempotency key validation.

    use super::{IdempotencyKey, IdempotencyKeyValidationError};

    #[test]
    fn new_rejects_blank_keys() {
        let result = IdempotencyKey::new("   ");

        assert_eq!(result, Err(IdempotencyKeyValidationError::EmptyKey));
    }

    #[test]
    fn new_rejects_non_uuid_values() {
        let result = IdempotencyKey::new("not-a-uuid");

        assert_eq!(result, Err(IdempotencyKeyValidationError::InvalidKey));
    }

    #[test]
    fn from_uuid_preserves_display_form() {
        let key = IdempotencyKey::new("550e8400-e29b-41d4-a716-446655440000")
            .expect("fixture UUID should validate");

        assert_eq!(key.as_uuid().to_string(), key.as_ref());
    }
}
