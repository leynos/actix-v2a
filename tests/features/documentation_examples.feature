Feature: Documented user-guide examples
  Every fenced example in the user-facing documentation is registered through a
  tested-example marker, and the operator output shown in the guide matches the
  tracing events the crate emits at runtime.

  Scenario: Every user-facing fence is registered
    Given the published user-facing documentation is loaded
    Then every fenced example has a stable unique identifier

  Scenario: Documented operator output matches the emitted tracing event
    Given the documented operator output for a malformed Last-Event-ID header
    When a duplicate Last-Event-ID header is rejected by the Actix adapter
    Then the emitted tracing event carries every documented field