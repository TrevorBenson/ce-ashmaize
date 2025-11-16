"""
Unit tests for retry.py module

Tests cover:
- SubmissionRetryItem dataclass
- SubmissionRetryManager class
- _format_success_messages() function
- retry_worker() function
"""

import json
import threading
from datetime import datetime, timezone, timedelta
from pathlib import Path
from unittest.mock import Mock, MagicMock, patch, call

import pytest

from retry import (
    SubmissionRetryItem,
    SubmissionRetryManager,
    _format_success_messages,
    retry_worker,
)
from submission import SubmissionResult


class TestSubmissionRetryItem:
    """Test SubmissionRetryItem dataclass"""

    @pytest.fixture
    def sample_item(self):
        return SubmissionRetryItem(
            address="addr123",
            challenge_id="ch456",
            nonce="nonce789",
            challenge_data={"challengeId": "ch456", "difficulty": 5},
            attempt_count=1,
            next_retry_time=datetime(2024, 1, 1, 12, 0, 0, tzinfo=timezone.utc),
            first_attempt_time=datetime(2024, 1, 1, 11, 0, 0, tzinfo=timezone.utc),
            last_error="Timeout error"
        )

    def test_creation(self, sample_item):
        assert sample_item.address == "addr123"
        assert sample_item.challenge_id == "ch456"
        assert sample_item.nonce == "nonce789"
        assert sample_item.attempt_count == 1

    def test_less_than_comparison(self, sample_item):
        earlier_item = SubmissionRetryItem(
            address="addr", challenge_id="ch1", nonce="n1",
            challenge_data={}, attempt_count=1,
            next_retry_time=datetime(2024, 1, 1, 11, 0, 0, tzinfo=timezone.utc),
            first_attempt_time=datetime(2024, 1, 1, 10, 0, 0, tzinfo=timezone.utc),
            last_error=""
        )
        later_item = SubmissionRetryItem(
            address="addr", challenge_id="ch2", nonce="n2",
            challenge_data={}, attempt_count=1,
            next_retry_time=datetime(2024, 1, 1, 13, 0, 0, tzinfo=timezone.utc),
            first_attempt_time=datetime(2024, 1, 1, 10, 0, 0, tzinfo=timezone.utc),
            last_error=""
        )
        assert earlier_item < later_item
        assert not (later_item < earlier_item)

    def test_to_dict(self, sample_item):
        result = sample_item.to_dict()
        assert result["address"] == "addr123"
        assert result["challenge_id"] == "ch456"
        assert result["nonce"] == "nonce789"
        assert result["attempt_count"] == 1
        assert result["next_retry_time"] == "2024-01-01T12:00:00+00:00"
        assert result["first_attempt_time"] == "2024-01-01T11:00:00+00:00"
        assert result["last_error"] == "Timeout error"

    def test_from_dict(self):
        data = {
            "address": "addr123",
            "challenge_id": "ch456",
            "nonce": "nonce789",
            "challenge_data": {"challengeId": "ch456"},
            "attempt_count": 2,
            "next_retry_time": "2024-01-01T12:00:00+00:00",
            "first_attempt_time": "2024-01-01T11:00:00+00:00",
            "last_error": "Error message"
        }
        item = SubmissionRetryItem.from_dict(data)
        assert item.address == "addr123"
        assert item.challenge_id == "ch456"
        assert item.attempt_count == 2
        assert item.next_retry_time == datetime(2024, 1, 1, 12, 0, 0, tzinfo=timezone.utc)
        assert item.first_attempt_time == datetime(2024, 1, 1, 11, 0, 0, tzinfo=timezone.utc)

    def test_roundtrip_serialization(self, sample_item):
        data = sample_item.to_dict()
        restored = SubmissionRetryItem.from_dict(data)
        assert restored.address == sample_item.address
        assert restored.challenge_id == sample_item.challenge_id
        assert restored.attempt_count == sample_item.attempt_count
        assert restored.next_retry_time == sample_item.next_retry_time


class TestSubmissionRetryManager:
    """Test SubmissionRetryManager class"""

    @pytest.fixture
    def temp_journal(self, tmp_path):
        return str(tmp_path / "test_retry.journal")

    @pytest.fixture
    def mock_db_manager(self):
        manager = Mock()
        manager.update_challenge = Mock()
        return manager

    @pytest.fixture
    def manager(self, mock_db_manager, temp_journal):
        return SubmissionRetryManager(mock_db_manager, temp_journal)

    @pytest.fixture
    def sample_retry_item(self):
        return SubmissionRetryItem(
            address="addr123",
            challenge_id="ch456",
            nonce="nonce789",
            challenge_data={
                "challengeId": "ch456",
                "latestSubmission": "2024-12-31T23:59:59Z"
            },
            attempt_count=1,
            next_retry_time=datetime.now(timezone.utc) + timedelta(seconds=10),
            first_attempt_time=datetime.now(timezone.utc),
            last_error="Timeout"
        )

    def test_initialization(self, manager):
        assert manager._queue is not None
        assert manager._lock is not None
        assert manager._active_retries == {}
        assert manager._stats["total_queued"] == 0
        assert manager._stats["total_success"] == 0

    def test_add_retry_success(self, manager, sample_retry_item):
        result = manager.add_retry(sample_retry_item)
        assert result is True
        assert "addr123:ch456" in manager._active_retries
        assert manager._stats["total_queued"] == 1

    def test_add_retry_duplicate_returns_false(self, manager, sample_retry_item):
        manager.add_retry(sample_retry_item)
        result = manager.add_retry(sample_retry_item)
        assert result is False
        assert manager._stats["total_queued"] == 1

    def test_get_next_ready_retry_returns_item(self, manager, sample_retry_item):
        past_time = datetime.now(timezone.utc) - timedelta(seconds=1)
        sample_retry_item.next_retry_time = past_time
        manager.add_retry(sample_retry_item)

        item = manager.get_next_ready_retry(timeout=0.1)
        assert item is not None
        assert item.challenge_id == "ch456"

    def test_get_next_ready_retry_waits_for_future_item(self, manager, sample_retry_item):
        future_time = datetime.now(timezone.utc) + timedelta(seconds=10)
        sample_retry_item.next_retry_time = future_time
        manager.add_retry(sample_retry_item)

        item = manager.get_next_ready_retry(timeout=0.1)
        assert item is None

    def test_get_next_ready_retry_empty_queue(self, manager):
        item = manager.get_next_ready_retry(timeout=0.1)
        assert item is None

    def test_update_retry_attempt(self, manager, sample_retry_item):
        manager.add_retry(sample_retry_item)
        sample_retry_item.attempt_count = 2
        sample_retry_item.last_error = "New error"

        manager.update_retry_attempt(sample_retry_item)

        key = "addr123:ch456"
        assert manager._active_retries[key].attempt_count == 2
        assert manager._active_retries[key].last_error == "New error"

    def test_remove_retry_success(self, manager, sample_retry_item):
        manager.add_retry(sample_retry_item)
        manager.remove_retry("addr123", "ch456", "success")

        assert "addr123:ch456" not in manager._active_retries
        assert manager._stats["total_success"] == 1

    def test_remove_retry_failed(self, manager, sample_retry_item):
        manager.add_retry(sample_retry_item)
        manager.remove_retry("addr123", "ch456", "failed")

        assert "addr123:ch456" not in manager._active_retries
        assert manager._stats["total_failed"] == 1

    def test_remove_retry_expired(self, manager, sample_retry_item):
        manager.add_retry(sample_retry_item)
        manager.remove_retry("addr123", "ch456", "expired")

        assert "addr123:ch456" not in manager._active_retries
        assert manager._stats["total_expired"] == 1

    def test_remove_retry_already_submitted(self, manager, sample_retry_item):
        manager.add_retry(sample_retry_item)
        manager.remove_retry("addr123", "ch456", "already_submitted")

        assert "addr123:ch456" not in manager._active_retries
        assert manager._stats["total_success"] == 1

    def test_get_stats(self, manager, sample_retry_item):
        manager.add_retry(sample_retry_item)
        manager.remove_retry("addr123", "ch456", "success")

        stats = manager.get_stats()
        assert stats["total_queued"] == 1
        assert stats["total_success"] == 1
        assert stats["current_queue_depth"] == 0

    def test_save_snapshot(self, manager, sample_retry_item, temp_journal):
        manager.add_retry(sample_retry_item)
        manager.save_snapshot()

        assert Path(temp_journal).exists()
        with open(temp_journal, "r") as f:
            lines = f.readlines()
            assert len(lines) > 0
            entry = json.loads(lines[0])
            assert entry["action"] == "add_retry"
            assert entry["payload"]["challenge_id"] == "ch456"

    def test_load_from_journal_recovers_items(self, mock_db_manager, temp_journal):
        item = SubmissionRetryItem(
            address="addr123",
            challenge_id="ch456",
            nonce="nonce789",
            challenge_data={
                "challengeId": "ch456",
                "latestSubmission": "2099-12-31T23:59:59Z"
            },
            attempt_count=1,
            next_retry_time=datetime.now(timezone.utc) + timedelta(seconds=10),
            first_attempt_time=datetime.now(timezone.utc),
            last_error="Error"
        )

        with open(temp_journal, "w") as f:
            entry = {
                "ts": datetime.now(timezone.utc).isoformat(),
                "action": "add_retry",
                "payload": item.to_dict()
            }
            f.write(json.dumps(entry) + "\n")

        manager = SubmissionRetryManager(mock_db_manager, temp_journal)
        assert "addr123:ch456" in manager._active_retries

    def test_load_from_journal_expires_old_items(self, mock_db_manager, temp_journal):
        item = SubmissionRetryItem(
            address="addr123",
            challenge_id="ch456",
            nonce="nonce789",
            challenge_data={
                "challengeId": "ch456",
                "latestSubmission": "2020-01-01T00:00:00Z"
            },
            attempt_count=1,
            next_retry_time=datetime.now(timezone.utc),
            first_attempt_time=datetime.now(timezone.utc),
            last_error="Error"
        )

        with open(temp_journal, "w") as f:
            entry = {
                "ts": datetime.now(timezone.utc).isoformat(),
                "action": "add_retry",
                "payload": item.to_dict()
            }
            f.write(json.dumps(entry) + "\n")

        manager = SubmissionRetryManager(mock_db_manager, temp_journal)
        assert "addr123:ch456" not in manager._active_retries
        mock_db_manager.update_challenge.assert_called_once_with(
            "addr123", "ch456", {"status": "expired"}
        )

    def test_load_from_journal_handles_malformed_entries(
        self, mock_db_manager, temp_journal, caplog
    ):
        with open(temp_journal, "w") as f:
            f.write("{invalid json\n")
            f.write(json.dumps({"action": "add_retry", "payload": {}}) + "\n")

        manager = SubmissionRetryManager(mock_db_manager, temp_journal)
        assert len(manager._active_retries) == 0

    def test_load_from_journal_adjusts_past_retry_times(
        self, mock_db_manager, temp_journal
    ):
        """
        Test that items loaded from journal with past next_retry_time are adjusted
        and immediately available for retry. This simulates the scenario where the
        program was stopped for hours and has items past their expected retry time.
        """
        past_time = datetime.now(timezone.utc) - timedelta(minutes=10)
        item = SubmissionRetryItem(
            address="addr123",
            challenge_id="ch456",
            nonce="nonce789",
            challenge_data={
                "challengeId": "ch456",
                "latestSubmission": "2099-12-31T23:59:59Z"
            },
            attempt_count=1,
            next_retry_time=past_time,
            first_attempt_time=datetime.now(timezone.utc) - timedelta(minutes=15),
            last_error="Timeout"
        )

        with open(temp_journal, "w") as f:
            entry = {
                "ts": datetime.now(timezone.utc).isoformat(),
                "action": "add_retry",
                "payload": item.to_dict()
            }
            f.write(json.dumps(entry) + "\n")

        manager = SubmissionRetryManager(mock_db_manager, temp_journal)

        assert "addr123:ch456" in manager._active_retries

        retrieved_item = manager.get_next_ready_retry(timeout=0.1)
        assert retrieved_item is not None
        assert retrieved_item.challenge_id == "ch456"

        now = datetime.now(timezone.utc)
        assert retrieved_item.next_retry_time < now
        assert (now - retrieved_item.next_retry_time).total_seconds() < 5

    def test_thread_safety_add_retry(self, manager, sample_retry_item):
        results = []

        def add_item():
            result = manager.add_retry(sample_retry_item)
            results.append(result)

        threads = [threading.Thread(target=add_item) for _ in range(10)]
        for t in threads:
            t.start()
        for t in threads:
            t.join()

        assert results.count(True) == 1
        assert results.count(False) == 9

    def test_multiple_addresses_same_challenge(self, manager):
        """
        Test that multiple addresses can retry the same challenge_id simultaneously.
        This is the critical test for the composite key fix.
        """
        item1 = SubmissionRetryItem(
            address="addr1",
            challenge_id="**D18C11",
            nonce="0x001",
            challenge_data={
                "challengeId": "**D18C11",
                "latestSubmission": "2099-12-31T23:59:59Z"
            },
            attempt_count=1,
            next_retry_time=datetime.now(timezone.utc) + timedelta(seconds=10),
            first_attempt_time=datetime.now(timezone.utc),
            last_error="Timeout"
        )

        item2 = SubmissionRetryItem(
            address="addr2",
            challenge_id="**D18C11",
            nonce="0x002",
            challenge_data={
                "challengeId": "**D18C11",
                "latestSubmission": "2099-12-31T23:59:59Z"
            },
            attempt_count=1,
            next_retry_time=datetime.now(timezone.utc) + timedelta(seconds=15),
            first_attempt_time=datetime.now(timezone.utc),
            last_error="Connection error"
        )

        result1 = manager.add_retry(item1)
        result2 = manager.add_retry(item2)

        assert result1 is True
        assert result2 is True

        assert "addr1:**D18C11" in manager._active_retries
        assert "addr2:**D18C11" in manager._active_retries
        assert manager._active_retries["addr1:**D18C11"].nonce == "0x001"
        assert manager._active_retries["addr2:**D18C11"].nonce == "0x002"
        assert manager._stats["total_queued"] == 2

        manager.remove_retry("addr1", "**D18C11", "success")
        assert "addr1:**D18C11" not in manager._active_retries
        assert "addr2:**D18C11" in manager._active_retries

    def test_journal_replay_multiple_addresses_same_challenge(
        self, mock_db_manager, temp_journal
    ):
        """
        Test that journal replay correctly handles multiple addresses retrying the same challenge.
        This verifies the fix works after process restart.
        """
        item1 = SubmissionRetryItem(
            address="addr1",
            challenge_id="**D18C11",
            nonce="0x001",
            challenge_data={
                "challengeId": "**D18C11",
                "latestSubmission": "2099-12-31T23:59:59Z"
            },
            attempt_count=1,
            next_retry_time=datetime.now(timezone.utc) + timedelta(seconds=10),
            first_attempt_time=datetime.now(timezone.utc),
            last_error="Timeout"
        )

        item2 = SubmissionRetryItem(
            address="addr2",
            challenge_id="**D18C11",
            nonce="0x002",
            challenge_data={
                "challengeId": "**D18C11",
                "latestSubmission": "2099-12-31T23:59:59Z"
            },
            attempt_count=2,
            next_retry_time=datetime.now(timezone.utc) + timedelta(seconds=20),
            first_attempt_time=datetime.now(timezone.utc) - timedelta(seconds=30),
            last_error="HTTP Error 500"
        )

        with open(temp_journal, "w") as f:
            entry1 = {
                "ts": datetime.now(timezone.utc).isoformat(),
                "action": "add_retry",
                "payload": item1.to_dict()
            }
            entry2 = {
                "ts": datetime.now(timezone.utc).isoformat(),
                "action": "add_retry",
                "payload": item2.to_dict()
            }
            f.write(json.dumps(entry1) + "\n")
            f.write(json.dumps(entry2) + "\n")

        manager = SubmissionRetryManager(mock_db_manager, temp_journal)

        assert "addr1:**D18C11" in manager._active_retries
        assert "addr2:**D18C11" in manager._active_retries
        assert manager._active_retries["addr1:**D18C11"].nonce == "0x001"
        assert manager._active_retries["addr2:**D18C11"].nonce == "0x002"
        assert manager._active_retries["addr1:**D18C11"].attempt_count == 1
        assert manager._active_retries["addr2:**D18C11"].attempt_count == 2


class TestFormatSuccessMessages:
    """Test _format_success_messages() function"""

    def test_validated_message(self):
        msgs = _format_success_messages("validated", "ch123", 3)
        assert len(msgs) == 4
        assert "✅ Retry successful!" in msgs[1]
        assert "🎉 Successfully validated" in msgs[2]
        assert "ch123" in msgs[2]
        assert "3 attempts" in msgs[2]

    def test_solved_message(self):
        msgs = _format_success_messages("solved", "ch456", 2)
        assert len(msgs) == 4
        assert "✅ Retry successful!" in msgs[1]
        assert "✅ Successfully solved" in msgs[2]
        assert "ch456" in msgs[2]
        assert "2 attempts" in msgs[2]

    def test_already_exists_message(self):
        msgs = _format_success_messages("already_exists", "ch789", 1)
        assert len(msgs) == 5
        assert "-----------------------------------------------" in msgs[0]
        assert "✅ Solution was previously submitted" in msgs[1]
        assert "⚠️  Crypto receipt not available" in msgs[2]
        assert "📝 Challenge marked as 'solved'" in msgs[3]
        assert "attempt 1" in msgs[3]
        assert "-----------------------------------------------" in msgs[4]

    def test_unknown_status_returns_empty(self):
        msgs = _format_success_messages("unknown_status", "ch999", 1)
        assert msgs == []


class TestRetryWorker:
    """Test retry_worker() function"""

    @pytest.fixture
    def mock_db_manager(self):
        return Mock()

    @pytest.fixture
    def mock_retry_manager(self):
        manager = Mock()
        manager.get_next_ready_retry = Mock(return_value=None)
        manager.remove_retry = Mock()
        manager.update_retry_attempt = Mock()
        manager._queue = Mock()
        return manager

    @pytest.fixture
    def mock_tui_app(self):
        return Mock()

    @pytest.fixture
    def mock_session(self):
        return Mock()

    @pytest.fixture
    def stop_event(self):
        event = threading.Event()
        return event

    @pytest.fixture
    def retry_config(self):
        return {
            "initial_delay": 5,
            "max_delay": 60,
            "max_attempts": 5,
            "backoff_multiplier": 2.0,
        }

    @pytest.fixture
    def sample_retry_item(self):
        return SubmissionRetryItem(
            address="addr123",
            challenge_id="ch456",
            nonce="nonce789",
            challenge_data={
                "challengeId": "ch456",
                "latestSubmission": "2099-12-31T23:59:59Z"
            },
            attempt_count=1,
            next_retry_time=datetime.now(timezone.utc),
            first_attempt_time=datetime.now(timezone.utc),
            last_error="Timeout"
        )

    @patch("retry.submit_solution")
    def test_worker_stops_when_event_set(
        self, mock_submit, mock_db_manager, mock_retry_manager,
        mock_tui_app, mock_session, stop_event, retry_config
    ):
        stop_event.set()
        retry_worker(
            mock_db_manager, mock_retry_manager, stop_event,
            mock_tui_app, mock_session, retry_config
        )
        mock_tui_app.post_message.assert_called()

    @patch("retry.submit_solution")
    def test_worker_handles_expired_challenge(
        self, mock_submit, mock_db_manager, mock_retry_manager,
        mock_tui_app, mock_session, stop_event, retry_config
    ):
        expired_item = SubmissionRetryItem(
            address="addr123",
            challenge_id="ch456",
            nonce="nonce789",
            challenge_data={
                "challengeId": "ch456",
                "latestSubmission": "2020-01-01T00:00:00Z"
            },
            attempt_count=1,
            next_retry_time=datetime.now(timezone.utc),
            first_attempt_time=datetime.now(timezone.utc),
            last_error="Timeout"
        )

        call_count = [0]
        def side_effect_func(*args, **kwargs):
            call_count[0] += 1
            if call_count[0] == 1:
                return expired_item
            stop_event.set()
            return None

        mock_retry_manager.get_next_ready_retry.side_effect = side_effect_func

        retry_worker(
            mock_db_manager, mock_retry_manager, stop_event,
            mock_tui_app, mock_session, retry_config
        )

        mock_retry_manager.remove_retry.assert_called_with("addr123", "ch456", "expired")
        mock_db_manager.update_challenge.assert_called()

    @patch("retry.submit_solution")
    def test_worker_handles_validated_result(
        self, mock_submit, mock_db_manager, mock_retry_manager,
        mock_tui_app, mock_session, stop_event, retry_config, sample_retry_item
    ):
        mock_submit.return_value = SubmissionResult(
            "validated",
            {"status": "validated", "salt": "nonce789"}
        )

        call_count = [0]
        def side_effect_func(*args, **kwargs):
            call_count[0] += 1
            if call_count[0] == 1:
                return sample_retry_item
            stop_event.set()
            return None

        mock_retry_manager.get_next_ready_retry.side_effect = side_effect_func

        retry_worker(
            mock_db_manager, mock_retry_manager, stop_event,
            mock_tui_app, mock_session, retry_config
        )

        mock_retry_manager.remove_retry.assert_called_with("addr123", "ch456", "success")
        mock_db_manager.update_challenge.assert_called()

    @patch("retry.submit_solution")
    def test_worker_handles_already_exists_result(
        self, mock_submit, mock_db_manager, mock_retry_manager,
        mock_tui_app, mock_session, stop_event, retry_config, sample_retry_item
    ):
        mock_submit.return_value = SubmissionResult(
            "already_exists",
            {"status": "solved", "salt": "nonce789"}
        )

        call_count = [0]
        def side_effect_func(*args, **kwargs):
            call_count[0] += 1
            if call_count[0] == 1:
                return sample_retry_item
            stop_event.set()
            return None

        mock_retry_manager.get_next_ready_retry.side_effect = side_effect_func

        retry_worker(
            mock_db_manager, mock_retry_manager, stop_event,
            mock_tui_app, mock_session, retry_config
        )

        mock_retry_manager.remove_retry.assert_called_with("addr123", "ch456", "already_submitted")

    @patch("retry.submit_solution")
    def test_worker_handles_max_retries_exhausted(
        self, mock_submit, mock_db_manager, mock_retry_manager,
        mock_tui_app, mock_session, stop_event, retry_config, sample_retry_item
    ):
        sample_retry_item.attempt_count = 4
        mock_submit.return_value = SubmissionResult(
            "should_retry", None, "Timeout error"
        )

        call_count = [0]
        def side_effect_func(*args, **kwargs):
            call_count[0] += 1
            if call_count[0] == 1:
                return sample_retry_item
            stop_event.set()
            return None

        mock_retry_manager.get_next_ready_retry.side_effect = side_effect_func

        retry_worker(
            mock_db_manager, mock_retry_manager, stop_event,
            mock_tui_app, mock_session, retry_config
        )

        mock_retry_manager.remove_retry.assert_called_with("addr123", "ch456", "failed")
        mock_db_manager.update_challenge.assert_called_with(
            "addr123", "ch456", {"status": "submission_failed"}
        )

    @patch("retry.submit_solution")
    def test_worker_requeues_for_retry(
        self, mock_submit, mock_db_manager, mock_retry_manager,
        mock_tui_app, mock_session, stop_event, retry_config, sample_retry_item
    ):
        mock_submit.return_value = SubmissionResult(
            "should_retry", None, "Timeout error"
        )

        call_count = [0]
        def side_effect_func(*args, **kwargs):
            call_count[0] += 1
            if call_count[0] == 1:
                return sample_retry_item
            stop_event.set()
            return None

        mock_retry_manager.get_next_ready_retry.side_effect = side_effect_func

        retry_worker(
            mock_db_manager, mock_retry_manager, stop_event,
            mock_tui_app, mock_session, retry_config
        )

        mock_retry_manager.update_retry_attempt.assert_called()
        mock_retry_manager._queue.put.assert_called()

    @patch("retry.submit_solution")
    def test_worker_handles_non_retryable_error(
        self, mock_submit, mock_db_manager, mock_retry_manager,
        mock_tui_app, mock_session, stop_event, retry_config, sample_retry_item
    ):
        mock_submit.return_value = SubmissionResult(
            "failed", None, "Invalid request"
        )

        call_count = [0]
        def side_effect_func(*args, **kwargs):
            call_count[0] += 1
            if call_count[0] == 1:
                return sample_retry_item
            stop_event.set()
            return None

        mock_retry_manager.get_next_ready_retry.side_effect = side_effect_func

        retry_worker(
            mock_db_manager, mock_retry_manager, stop_event,
            mock_tui_app, mock_session, retry_config
        )

        mock_retry_manager.remove_retry.assert_called_with("addr123", "ch456", "failed")

    @patch("retry.submit_solution")
    def test_worker_handles_unexpected_status(
        self, mock_submit, mock_db_manager, mock_retry_manager,
        mock_tui_app, mock_session, stop_event, retry_config, sample_retry_item
    ):
        mock_submit.return_value = SubmissionResult("unknown_status", None, None)

        call_count = [0]
        def side_effect_func(*args, **kwargs):
            call_count[0] += 1
            if call_count[0] == 1:
                return sample_retry_item
            stop_event.set()
            return None

        mock_retry_manager.get_next_ready_retry.side_effect = side_effect_func

        retry_worker(
            mock_db_manager, mock_retry_manager, stop_event,
            mock_tui_app, mock_session, retry_config
        )

        mock_retry_manager.remove_retry.assert_called_with("addr123", "ch456", "failed")

