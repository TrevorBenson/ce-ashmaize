import unittest
from datetime import datetime
from unittest.mock import MagicMock, patch

from tui import LogMessage, OrchestratorTUI


class TestLogMessage(unittest.TestCase):
    def test_logmessage_accepts_string(self):
        msg = LogMessage("Test message")
        self.assertEqual(msg.message, ["Test message"])

    def test_logmessage_accepts_list(self):
        messages = ["Line 1", "Line 2", "Line 3"]
        msg = LogMessage(messages)
        self.assertEqual(msg.message, messages)

    def test_logmessage_list_is_not_copied(self):
        messages = ["Line 1", "Line 2"]
        msg = LogMessage(messages)
        self.assertIs(msg.message, messages)


class TestTUILogOutput(unittest.TestCase):
    def setUp(self):
        self.mock_db = MagicMock()
        self.mock_retry = MagicMock()
        self.mock_db.get_addresses.return_value = []
        self.mock_db.get_all_wallet_statistics.return_value = ({}, {})

        self.tui = OrchestratorTUI(
            db_manager=self.mock_db,
            retry_manager=self.mock_retry,
            worker_functions={},
            worker_args={},
        )

        self.tui.log_widget = MagicMock()

    @patch('tui.datetime')
    def test_single_string_message_output(self, mock_datetime):
        mock_datetime.now.return_value.strftime.return_value = "12:34:56"

        msg = LogMessage("Test message")
        self.tui.on_log_message(msg)

        self.tui.log_widget.write_line.assert_called_once_with(
            "[12:34:56] Test message"
        )

    @patch('tui.datetime')
    def test_list_message_output_same_timestamp(self, mock_datetime):
        mock_datetime.now.return_value.strftime.return_value = "12:34:56"

        messages = ["Line 1", "Line 2", "Line 3"]
        msg = LogMessage(messages)
        self.tui.on_log_message(msg)

        expected_calls = [
            unittest.mock.call("[12:34:56] Line 1"),
            unittest.mock.call("[12:34:56] Line 2"),
            unittest.mock.call("[12:34:56] Line 3"),
        ]
        self.tui.log_widget.write_line.assert_has_calls(expected_calls)
        self.assertEqual(self.tui.log_widget.write_line.call_count, 3)

    @patch('tui.datetime')
    def test_multiple_calls_vs_single_list_call(self, mock_datetime):
        timestamps = ["12:34:56", "12:34:57", "12:34:58"]
        timestamp_iter = iter(timestamps)
        mock_datetime.now.return_value.strftime.side_effect = (
            lambda fmt: next(timestamp_iter)
        )

        log_widget_separate = MagicMock()
        self.tui.log_widget = log_widget_separate

        self.tui.on_log_message(LogMessage("Line 1"))
        self.tui.on_log_message(LogMessage("Line 2"))
        self.tui.on_log_message(LogMessage("Line 3"))

        separate_calls = [
            "[12:34:56] Line 1",
            "[12:34:57] Line 2",
            "[12:34:58] Line 3",
        ]
        actual_separate = [
            call[0][0] for call in log_widget_separate.write_line.call_args_list
        ]
        self.assertEqual(actual_separate, separate_calls)

        timestamp_iter = iter(["12:34:59"])
        mock_datetime.now.return_value.strftime.side_effect = (
            lambda fmt: next(timestamp_iter)
        )

        log_widget_list = MagicMock()
        self.tui.log_widget = log_widget_list

        self.tui.on_log_message(LogMessage(["Line 1", "Line 2", "Line 3"]))

        list_calls = [
            "[12:34:59] Line 1",
            "[12:34:59] Line 2",
            "[12:34:59] Line 3",
        ]
        actual_list = [
            call[0][0] for call in log_widget_list.write_line.call_args_list
        ]
        self.assertEqual(actual_list, list_calls)

        for timestamp in list_calls:
            self.assertTrue(timestamp.startswith("[12:34:59]"))

    @patch('tui.datetime')
    @patch('tui.logging')
    def test_logging_integration(self, mock_logging, mock_datetime):
        mock_datetime.now.return_value.strftime.return_value = "12:34:56"

        messages = ["Line 1", "Line 2"]
        msg = LogMessage(messages)
        self.tui.on_log_message(msg)

        expected_logging_calls = [
            unittest.mock.call("Line 1"),
            unittest.mock.call("Line 2"),
        ]
        mock_logging.info.assert_has_calls(expected_logging_calls)

    @patch('tui.datetime')
    def test_empty_list_message(self, mock_datetime):
        mock_datetime.now.return_value.strftime.return_value = "12:34:56"

        msg = LogMessage([])
        self.tui.on_log_message(msg)

        self.tui.log_widget.write_line.assert_not_called()


class TestTimestampBehaviorComparison(unittest.TestCase):
    """
    Demonstrates the key difference between separate calls and list calls:
    - Separate calls: Each line gets a different timestamp (when logged at different times)
    - List calls: All lines share the same timestamp (atomic logging)
    """

    def setUp(self):
        self.mock_db = MagicMock()
        self.mock_retry = MagicMock()
        self.mock_db.get_addresses.return_value = []
        self.mock_db.get_all_wallet_statistics.return_value = ({}, {})

        self.tui = OrchestratorTUI(
            db_manager=self.mock_db,
            retry_manager=self.mock_retry,
            worker_functions={},
            worker_args={},
        )
        self.tui.log_widget = MagicMock()

    @patch('tui.datetime')
    def test_separate_calls_show_different_timestamps(self, mock_datetime):
        """OLD WAY: Multiple post_message calls = different timestamps"""
        mock_datetime.now.return_value.strftime.side_effect = [
            "10:00:01",
            "10:00:02",
            "10:00:03",
            "10:00:04",
        ]

        self.tui.on_log_message(LogMessage("-----------------------------------------------"))
        self.tui.on_log_message(LogMessage("Found nonce: 0x123abc"))
        self.tui.on_log_message(LogMessage("Solved in 45.32 seconds"))
        self.tui.on_log_message(LogMessage("-----------------------------------------------"))

        output = [
            call[0][0] for call in self.tui.log_widget.write_line.call_args_list
        ]

        print("\n=== Separate Calls (OLD WAY) ===")
        for line in output:
            print(line)

        self.assertEqual(
            output,
            [
                "[10:00:01] -----------------------------------------------",
                "[10:00:02] Found nonce: 0x123abc",
                "[10:00:03] Solved in 45.32 seconds",
                "[10:00:04] -----------------------------------------------",
            ],
        )

        timestamps = [line[1:9] for line in output]
        self.assertEqual(len(set(timestamps)), 4)

    @patch('tui.datetime')
    def test_list_call_shows_same_timestamp(self, mock_datetime):
        """NEW WAY: Single post_message with list = same timestamp for all lines"""
        mock_datetime.now.return_value.strftime.return_value = "10:00:05"

        self.tui.on_log_message(
            LogMessage([
                "-----------------------------------------------",
                "Found nonce: 0x123abc",
                "Solved in 45.32 seconds",
                "-----------------------------------------------",
            ])
        )

        output = [
            call[0][0] for call in self.tui.log_widget.write_line.call_args_list
        ]

        print("\n=== Single List Call (NEW WAY) ===")
        for line in output:
            print(line)

        self.assertEqual(
            output,
            [
                "[10:00:05] -----------------------------------------------",
                "[10:00:05] Found nonce: 0x123abc",
                "[10:00:05] Solved in 45.32 seconds",
                "[10:00:05] -----------------------------------------------",
            ],
        )

        timestamps = [line[1:9] for line in output]
        self.assertEqual(len(set(timestamps)), 1)
        self.assertEqual(timestamps[0], "10:00:05")


if __name__ == "__main__":
    unittest.main(verbosity=2)

