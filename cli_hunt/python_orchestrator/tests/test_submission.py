"""
Unit tests for submission.py module

Tests cover:
- SubmissionResult dataclass
- should_retry_error() function
- check_already_exists_error() function
- submit_solution() function (happy and unhappy paths)
"""

import json
import logging
from datetime import datetime, timezone
from unittest.mock import Mock, patch, MagicMock

import pytest
from curl_cffi import requests

from submission import (
    SubmissionResult,
    should_retry_error,
    check_already_exists_error,
    submit_solution,
)


class TestSubmissionResult:
    """Test SubmissionResult dataclass"""

    def test_creation_with_all_fields(self):
        result = SubmissionResult(
            status="validated",
            update={"key": "value"},
            error="some error"
        )
        assert result.status == "validated"
        assert result.update == {"key": "value"}
        assert result.error == "some error"

    def test_creation_with_defaults(self):
        result = SubmissionResult(status="solved")
        assert result.status == "solved"
        assert result.update is None
        assert result.error is None

    def test_creation_minimal(self):
        result = SubmissionResult("failed", error="timeout")
        assert result.status == "failed"
        assert result.error == "timeout"
        assert result.update is None


class TestShouldRetryError:
    """Test should_retry_error() function"""

    def test_timeout_error_returns_true(self):
        error = requests.exceptions.Timeout("Connection timeout")
        assert should_retry_error(error) is True

    def test_connection_error_returns_true(self):
        error = requests.exceptions.ConnectionError("Connection refused")
        assert should_retry_error(error) is True

    def test_500_error_returns_true(self):
        response = Mock()
        response.status_code = 500
        error = requests.exceptions.HTTPError("Server error")
        error.response = response
        assert should_retry_error(error) is True

    def test_503_error_returns_true(self):
        response = Mock()
        response.status_code = 503
        error = requests.exceptions.HTTPError("Service unavailable")
        error.response = response
        assert should_retry_error(error) is True

    def test_429_error_returns_true(self):
        response = Mock()
        response.status_code = 429
        error = requests.exceptions.HTTPError("Too many requests")
        error.response = response
        assert should_retry_error(error) is True

    def test_400_error_returns_false(self):
        response = Mock()
        response.status_code = 400
        error = requests.exceptions.HTTPError("Bad request")
        error.response = response
        assert should_retry_error(error) is False

    def test_404_error_returns_false(self):
        response = Mock()
        response.status_code = 404
        error = requests.exceptions.HTTPError("Not found")
        error.response = response
        assert should_retry_error(error) is False

    def test_error_without_response_returns_false(self):
        error = requests.exceptions.HTTPError("Generic error")
        assert should_retry_error(error) is False

    def test_request_exception_without_status_returns_false(self):
        error = requests.exceptions.RequestException("Generic error")
        assert should_retry_error(error) is False


class TestCheckAlreadyExistsError:
    """Test check_already_exists_error() function"""

    def test_returns_true_for_400_with_already_exists_message(self):
        response = Mock()
        response.status_code = 400
        response.json.return_value = {"message": "Solution already exists"}
        error = requests.exceptions.HTTPError("Bad request")
        error.response = response
        assert check_already_exists_error(error) is True

    def test_returns_true_case_insensitive(self):
        response = Mock()
        response.status_code = 400
        response.json.return_value = {"message": "Solution ALREADY EXISTS"}
        error = requests.exceptions.HTTPError("Bad request")
        error.response = response
        assert check_already_exists_error(error) is True

    def test_returns_false_for_400_with_different_message(self):
        response = Mock()
        response.status_code = 400
        response.json.return_value = {"message": "Invalid solution"}
        error = requests.exceptions.HTTPError("Bad request")
        error.response = response
        assert check_already_exists_error(error) is False

    def test_returns_false_for_non_400_status(self):
        response = Mock()
        response.status_code = 500
        response.json.return_value = {"message": "Solution already exists"}
        error = requests.exceptions.HTTPError("Server error")
        error.response = response
        assert check_already_exists_error(error) is False

    def test_returns_false_when_json_decode_fails(self):
        response = Mock()
        response.status_code = 400
        response.json.side_effect = json.JSONDecodeError("Invalid JSON", "", 0)
        error = requests.exceptions.HTTPError("Bad request")
        error.response = response
        assert check_already_exists_error(error) is False

    def test_returns_false_when_no_message_key(self):
        response = Mock()
        response.status_code = 400
        response.json.return_value = {"error": "Something went wrong"}
        error = requests.exceptions.HTTPError("Bad request")
        error.response = response
        assert check_already_exists_error(error) is False

    def test_returns_false_when_no_response(self):
        error = requests.exceptions.HTTPError("Bad request")
        assert check_already_exists_error(error) is False


class TestSubmitSolution:
    """Test submit_solution() function"""

    @pytest.fixture
    def mock_session(self):
        return Mock(spec=requests.Session)

    def test_successful_submission_with_crypto_receipt(self, mock_session):
        response = Mock()
        response.json.return_value = {
            "crypto_receipt": {"signature": "abc123"}
        }
        mock_session.post.return_value = response

        result = submit_solution(
            "addr123", "challenge456", "nonce789", mock_session, timeout=10
        )

        assert result.status == "validated"
        assert result.update["status"] == "validated"
        assert result.update["salt"] == "nonce789"
        assert result.update["cryptoReceipt"] == {"signature": "abc123"}
        assert "submittedAt" in result.update
        assert "validatedAt" in result.update
        assert result.error is None

        mock_session.post.assert_called_once()
        call_args = mock_session.post.call_args
        assert "addr123" in call_args[0][0]
        assert "challenge456" in call_args[0][0]
        assert "nonce789" in call_args[0][0]
        assert call_args[1]["timeout"] == 10

    def test_successful_submission_without_crypto_receipt(self, mock_session):
        response = Mock()
        response.json.return_value = {}
        mock_session.post.return_value = response

        result = submit_solution(
            "addr123", "challenge456", "nonce789", mock_session
        )

        assert result.status == "solved"
        assert result.update["status"] == "solved"
        assert result.update["salt"] == "nonce789"
        assert "cryptoReceipt" not in result.update
        assert "submittedAt" in result.update
        assert result.error is None

    def test_json_decode_error(self, mock_session, caplog):
        response = Mock()
        response.json.side_effect = json.JSONDecodeError("Invalid", "", 0)
        mock_session.post.return_value = response

        with caplog.at_level(logging.WARNING):
            result = submit_solution(
                "addr123", "challenge456", "nonce789", mock_session
            )

        assert result.status == "submission_error"
        assert result.update["status"] == "submission_error"
        assert result.update["salt"] == "nonce789"
        assert "Failed to decode" in caplog.text

    def test_http_error_already_exists(self, mock_session):
        response = Mock()
        response.status_code = 400
        response.json.return_value = {"message": "Solution already exists"}
        error = requests.exceptions.HTTPError("Bad request")
        error.response = response
        mock_session.post.side_effect = error

        result = submit_solution(
            "addr123", "challenge456", "nonce789", mock_session
        )

        assert result.status == "already_exists"
        assert result.update["status"] == "solved"
        assert result.update["salt"] == "nonce789"
        assert "submittedAt" in result.update
        assert result.error is None

    def test_http_error_retryable_500(self, mock_session):
        response = Mock()
        response.status_code = 500
        error = requests.exceptions.HTTPError("Server error")
        error.response = response
        mock_session.post.side_effect = error

        result = submit_solution(
            "addr123", "challenge456", "nonce789", mock_session
        )

        assert result.status == "should_retry"
        assert result.update is None
        assert result.error is not None
        assert "Server error" in result.error or "500" in result.error

    def test_http_error_retryable_429(self, mock_session):
        response = Mock()
        response.status_code = 429
        error = requests.exceptions.HTTPError("Rate limited")
        error.response = response
        mock_session.post.side_effect = error

        result = submit_solution(
            "addr123", "challenge456", "nonce789", mock_session
        )

        assert result.status == "should_retry"
        assert result.error is not None

    def test_http_error_non_retryable_404(self, mock_session):
        response = Mock()
        response.status_code = 404
        error = requests.exceptions.HTTPError("Not found")
        error.response = response
        mock_session.post.side_effect = error

        result = submit_solution(
            "addr123", "challenge456", "nonce789", mock_session
        )

        assert result.status == "failed"
        assert result.update is None
        assert result.error is not None

    def test_timeout_error_retryable(self, mock_session):
        error = requests.exceptions.Timeout("Connection timeout")
        mock_session.post.side_effect = error

        result = submit_solution(
            "addr123", "challenge456", "nonce789", mock_session
        )

        assert result.status == "should_retry"
        assert result.update is None
        assert "timeout" in result.error.lower()

    def test_connection_error_retryable(self, mock_session):
        error = requests.exceptions.ConnectionError("Connection refused")
        mock_session.post.side_effect = error

        result = submit_solution(
            "addr123", "challenge456", "nonce789", mock_session
        )

        assert result.status == "should_retry"
        assert result.update is None
        assert result.error is not None

    def test_error_message_truncated_when_too_long(self, mock_session):
        long_error = "x" * 150
        error = requests.exceptions.RequestException(long_error)
        mock_session.post.side_effect = error

        result = submit_solution(
            "addr123", "challenge456", "nonce789", mock_session
        )

        assert len(result.error) <= 104
        assert result.error.endswith("...")

    def test_default_timeout_used(self, mock_session):
        response = Mock()
        response.json.return_value = {}
        mock_session.post.return_value = response

        submit_solution("addr123", "challenge456", "nonce789", mock_session)

        call_args = mock_session.post.call_args
        assert call_args[1]["timeout"] == 10

    def test_custom_timeout_used(self, mock_session):
        response = Mock()
        response.json.return_value = {}
        mock_session.post.return_value = response

        submit_solution(
            "addr123", "challenge456", "nonce789", mock_session, timeout=30
        )

        call_args = mock_session.post.call_args
        assert call_args[1]["timeout"] == 30

    def test_raises_for_status_called(self, mock_session):
        response = Mock()
        response.json.return_value = {}
        response.raise_for_status = Mock()
        mock_session.post.return_value = response

        submit_solution("addr123", "challenge456", "nonce789", mock_session)

        response.raise_for_status.assert_called_once()

    def test_correct_url_format(self, mock_session):
        response = Mock()
        response.json.return_value = {}
        mock_session.post.return_value = response

        submit_solution("myaddress", "ch123", "n456", mock_session)

        call_args = mock_session.post.call_args
        url = call_args[0][0]
        assert url == "https://scavenger.prod.gd.midnighttge.io/solution/myaddress/ch123/n456"

