Feature: Shared SSE wire contract
  The shared SSE module exposes validated replay, framing, cache, heartbeat,
  and stream-reset helpers through the crate root.

  Scenario: Reconnect requests preserve the Last-Event-ID cursor
    Given a reconnect request with Last-Event-ID "evt-42"
    When the replay cursor is extracted
    Then the replay cursor preserves "evt-42"
    And the shared Last-Event-ID header name ignores non-matching headers

  Scenario: Live streams use the canonical cache and heartbeat policy
    Given an event-stream response
    When the live-stream cache policy and heartbeat are applied
    Then the response uses the canonical no-store cache policy
    And the heartbeat frame is the canonical empty comment

  Scenario: Event and stream reset frames render through the public API
    Given a downstream event with id "evt-100", event "message_created", and payload "hello"
    When the stream reset frame is rendered
    Then the event and stream reset frames match the approved wire format
