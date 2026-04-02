//! Generic mutation discriminator for idempotent operations.

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Validation errors for [`MutationType`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MutationTypeValidationError {
    /// The discriminator was empty once trimmed.
    #[error("mutation type must not be empty")]
    Empty,
    /// The discriminator contained surrounding whitespace.
    #[error("mutation type must not include surrounding whitespace")]
    SurroundingWhitespace,
}

/// Generic mutation discriminator used to namespace idempotency keys.
///
/// Applications supply stable values such as `routes`, `preferences`, or
/// `offline_bundle_upsert`. `actix-v2a` keeps this as a validated string
/// rather than an application-specific enum so multiple apps can share the
/// same idempotency primitives.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct MutationType(String);

impl MutationType {
    /// Validate and construct a mutation discriminator.
    ///
    /// # Errors
    ///
    /// Returns [`MutationTypeValidationError::Empty`] when the input is blank
    /// once trimmed and
    /// [`MutationTypeValidationError::SurroundingWhitespace`] when the input
    /// contains leading or trailing whitespace.
    pub fn new(candidate: impl Into<String>) -> Result<Self, MutationTypeValidationError> {
        let candidate_text = candidate.into();
        if candidate_text.trim().is_empty() {
            return Err(MutationTypeValidationError::Empty);
        }
        if candidate_text.trim() != candidate_text {
            return Err(MutationTypeValidationError::SurroundingWhitespace);
        }

        Ok(Self(candidate_text))
    }

    /// Access the discriminator as a string.
    #[must_use]
    pub const fn as_str(&self) -> &str { self.0.as_str() }
}

impl fmt::Display for MutationType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl From<MutationType> for String {
    fn from(value: MutationType) -> Self { value.0 }
}

impl TryFrom<String> for MutationType {
    type Error = MutationTypeValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> { Self::new(value) }
}

#[cfg(test)]
mod tests {
    //! Regression coverage for mutation discriminator validation.

    use super::{MutationType, MutationTypeValidationError};

    #[test]
    fn new_rejects_blank_values() {
        let result = MutationType::new("   ");

        assert_eq!(result, Err(MutationTypeValidationError::Empty));
    }

    #[test]
    fn new_rejects_surrounding_whitespace() {
        let result = MutationType::new(" routes ");

        assert_eq!(
            result,
            Err(MutationTypeValidationError::SurroundingWhitespace)
        );
    }

    #[test]
    fn new_preserves_valid_value() {
        let mutation = MutationType::new("routes").expect("valid mutation type");

        assert_eq!(mutation.as_str(), "routes");
    }
}
