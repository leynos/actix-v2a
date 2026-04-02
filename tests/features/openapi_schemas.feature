Feature: shared OpenAPI schema fragments
  Scenario: shared envelope schemata are registered
    Given the shared OpenAPI schema document is generated
    Then the components section contains the ErrorCode schema wrapper
    And the components section contains the Error schema wrapper
    And the components section contains the ReplayMetadata schema wrapper

  Scenario: shared envelope schemata expose the expected fields
    Given the shared OpenAPI schema document is generated
    Then the Error schema exposes code message traceId and details fields
    And the ErrorCode schema enumerates the shared error codes
    And the ReplayMetadata schema exposes the replayed field
