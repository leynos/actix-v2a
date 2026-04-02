//! Stored idempotency records and replay/conflict helpers.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::idempotency::{IdempotencyKey, MutationType, PayloadHash};

/// Snapshot of a previously computed response that can be replayed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseSnapshot {
    /// HTTP status code to replay.
    pub status_code: u16,
    /// JSON response body to replay.
    pub body: Value,
}

impl ResponseSnapshot {
    /// Construct a response snapshot from explicit values.
    #[must_use]
    pub const fn new(status_code: u16, body: Value) -> Self { Self { status_code, body } }
}

/// Stored idempotency record linking a request to its replay snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdempotencyRecord {
    /// The client-provided idempotency key.
    pub key: IdempotencyKey,
    /// Application-provided mutation discriminator.
    pub mutation_type: MutationType,
    /// SHA-256 hash of the canonicalized request payload.
    pub payload_hash: PayloadHash,
    /// Snapshot of the original response.
    pub response: ResponseSnapshot,
}

impl IdempotencyRecord {
    /// Construct a record from explicit values.
    #[must_use]
    pub const fn new(
        key: IdempotencyKey,
        mutation_type: MutationType,
        payload_hash: PayloadHash,
        response: ResponseSnapshot,
    ) -> Self {
        Self {
            key,
            mutation_type,
            payload_hash,
            response,
        }
    }
}

/// Query parameters used when checking an idempotency store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdempotencyLookupQuery {
    /// The idempotency key to look up.
    pub key: IdempotencyKey,
    /// The mutation discriminator in scope for the request.
    pub mutation_type: MutationType,
    /// Hash of the incoming request payload.
    pub payload_hash: PayloadHash,
}

impl IdempotencyLookupQuery {
    /// Construct a lookup query.
    #[must_use]
    pub const fn new(
        key: IdempotencyKey,
        mutation_type: MutationType,
        payload_hash: PayloadHash,
    ) -> Self {
        Self {
            key,
            mutation_type,
            payload_hash,
        }
    }
}

/// Outcome of checking an idempotency store for an existing request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IdempotencyLookupResult {
    /// No record exists for this key and mutation type.
    NotFound,
    /// A record exists and the payload hash matches, so the response can replay.
    MatchingPayload(IdempotencyRecord),
    /// A record exists but the payload hash differs, so the request conflicts.
    ConflictingPayload(IdempotencyRecord),
}

impl IdempotencyLookupResult {
    /// Classify one existing record against the incoming payload hash.
    #[must_use]
    pub fn classify(record: IdempotencyRecord, incoming_hash: &PayloadHash) -> Self {
        if &record.payload_hash == incoming_hash {
            Self::MatchingPayload(record)
        } else {
            Self::ConflictingPayload(record)
        }
    }
}

/// Shared response metadata for idempotent command payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayMetadata {
    /// Whether the response was replayed from an existing idempotency record.
    pub replayed: bool,
}

impl ReplayMetadata {
    /// Construct metadata for a fresh response.
    #[must_use]
    pub const fn fresh() -> Self { Self { replayed: false } }

    /// Construct metadata for a replayed response.
    #[must_use]
    pub const fn replayed() -> Self { Self { replayed: true } }
}

#[cfg(test)]
mod tests {
    //! Regression coverage for replay and conflict helpers.

    use serde_json::json;

    use super::{
        IdempotencyKey,
        IdempotencyLookupQuery,
        IdempotencyLookupResult,
        IdempotencyRecord,
        MutationType,
        PayloadHash,
        ReplayMetadata,
        ResponseSnapshot,
    };

    #[test]
    fn classify_reports_replay_for_matching_hash() {
        let key = IdempotencyKey::new("550e8400-e29b-41d4-a716-446655440000")
            .expect("fixture UUID should validate");
        let mutation = MutationType::new("routes").expect("fixture mutation should validate");
        let payload_hash = PayloadHash::from_bytes([7_u8; 32]);
        let record = IdempotencyRecord::new(
            key.clone(),
            mutation.clone(),
            payload_hash.clone(),
            ResponseSnapshot::new(200, json!({"ok": true})),
        );

        let result = IdempotencyLookupResult::classify(record.clone(), &payload_hash);

        assert_eq!(result, IdempotencyLookupResult::MatchingPayload(record));
    }

    #[test]
    fn classify_reports_conflict_for_different_hash() {
        let key = IdempotencyKey::new("550e8400-e29b-41d4-a716-446655440000")
            .expect("fixture UUID should validate");
        let mutation = MutationType::new("routes").expect("fixture mutation should validate");
        let record = IdempotencyRecord::new(
            key.clone(),
            mutation.clone(),
            PayloadHash::from_bytes([7_u8; 32]),
            ResponseSnapshot::new(200, json!({"ok": true})),
        );

        let result =
            IdempotencyLookupResult::classify(record.clone(), &PayloadHash::from_bytes([8_u8; 32]));

        assert_eq!(result, IdempotencyLookupResult::ConflictingPayload(record));
    }

    #[test]
    fn replay_metadata_serializes_to_shared_shape() {
        let payload =
            serde_json::to_value(ReplayMetadata::replayed()).expect("metadata should serialize");

        assert_eq!(payload, json!({"replayed": true}));
    }

    #[test]
    fn lookup_query_groups_related_parameters() {
        let key = IdempotencyKey::new("550e8400-e29b-41d4-a716-446655440000")
            .expect("fixture UUID should validate");
        let mutation = MutationType::new("routes").expect("fixture mutation should validate");
        let query = IdempotencyLookupQuery::new(
            key.clone(),
            mutation.clone(),
            PayloadHash::from_bytes([1_u8; 32]),
        );

        assert_eq!(query.key, key);
        assert_eq!(query.mutation_type, mutation);
    }
}
