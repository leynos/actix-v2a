Feature: Pagination documentation invariants
  The pagination documentation promises specific behaviour for limits, cursors,
  and error handling. These scenarios verify that the documented invariants hold
  at runtime.

  Scenario: Default limit is applied when no limit is provided
    Given pagination documentation parameters without a limit
    Then the documented normalized limit equals DEFAULT_LIMIT

  Scenario: Maximum limit caps oversized requests
    Given pagination documentation parameters with limit 500
    Then the documented normalized limit equals MAX_LIMIT

  Scenario: Zero limit is rejected with an error
    When pagination documentation parameters are created with limit 0
    Then page parameter creation fails with InvalidLimit error

  Scenario: Invalid base64 token produces InvalidBase64 error
    Given an invalid base64 cursor token "not!valid"
    When the documentation cursor is decoded
    Then decoding fails with InvalidBase64 error

  Scenario: Structurally invalid JSON produces Deserialize error
    Given a base64url token containing invalid JSON
    When the documentation cursor is decoded
    Then decoding fails with Deserialize error

  Scenario: Oversized cursor token produces TokenTooLong error
    Given an oversized cursor token
    When the documentation cursor is decoded
    Then decoding fails with TokenTooLong error

  Scenario: Error display strings are human-readable
    Given pagination errors of different documented variants
    Then each pagination error display string contains a descriptive message
