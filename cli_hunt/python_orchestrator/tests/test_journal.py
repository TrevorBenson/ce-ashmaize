"""
Unit tests for journal.py module

Tests cover:
- Journal class and all its methods
- Thread safety
- Error handling
- File operations
"""

import json
import logging
import os
import threading
from pathlib import Path
from unittest.mock import Mock, patch, mock_open

import pytest

from journal import Journal


class TestJournalInit:
    """Test Journal initialization"""

    def test_init_with_string_path(self, tmp_path):
        journal_path = str(tmp_path / "test.journal")
        journal = Journal(journal_path)
        assert journal.journal_file == Path(journal_path)
        assert journal._lock is not None

    def test_init_with_pathlib_path(self, tmp_path):
        journal_path = tmp_path / "test.journal"
        journal = Journal(str(journal_path))
        assert isinstance(journal.journal_file, Path)

    def test_init_creates_lock(self, tmp_path):
        journal = Journal(str(tmp_path / "test.journal"))
        assert isinstance(journal._lock, type(threading.Lock()))


class TestJournalAppend:
    """Test Journal.append() method"""

    @pytest.fixture
    def journal(self, tmp_path):
        return Journal(str(tmp_path / "test.journal"))

    def test_append_writes_to_file(self, journal):
        journal.append("test_action", {"key": "value"})

        assert journal.journal_file.exists()
        with open(journal.journal_file, "r") as f:
            line = f.readline()
            entry = json.loads(line)
            assert entry["action"] == "test_action"
            assert entry["payload"] == {"key": "value"}
            assert "ts" in entry

    def test_append_multiple_entries(self, journal):
        journal.append("action1", {"data": 1})
        journal.append("action2", {"data": 2})
        journal.append("action3", {"data": 3})

        with open(journal.journal_file, "r") as f:
            lines = f.readlines()
            assert len(lines) == 3
            assert json.loads(lines[0])["action"] == "action1"
            assert json.loads(lines[1])["action"] == "action2"
            assert json.loads(lines[2])["action"] == "action3"

    def test_append_complex_payload(self, journal):
        complex_payload = {
            "nested": {"key": "value"},
            "list": [1, 2, 3],
            "string": "test",
            "number": 42,
            "bool": True
        }
        journal.append("complex_action", complex_payload)

        with open(journal.journal_file, "r") as f:
            entry = json.loads(f.readline())
            assert entry["payload"] == complex_payload

    def test_append_handles_ioerror(self, journal, caplog):
        with patch("builtins.open", side_effect=IOError("Disk full")):
            with caplog.at_level(logging.ERROR):
                journal.append("action", {"key": "value"})

            assert "Failed to write to journal" in caplog.text
            assert "Disk full" in caplog.text

    def test_append_timestamp_format(self, journal):
        journal.append("test_action", {})

        with open(journal.journal_file, "r") as f:
            entry = json.loads(f.readline())
            assert "ts" in entry
            assert "T" in entry["ts"]
            assert "+" in entry["ts"] or "Z" in entry["ts"]

    def test_append_is_thread_safe(self, journal):
        def append_entries(thread_id):
            for i in range(10):
                journal.append(f"action_{thread_id}", {"thread": thread_id, "i": i})

        threads = [threading.Thread(target=append_entries, args=(i,)) for i in range(5)]
        for t in threads:
            t.start()
        for t in threads:
            t.join()

        with open(journal.journal_file, "r") as f:
            lines = f.readlines()
            assert len(lines) == 50


class TestJournalReplay:
    """Test Journal.replay() method"""

    @pytest.fixture
    def journal(self, tmp_path):
        return Journal(str(tmp_path / "test.journal"))

    def test_replay_empty_journal(self, journal):
        handler = Mock()
        count = journal.replay(handler)

        assert count == 0
        handler.assert_not_called()

    def test_replay_nonexistent_file(self, journal):
        handler = Mock()
        count = journal.replay(handler)

        assert count == 0
        handler.assert_not_called()

    def test_replay_calls_handler_for_each_entry(self, journal):
        journal.append("action1", {"data": 1})
        journal.append("action2", {"data": 2})
        journal.append("action3", {"data": 3})

        handler = Mock()
        count = journal.replay(handler)

        assert count == 3
        assert handler.call_count == 3
        handler.assert_any_call("action1", {"data": 1})
        handler.assert_any_call("action2", {"data": 2})
        handler.assert_any_call("action3", {"data": 3})

    def test_replay_handles_malformed_json(self, journal, caplog):
        with open(journal.journal_file, "w") as f:
            f.write('{"action": "valid", "payload": {}}\n')
            f.write('{invalid json\n')
            f.write('{"action": "valid2", "payload": {}}\n')

        handler = Mock()
        with caplog.at_level(logging.WARNING):
            count = journal.replay(handler)

        assert count == 2
        assert handler.call_count == 2
        assert "Skipping malformed journal entry" in caplog.text

    def test_replay_handles_missing_keys(self, journal, caplog):
        with open(journal.journal_file, "w") as f:
            f.write('{"action": "valid", "payload": {}}\n')
            f.write('{"missing_action": "test"}\n')
            f.write('{"action": "valid2", "payload": {}}\n')

        handler = Mock()
        with caplog.at_level(logging.WARNING):
            count = journal.replay(handler)

        assert count == 2
        assert handler.call_count == 2

    def test_replay_handles_ioerror(self, journal, caplog):
        journal.append("action", {})

        with patch("builtins.open", side_effect=IOError("Permission denied")):
            with caplog.at_level(logging.ERROR):
                count = journal.replay(Mock())

            assert count == 0
            assert "Error reading journal" in caplog.text

    def test_replay_returns_correct_count(self, journal):
        for i in range(100):
            journal.append(f"action_{i}", {"index": i})

        count = journal.replay(Mock())
        assert count == 100

    def test_replay_handler_receives_correct_data(self, journal):
        payload = {"key": "value", "nested": {"a": 1, "b": 2}}
        journal.append("test_action", payload)

        results = []

        def handler(action, payload):
            results.append((action, payload))

        journal.replay(handler)

        assert len(results) == 1
        assert results[0][0] == "test_action"
        assert results[0][1] == payload


class TestJournalClear:
    """Test Journal.clear() method"""

    @pytest.fixture
    def journal(self, tmp_path):
        return Journal(str(tmp_path / "test.journal"))

    def test_clear_removes_content(self, journal):
        journal.append("action1", {"data": 1})
        journal.append("action2", {"data": 2})

        assert journal.journal_file.exists()
        assert journal.journal_file.stat().st_size > 0

        journal.clear()

        assert journal.journal_file.exists()
        assert journal.journal_file.stat().st_size == 0

    def test_clear_nonexistent_file_doesnt_error(self, journal):
        journal.clear()

    def test_clear_handles_ioerror_silently(self, journal):
        journal.append("action", {})

        with patch.object(Path, "write_text", side_effect=IOError("Permission denied")):
            journal.clear()

    def test_clear_after_clear_is_idempotent(self, journal):
        journal.append("action", {})
        journal.clear()
        journal.clear()

        assert journal.journal_file.stat().st_size == 0


class TestJournalSnapshot:
    """Test Journal.snapshot() method"""

    @pytest.fixture
    def journal(self, tmp_path):
        return Journal(str(tmp_path / "test.journal"))

    def test_snapshot_writes_entries(self, journal):
        entries = [
            {"ts": "2024-01-01T00:00:00Z", "action": "action1", "payload": {"data": 1}},
            {"ts": "2024-01-01T00:00:01Z", "action": "action2", "payload": {"data": 2}},
        ]

        def serializer():
            return entries

        journal.snapshot(serializer)

        assert journal.journal_file.exists()
        with open(journal.journal_file, "r") as f:
            lines = f.readlines()
            assert len(lines) == 2
            assert json.loads(lines[0]) == entries[0]
            assert json.loads(lines[1]) == entries[1]

    def test_snapshot_overwrites_existing(self, journal):
        journal.append("old_action", {"data": "old"})

        entries = [{"ts": "2024-01-01T00:00:00Z", "action": "new_action", "payload": {"data": "new"}}]
        journal.snapshot(lambda: entries)

        with open(journal.journal_file, "r") as f:
            lines = f.readlines()
            assert len(lines) == 1
            assert json.loads(lines[0])["action"] == "new_action"

    def test_snapshot_uses_temp_file(self, journal):
        entries = [{"ts": "2024-01-01T00:00:00Z", "action": "test", "payload": {}}]

        with patch("pathlib.Path.replace") as mock_replace:
            journal.snapshot(lambda: entries)
            mock_replace.assert_called_once()

    def test_snapshot_sets_permissions(self, journal):
        entries = [{"ts": "2024-01-01T00:00:00Z", "action": "test", "payload": {}}]

        with patch("os.chmod") as mock_chmod:
            journal.snapshot(lambda: entries)
            mock_chmod.assert_called_with(journal.journal_file, 0o600)

    def test_snapshot_handles_chmod_error_gracefully(self, journal):
        entries = [{"ts": "2024-01-01T00:00:00Z", "action": "test", "payload": {}}]

        with patch("os.chmod", side_effect=OSError("Permission denied")):
            journal.snapshot(lambda: entries)

        assert journal.journal_file.exists()

    def test_snapshot_handles_ioerror(self, journal, caplog):
        def serializer():
            return [{"action": "test", "payload": {}}]

        with patch("builtins.open", side_effect=IOError("Disk full")):
            with caplog.at_level(logging.ERROR):
                journal.snapshot(serializer)

            assert "Failed to save journal snapshot" in caplog.text

    def test_snapshot_logs_success(self, journal, caplog):
        entries = [{"ts": "2024-01-01T00:00:00Z", "action": "test", "payload": {}}]

        with caplog.at_level(logging.INFO):
            journal.snapshot(lambda: entries)

        assert "Journal snapshot saved" in caplog.text

    def test_snapshot_empty_list(self, journal):
        journal.snapshot(lambda: [])

        assert journal.journal_file.exists()
        assert journal.journal_file.stat().st_size == 0

    def test_snapshot_is_thread_safe(self, journal):
        def do_snapshot(thread_id):
            entries = [
                {"ts": f"2024-01-01T00:00:{thread_id:02d}Z", "action": f"action_{thread_id}", "payload": {}}
            ]
            journal.snapshot(lambda: entries)

        threads = [threading.Thread(target=do_snapshot, args=(i,)) for i in range(10)]
        for t in threads:
            t.start()
        for t in threads:
            t.join()

        assert journal.journal_file.exists()


class TestJournalIntegration:
    """Integration tests for Journal"""

    @pytest.fixture
    def journal(self, tmp_path):
        return Journal(str(tmp_path / "test.journal"))

    def test_append_replay_roundtrip(self, journal):
        test_data = [
            ("action1", {"data": 1, "value": "test1"}),
            ("action2", {"data": 2, "value": "test2"}),
            ("action3", {"data": 3, "value": "test3"}),
        ]

        for action, payload in test_data:
            journal.append(action, payload)

        results = []

        def handler(action, payload):
            results.append((action, payload))

        count = journal.replay(handler)

        assert count == 3
        assert len(results) == 3
        for i, (action, payload) in enumerate(results):
            assert action == test_data[i][0]
            assert payload == test_data[i][1]

    def test_snapshot_then_replay(self, journal):
        entries = [
            {"ts": "2024-01-01T00:00:00Z", "action": "snapshot_action", "payload": {"key": "value"}},
            {"ts": "2024-01-01T00:00:01Z", "action": "another_action", "payload": {"key2": "value2"}},
        ]

        journal.snapshot(lambda: entries)

        results = []

        def handler(action, payload):
            results.append((action, payload))

        journal.replay(handler)

        assert len(results) == 2
        assert results[0][0] == "snapshot_action"
        assert results[1][0] == "another_action"

    def test_append_clear_replay(self, journal):
        journal.append("action1", {"data": 1})
        journal.append("action2", {"data": 2})
        journal.clear()

        count = journal.replay(Mock())
        assert count == 0

    def test_concurrent_append_and_snapshot(self, journal):
        def append_many():
            for i in range(50):
                journal.append(f"action_{i}", {"i": i})

        def take_snapshot():
            entries = [
                {"ts": "2024-01-01T00:00:00Z", "action": "snapshot", "payload": {}}
            ]
            journal.snapshot(lambda: entries)

        append_thread = threading.Thread(target=append_many)
        snapshot_thread = threading.Thread(target=take_snapshot)

        append_thread.start()
        snapshot_thread.start()
        append_thread.join()
        snapshot_thread.join()

        assert journal.journal_file.exists()

