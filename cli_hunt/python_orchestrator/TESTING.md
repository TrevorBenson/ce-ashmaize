# Unit Testing Documentation

## Overview

Comprehensive unit tests for the Python orchestrator modules covering:
- `submission.py` - HTTP submission handling and error classification
- `retry.py` - Retry queue management and exponential backoff
- `journal.py` - Write-ahead logging for crash recovery

## Test Structure

All test files are located in the `tests/` directory:
- `tests/test_submission.py`
- `tests/test_retry.py`
- `tests/test_journal.py`
- `tests/test_tui_logging.py`
- `tests/conftest.py` - pytest configuration for module imports

## Test Files

### tests/test_submission.py
Tests for submission logic including:
- **SubmissionResult dataclass** - Data structure validation
- **should_retry_error()** - Error classification (timeout, 5xx, 429, etc.)
- **check_already_exists_error()** - Detection of duplicate submissions
- **submit_solution()** - Full submission workflow
  - Happy paths: validated, solved, already_exists
  - Unhappy paths: timeouts, HTTP errors, JSON decode errors
  - Error message truncation
  - URL formatting

### tests/test_retry.py
Tests for retry logic including:
- **SubmissionRetryItem dataclass** - Serialization/deserialization
- **SubmissionRetryManager** - Queue management
  - Adding/removing retries
  - Priority queue ordering
  - Journal persistence
  - Expiration handling
  - Thread safety
- **_format_success_messages()** - Log message formatting
- **retry_worker()** - Worker function
  - Challenge expiration
  - Success scenarios (validated, solved, already_exists)
  - Retry exhaustion
  - Exponential backoff
  - Non-retryable errors

### tests/test_journal.py
Tests for journaling logic including:
- **Journal class** - Write-ahead logging
  - Appending entries
  - Replaying journal on startup
  - Clearing journal after checkpoint
  - Snapshotting active state
  - Thread safety
  - Error handling (IOError, malformed JSON)
  - File permissions

## Test Coverage

### Happy Paths ✓
- Successful submission with crypto receipt
- Successful submission without receipt
- Solution already exists detection
- Retry success after transient failure
- Journal replay on restart
- Thread-safe concurrent operations

### Unhappy Paths ✓
- Network timeouts (retryable)
- Connection errors (retryable)
- 5xx server errors (retryable)
- 429 rate limiting (retryable)
- 4xx client errors (non-retryable)
- JSON decode failures
- Malformed journal entries
- Expired challenges
- Retry exhaustion
- Disk I/O errors

### Edge Cases ✓
- Empty queues/journals
- Concurrent access
- Long error messages (truncation)
- Missing response fields
- Malformed timestamps
- Duplicate retry items
- File permission errors
- Race conditions

## Running Tests

### Install Test Dependencies

```bash
uv add --dev pytest pytest-cov pytest-mock
```

### Run All Tests

```bash
uv run pytest
```

### Run Specific Test File

```bash
uv run pytest test_submission.py
uv run pytest test_retry.py
uv run pytest test_journal.py
```

### Run Specific Test Class

```bash
uv run pytest test_submission.py::TestSubmitSolution
uv run pytest test_retry.py::TestSubmissionRetryManager
uv run pytest test_journal.py::TestJournalReplay
```

### Run Specific Test

```bash
uv run pytest test_submission.py::TestSubmitSolution::test_successful_submission_with_crypto_receipt
```

### Run with Coverage Report

```bash
uv run pytest --cov=submission --cov=retry --cov=journal --cov-report=html
```

View coverage report: `htmlcov/index.html`

### Run with Verbose Output

```bash
uv run pytest -v
```

### Run Only Fast Tests (exclude slow)

```bash
uv run pytest -m "not slow"
```

## Test Structure

Each test file follows the pattern:

```python
class TestClassName:
    """Test description"""
    
    @pytest.fixture
    def fixture_name(self):
        """Setup test data"""
        return test_data
    
    def test_happy_path(self, fixture_name):
        """Test normal operation"""
        # Arrange
        # Act
        # Assert
    
    def test_unhappy_path(self, fixture_name):
        """Test error handling"""
        # Arrange with mocked error
        # Act
        # Assert error handled correctly
```

## Mocking Strategy

Tests use `unittest.mock` to isolate units:

- **HTTP requests** - Mocked `session.post()` to avoid network calls
- **File I/O** - Temporary directories via `pytest.tmp_path`
- **Time** - Controlled datetime objects for expiration testing
- **Logging** - `caplog` fixture to verify log messages
- **Threading** - Real threads to verify thread safety

## Code Coverage Goals

Target coverage for each module:

| Module | Target | Current |
|--------|--------|---------|
| submission.py | 95%+ | ✓ |
| retry.py | 90%+ | ✓ |
| journal.py | 95%+ | ✓ |

## Continuous Integration

Tests can be integrated with CI/CD pipelines:

```yaml
# .github/workflows/test.yml
name: Tests
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Install uv
        run: curl -LsSf https://astral.sh/uv/install.sh | sh
      - name: Install dependencies
        run: uv sync --dev
      - name: Run tests
        run: uv run pytest --cov --cov-report=xml
      - name: Upload coverage
        uses: codecov/codecov-action@v2
```

## Test Maintenance

### Adding New Tests

When adding new functionality:

1. Write tests first (TDD approach)
2. Include happy path, unhappy path, and edge cases
3. Mock external dependencies
4. Verify thread safety if applicable
5. Update this documentation

### Debugging Failed Tests

```bash
# Run with detailed output
uv run pytest -vv --tb=long

# Run with Python debugger
uv run pytest --pdb

# Run last failed test only
uv run pytest --lf
```

## Test Data

Tests use realistic data patterns observed in production:

- Challenge IDs: `"ch456"`, `"D17C01"`
- Addresses: `"addr123"`, 64-char hex strings
- Nonces: `"nonce789"`, hex strings
- Timestamps: ISO 8601 format with timezone
- Error messages: Actual curl/HTTP error formats

## Known Limitations

1. `retry_worker()` tests use mocking rather than real threading
2. Integration tests between modules are limited
3. Performance tests not included (benchmarking separate)
4. Tests assume Python 3.10+ (pathlib, type hints)

## Future Improvements

- [ ] Add integration tests with all modules
- [ ] Add performance/load tests
- [ ] Add property-based testing with Hypothesis
- [ ] Add mutation testing with mutmut
- [ ] Add contract tests for external API
- [ ] Add snapshot tests for log output

