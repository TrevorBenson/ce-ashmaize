import json
import logging
from datetime import datetime, timezone
from dataclasses import dataclass

from curl_cffi import requests


@dataclass
class SubmissionResult:
    status: str
    update: dict | None = None
    error: str | None = None


def should_retry_error(error: requests.exceptions.RequestException) -> bool:
    if isinstance(error, (requests.exceptions.Timeout, requests.exceptions.ConnectionError)):
        return True
    if hasattr(error, "response") and error.response is not None:
        status_code = error.response.status_code
        if 500 <= status_code < 600 or status_code == 429:
            return True
        if 400 <= status_code < 500:
            return False
    return False


def check_already_exists_error(error: requests.exceptions.HTTPError) -> bool:
    if getattr(error, "response", None) and error.response.status_code == 400:
        try:
            return "already exists" in error.response.json().get("message", "").lower()
        except Exception:
            pass
    return False


def submit_solution(
    address: str,
    challenge_id: str,
    nonce: str,
    session: requests.Session,
    timeout: int = 10
) -> SubmissionResult:
    try:
        base_url = "https://scavenger.prod.gd.midnighttge.io/solution"
        submit_url = f"{base_url}/{address}/{challenge_id}/{nonce}"
        resp = session.post(submit_url, timeout=timeout)
        resp.raise_for_status()
        now = (datetime.now(timezone.utc)
               .isoformat(timespec="milliseconds")
               .replace("+00:00", "Z"))

        try:
            data = resp.json()
            if data.get("crypto_receipt"):
                update = {
                    "status": "validated",
                    "submittedAt": now,
                    "validatedAt": now,
                    "salt": nonce,
                    "cryptoReceipt": data["crypto_receipt"],
                }
                return SubmissionResult("validated", update)

            update = {"status": "solved", "submittedAt": now, "salt": nonce}
            return SubmissionResult("solved", update)
        except json.JSONDecodeError:
            msg = f"Failed to decode submission response for {challenge_id}"
            logging.warning(msg)
            return SubmissionResult(
                "submission_error",
                {"status": "submission_error", "salt": nonce}
            )

    except (requests.exceptions.HTTPError,
            requests.exceptions.RequestException) as e:
        if (isinstance(e, requests.exceptions.HTTPError) and
                check_already_exists_error(e)):
            now = (datetime.now(timezone.utc)
                   .isoformat(timespec="milliseconds")
                   .replace("+00:00", "Z"))
            update = {"status": "solved", "submittedAt": now, "salt": nonce}
            return SubmissionResult("already_exists", update)

        error_msg = str(e)
        if len(error_msg) > 100:
            error_msg = f"{error_msg[:100]}..."

        status = "should_retry" if should_retry_error(e) else "failed"
        return SubmissionResult(status, None, error_msg)
