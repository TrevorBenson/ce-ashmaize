import json
import logging
import os
import threading
from contextlib import suppress
from datetime import datetime, timezone
from pathlib import Path
from typing import Callable


class Journal:
    def __init__(self, journal_file: str):
        self.journal_file = Path(journal_file)
        self._lock = threading.Lock()

    def append(self, action: str, payload: dict):
        try:
            with open(self.journal_file, "a") as f:
                entry = {
                    "ts": datetime.now(timezone.utc).isoformat(),
                    "action": action,
                    "payload": payload,
                }
                f.write(json.dumps(entry) + "\n")
        except IOError as e:
            logging.error(f"Failed to write to journal {self.journal_file}: {e}")

    def replay(self, handler: Callable[[str, dict], None]):
        if not self.journal_file.exists():
            return 0

        replayed_count = 0
        try:
            with open(self.journal_file, "r") as f:
                for line in f:
                    try:
                        entry = json.loads(line)
                        action = entry.get("action")
                        if action is None:
                            logging.warning("Skipping journal entry with missing 'action' key")
                            continue
                        handler(action, entry.get("payload"))
                        replayed_count += 1
                    except (json.JSONDecodeError, KeyError) as e:
                        logging.warning(f"Skipping malformed journal entry: {e}")
        except IOError as e:
            logging.error(f"Error reading journal {self.journal_file}: {e}")

        return replayed_count

    def clear(self):
        with suppress(IOError):
            if self.journal_file.exists():
                self.journal_file.write_text("")

    def snapshot(self, serializer: Callable[[], list[dict]]):
        with self._lock:
            try:
                temp_file = Path(f"{self.journal_file}.tmp")
                entries = serializer()

                with open(temp_file, "w") as f:
                    for entry in entries:
                        f.write(json.dumps(entry) + "\n")

                temp_file.replace(self.journal_file)
                with suppress(OSError):
                    os.chmod(self.journal_file, 0o600)

                logging.info(f"Journal snapshot saved: {self.journal_file}")
            except IOError as e:
                logging.error(f"Failed to save journal snapshot {self.journal_file}: {e}")
