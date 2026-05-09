## Issue #1: Project Lacks Unit Tests

### Description
The helios project currently has zero unit test coverage. This makes it difficult to:
- Verify correctness of core functionality
- Catch regressions during refactoring
- Document expected behavior
- Enable confident code changes

### Affected Modules
- [ ] `src/utils.rs` - Utility functions (format_json, replace_variables, truncate)
- [ ] `src/models.rs` - Data structures and serialization
- [ ] `src/http_client.rs` - HTTP client and header parsing
- [ ] `src/storage.rs` - Data persistence
- [ ] `src/export_import.rs` - Import/export functionality

### Proposed Solution
Implement comprehensive unit tests following TDD principles:
1. Write failing tests first (RED)
2. Implement/fix code to make tests pass (GREEN)
3. Refactor if needed (REFACTOR)

### Expected Test Coverage
| Module | Test Count | Focus Areas |
|--------|------------|-------------|
| utils.rs | 9+ | Variable replacement, JSON formatting, truncation |
| models.rs | 11+ | Data structures, serialization, defaults |
| http_client.rs | 7+ | Header parsing, edge cases |

### Acceptance Criteria
- [ ] All core modules have unit tests
- [ ] Tests follow TDD red-green-refactor cycle
- [ ] All tests pass (`cargo test` exits 0)
- [ ] Bug discovered during TDD is fixed (e.g., missing PartialEq on Auth enum)

### Benefits
- Improved code reliability
- Easier refactoring
- Better documentation through tests
- Confidence in future changes

### Labels
- enhancement
- testing
- good first issue