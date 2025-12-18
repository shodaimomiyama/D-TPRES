# Requirements Document: {{featureName}}

## Introduction

[Provide a brief overview of the feature, its purpose, and its value to users]

## Alignment with Product Vision

[Explain how this feature supports the goals outlined in product.md]

## Requirements

### Requirement 1: [Requirement Name]

**User Story:** As a [role], I want [feature], so that [benefit]

#### Design Note

[Optional: Explain design decisions, rationale, or important context for this requirement]

#### Acceptance Criteria

1. WHEN [event] THEN system SHALL [response]
2. IF [precondition] THEN system SHALL [response]
3. WHEN [event] AND [condition] THEN system SHALL [response]

#### Test Coverage

<!--
IMPORTANT: This section is created through a collaborative discussion between AI and user.

AI WORKFLOW for Test Case Design:
1. After defining Acceptance Criteria, AI MUST lead a discussion with the user:
   - "Based on these Acceptance Criteria, I propose the following test cases..."
   - Present test function names and their purposes
   - Ask: "Do you want to add/modify any test cases?"

2. Discussion points to cover:
   - Success path tests (happy path)
   - Error/validation tests (what should fail?)
   - Edge cases (boundary conditions, empty data, etc.)
   - State machine tests (if applicable)
   - Security tests (for sensitive data)
   - Integration points (for cross-component behavior)

3. Document the agreed test cases in the table below.
-->

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_xxx_valid` | [What this test verifies] | Success |
| 2 | `test_xxx_error` | [What this test verifies] | Validation |
| ... | ... | ... | ... |

**Additional Tests (Beyond AC):**
- `test_xxx` - [Purpose]

### Requirement 2: [Requirement Name]

**User Story:** As a [role], I want [feature], so that [benefit]

#### Design Note

[Optional: Design decisions and context]

#### Acceptance Criteria

1. WHEN [event] THEN system SHALL [response]
2. IF [precondition] THEN system SHALL [response]

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_xxx` | [What this test verifies] | [Pattern] |
| 2 | `test_xxx` | [What this test verifies] | [Pattern] |

## Non-Functional Requirements

### Code Architecture and Modularity
- **Single Responsibility Principle**: Each file should have a single, well-defined purpose
- **Modular Design**: Components, utilities, and services should be isolated and reusable
- **Dependency Management**: Minimize interdependencies between modules
- **Clear Interfaces**: Define clean contracts between components and layers

### Performance
- [Performance requirements]

### Security
- [Security requirements]

### Reliability
- [Reliability requirements]

### Usability
- [Usability requirements]

## Test Coverage Summary

### Overall Statistics

| Requirement | Total AC | Test Functions | Coverage |
|-------------|----------|----------------|----------|
| 1. [Name] | X | Y | Z% |
| 2. [Name] | X | Y | Z% |
| **Total** | **X** | **Y** | **Z%** |

### Test Patterns Used

1. **Success Pattern**: Valid input verification with all getters
2. **Validation Pattern**: Invalid input error confirmation (empty data, out of bounds, duplicates)
3. **State Machine Pattern**: Valid/invalid state transition verification
4. **Mutation Pattern**: Setter operations and persistence confirmation
5. **Query Pattern**: Single/multiple item retrieval operations
6. **Security Pattern**: Sensitive data redaction in Debug output
7. **Type Safety Pattern**: Compile-time type safety documentation
8. **Crypto Pattern**: Cryptographic operation correctness verification

### Legend

- ✅ **Covered**: Test implemented and fully covers AC
- ⚠️ **Partial**: Indirect test (e.g., Debug redaction for Zeroize) or N/A
- ❌ **Missing**: Test not implemented

---

## AI Workflow Instructions

<!--
This section provides instructions for AI agents following the SDD workflow.

### Phase 1: Requirements Discussion Flow

When creating this document, the AI MUST follow this enhanced workflow:

1. **Define User Stories and Acceptance Criteria first**
   - Follow EARS (Easy Approach to Requirements Syntax) format
   - Each AC should be testable

2. **Lead Test Case Discussion (CRITICAL)**
   After defining each Requirement's AC, the AI MUST:

   a) **Propose test cases proactively**:
      "Based on AC1-4, I propose the following test cases:
      - `test_xxx_new_valid` - Verifies successful creation (AC1)
      - `test_xxx_invalid_param` - Verifies error on invalid input (AC2)
      ..."

   b) **Ask for user input**:
      "Would you like to:
      - Add more test cases for edge cases?
      - Modify any proposed test functions?
      - Add security-specific tests?
      - Include any integration tests?"

   c) **Discuss test patterns**:
      "For this requirement, I recommend using:
      - Success Pattern for AC1
      - Validation Pattern for AC2
      - State Machine Pattern for AC3
      Does this align with your testing strategy?"

   d) **Document agreed test cases** in the Test Coverage table

3. **Complete Test Coverage Summary**
   - Calculate coverage statistics
   - Identify any gaps
   - Propose tests for uncovered ACs

### Test Naming Conventions

- `test_{component}_{action}_{expected_result}`
- Examples:
  - `test_secret_new_valid` - Success case
  - `test_secret_new_invalid_threshold_zero` - Validation error case
  - `test_secret_split_transition` - State machine test
  - `test_secret_debug_redacted` - Security test

### Minimum Test Coverage Expectations

- Each AC should have at least one corresponding test
- Critical paths: 100% coverage
- Error handling: At least one test per error type
- Security-sensitive code: Debug redaction test required
-->
