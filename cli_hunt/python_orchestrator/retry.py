import logging
import queue
import threading
from dataclasses import dataclass, asdict
from datetime import datetime, timezone, timedelta

from curl_cffi import requests

from journal import Journal
from submission import submit_solution
from tui import (
    LogMessage,
    ChallengeUpdate,
    SolutionFound,
    RetryAttempting,
    RetrySuccess,
    RetryFailed,
    RetryExpired,
)


@dataclass
class SubmissionRetryItem:
    address: str
    challenge_id: str
    nonce: str
    challenge_data: dict
    attempt_count: int
    next_retry_time: datetime
    first_attempt_time: datetime
    last_error: str

    def __lt__(self, other):
        return self.next_retry_time < other.next_retry_time

    def to_dict(self) -> dict:
        d = asdict(self)
        d["next_retry_time"] = self.next_retry_time.isoformat()
        d["first_attempt_time"] = self.first_attempt_time.isoformat()
        return d

    @classmethod
    def from_dict(cls, data: dict):
        data = data.copy()
        data["next_retry_time"] = datetime.fromisoformat(data["next_retry_time"])
        data["first_attempt_time"] = datetime.fromisoformat(data["first_attempt_time"])
        return cls(**data)


class SubmissionRetryManager:
    def __init__(self, db_manager, journal_file: str):
        self._queue = queue.PriorityQueue()
        self._lock = threading.Lock()
        self._active_retries = {}
        self._db_manager = db_manager
        self._stats = {
            "total_queued": 0,
            "total_success": 0,
            "total_failed": 0,
            "total_expired": 0,
        }
        self._journal = Journal(journal_file)
        self._load_from_journal()

    @staticmethod
    def _make_key(address: str, challenge_id: str) -> str:
        """
        Create composite key from address and challenge_id to support
        multiple addresses retrying same challenge.
        """
        return f"{address}:{challenge_id}"

    def _load_from_journal(self):
        retry_items = {}

        def handler(action, payload):
            challenge_id = payload.get("challenge_id")
            address = payload.get("address")
            if not challenge_id or not address:
                return

            key = self._make_key(address, challenge_id)

            if action == "add_retry":
                retry_items[key] = payload
            elif action == "retry_attempt":
                if key in retry_items:
                    retry_items[key].update(payload)
            elif action in [
                "retry_success", "retry_failed", "retry_expired", "retry_already_submitted"
            ]:
                retry_items.pop(key, None)

        replayed = self._journal.replay(handler)

        if replayed > 0:
            logging.info(f"Replaying retry queue journal ({replayed} entries)...")

        now = datetime.now(timezone.utc)
        recovered = 0
        expired = 0

        for key, item_data in retry_items.items():
            try:
                challenge_id = item_data["challenge_id"]
                address = item_data["address"]
                latest_submission = datetime.fromisoformat(
                    item_data["challenge_data"]["latestSubmission"].replace("Z", "+00:00")
                )

                if now > latest_submission:
                    expired += 1
                    logging.info(f"Discarding expired retry item for {address}:{challenge_id}")
                    self._db_manager.update_challenge(
                        address, challenge_id, {"status": "expired"}
                    )
                    continue

                retry_item = SubmissionRetryItem.from_dict(item_data)

                if retry_item.next_retry_time < now:
                    retry_item.next_retry_time = now - timedelta(seconds=1)

                self._queue.put((retry_item.next_retry_time, retry_item))
                self._active_retries[key] = retry_item
                recovered += 1
            except (KeyError, ValueError) as e:
                logging.warning(f"Error recovering retry item {key}: {e}")

        if recovered > 0 or expired > 0:
            logging.info(f"Recovered {recovered} retry items from journal ({expired} expired)")

    def add_retry(self, item: SubmissionRetryItem) -> bool:
        with self._lock:
            key = self._make_key(item.address, item.challenge_id)
            if key in self._active_retries:
                return False

            self._journal.append("add_retry", item.to_dict())
            self._queue.put((item.next_retry_time, item))
            self._active_retries[key] = item
            self._stats["total_queued"] += 1
            return True

    def get_next_ready_retry(self, timeout=1.0):
        try:
            next_retry_time, item = self._queue.get(timeout=timeout)
            now = datetime.now(timezone.utc)

            if next_retry_time > now:
                wait_seconds = (next_retry_time - now).total_seconds()
                sleep_duration = min(wait_seconds, timeout)

                self._queue.put((next_retry_time, item))

                import time
                time.sleep(sleep_duration)
                return None

            return item
        except queue.Empty:
            return None

    def update_retry_attempt(self, item: SubmissionRetryItem):
        with self._lock:
            self._journal.append(
                "retry_attempt",
                {
                    "address": item.address,
                    "challenge_id": item.challenge_id,
                    "attempt_count": item.attempt_count,
                    "next_retry_time": item.next_retry_time.isoformat(),
                    "last_error": item.last_error,
                },
            )
            key = self._make_key(item.address, item.challenge_id)
            self._active_retries[key] = item

    def remove_retry(self, address: str, challenge_id: str, reason: str):
        with self._lock:
            key = self._make_key(address, challenge_id)
            self._active_retries.pop(key, None)
            self._journal.append(
                f"retry_{reason}",
                {"address": address, "challenge_id": challenge_id}
            )

            if reason in {"success", "already_submitted"}:
                self._stats["total_success"] += 1
            elif reason == "failed":
                self._stats["total_failed"] += 1
            elif reason == "expired":
                self._stats["total_expired"] += 1

    def get_stats(self) -> dict:
        with self._lock:
            return {
                **self._stats,
                "current_queue_depth": len(self._active_retries),
            }

    def save_snapshot(self):
        def serializer():
            entries = []
            for item in self._active_retries.values():
                entries.append({
                    "ts": datetime.now(timezone.utc).isoformat(),
                    "action": "add_retry",
                    "payload": item.to_dict(),
                })
            return entries

        self._journal.snapshot(serializer)


def _format_success_messages(result_status: str, challenge_id: str, attempts: int) -> list:
    messages = {
        "validated": [
            "-----------------------------------------------",
            f"✅ Retry successful! Solution submitted for {challenge_id}",
            f"🎉 Successfully validated challenge {challenge_id} (after {attempts} attempts)",
            "-----------------------------------------------",
        ],
        "solved": [
            "-----------------------------------------------",
            f"✅ Retry successful! Solution submitted for {challenge_id}",
            f"✅ Successfully solved challenge {challenge_id} (after {attempts} attempts)",
            "-----------------------------------------------",
        ],
        "already_exists": [
            "-----------------------------------------------",
            f"✅ Solution was previously submitted for {challenge_id}",
            "⚠️  Crypto receipt not available (server accepted but client lost response)",
            f"📝 Challenge marked as 'solved' - submission confirmed via retry (attempt {attempts})",
            "-----------------------------------------------",
        ]
    }
    return messages.get(result_status, [])


def retry_worker(
    db_manager,
    retry_manager: SubmissionRetryManager,
    stop_event,
    tui_app,
    session: requests.Session,
    retry_config: dict,
):
    tui_app.post_message(LogMessage("Retry worker thread started."))

    while not stop_event.is_set():
        retry_item = retry_manager.get_next_ready_retry(timeout=1.0)
        if retry_item is None:
            continue

        c = retry_item.challenge_data
        challenge_id = c["challengeId"]
        now = datetime.now(timezone.utc)
        latest_submission = datetime.fromisoformat(c["latestSubmission"].replace("Z", "+00:00"))

        if now > latest_submission:
            tui_app.post_message(LogMessage([
                "-----------------------------------------------",
                f"⏳ Challenge {challenge_id} expired while in retry queue",
                "⏳ Solution DISCARDED - submission deadline passed",
                "-----------------------------------------------",
            ]))
            retry_manager.remove_retry(retry_item.address, challenge_id, "expired")
            db_manager.update_challenge(retry_item.address, challenge_id, {"status": "expired"})
            tui_app.post_message(ChallengeUpdate(retry_item.address, challenge_id, "expired"))
            tui_app.post_message(RetryExpired(retry_item.address, challenge_id))
            continue

        attempt_num = retry_item.attempt_count + 1
        max_attempts = retry_config['max_attempts']
        tui_app.post_message(LogMessage(
            f"🔄 Retrying submission for {challenge_id} "
            f"(attempt {attempt_num}/{max_attempts})"
        ))

        result = submit_solution(
            retry_item.address, challenge_id, retry_item.nonce, session, timeout=30
        )

        if result.status in ("validated", "solved", "already_exists"):
            is_already = result.status == "already_exists"
            final_status = "solved" if is_already else result.status
            reason = "already_submitted" if is_already else "success"

            msgs = _format_success_messages(result.status, challenge_id, attempt_num)
            tui_app.post_message(LogMessage(msgs))
            tui_app.post_message(SolutionFound())
            retry_manager.remove_retry(retry_item.address, challenge_id, reason)
            db_manager.update_challenge(retry_item.address, challenge_id, result.update)
            tui_app.post_message(
                ChallengeUpdate(retry_item.address, challenge_id, final_status)
            )
            tui_app.post_message(
                RetrySuccess(retry_item.address, challenge_id, attempt_num)
            )

        elif result.status == "should_retry":
            max_attempts = retry_config["max_attempts"]
            if attempt_num >= max_attempts:
                elapsed = (now - retry_item.first_attempt_time).total_seconds() / 60
                tui_app.post_message(LogMessage([
                    "-----------------------------------------------",
                    f"🔄 Retrying submission for {challenge_id} "
                    f"(attempt {attempt_num}/{max_attempts} - FINAL)",
                    f"⚠️  Retry attempt {attempt_num} failed: {result.error}",
                    f"💀 All retry attempts exhausted for {challenge_id}",
                    f"💀 Solution LOST - max attempts ({max_attempts}) "
                    f"reached after {elapsed:.1f} minutes",
                    "-----------------------------------------------",
                ]))
                retry_manager.remove_retry(retry_item.address, challenge_id, "failed")
                db_manager.update_challenge(
                    retry_item.address, challenge_id, {"status": "submission_failed"}
                )
                tui_app.post_message(
                    ChallengeUpdate(retry_item.address, challenge_id, "submission_failed")
                )
                tui_app.post_message(
                    RetryFailed(retry_item.address, challenge_id, result.error)
                )
            else:
                retry_item.attempt_count += 1
                base_delay = retry_config["initial_delay"]
                multiplier = retry_config["backoff_multiplier"]
                delay = min(
                    base_delay * (multiplier ** (retry_item.attempt_count - 1)),
                    retry_config["max_delay"],
                )
                retry_item.next_retry_time = now + timedelta(seconds=delay)
                retry_item.last_error = result.error

                next_attempt = retry_item.attempt_count + 1
                tui_app.post_message(LogMessage([
                    f"⚠️  Retry attempt {retry_item.attempt_count} failed: "
                    f"{result.error}",
                    f"🔄 Requeued for retry (attempt {next_attempt}/{max_attempts}, "
                    f"next retry in {delay:.0f}s)",
                ]))
                retry_manager.update_retry_attempt(retry_item)
                retry_manager._queue.put((retry_item.next_retry_time, retry_item))
                tui_app.post_message(
                    RetryAttempting(retry_item.address, challenge_id,
                                    retry_item.attempt_count, delay)
                )

        elif result.status == "failed":
            tui_app.post_message(LogMessage([
                "-----------------------------------------------",
                f"❗️ Non-retryable error for {challenge_id}: {result.error}",
                "❌ Stopping retry attempts - error will not resolve",
                "-----------------------------------------------",
            ]))
            retry_manager.remove_retry(retry_item.address, challenge_id, "failed")
            db_manager.update_challenge(
                retry_item.address, challenge_id, {"status": "submission_failed"}
            )
            tui_app.post_message(
                ChallengeUpdate(retry_item.address, challenge_id, "submission_failed")
            )
            tui_app.post_message(
                RetryFailed(retry_item.address, challenge_id, result.error)
            )

        else:
            msg = f"⚠️  Unexpected submission result status: {result.status}"
            tui_app.post_message(LogMessage(msg))
            retry_manager.remove_retry(retry_item.address, challenge_id, "failed")
            db_manager.update_challenge(
                retry_item.address, challenge_id, {"status": "submission_failed"}
            )
            tui_app.post_message(
                ChallengeUpdate(retry_item.address, challenge_id, "submission_failed")
            )

    logging.info("Retry worker thread stopped.")
