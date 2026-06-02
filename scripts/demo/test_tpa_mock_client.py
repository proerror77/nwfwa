import contextlib
import io
import json
import sys
import unittest
import urllib.error
from pathlib import Path
from unittest import mock


sys.path.insert(0, str(Path(__file__).resolve().parent))
import tpa_mock_client  # noqa: E402


class TpaMockClientTest(unittest.TestCase):
    def test_request_can_return_http_error_json_for_normalize_only(self):
        error_body = io.BytesIO(
            b'{"code":"SOURCE_SYSTEM_MISMATCH","message":"source mismatch"}'
        )
        http_error = urllib.error.HTTPError(
            url="http://127.0.0.1:8080/api/v1/inbox/claims/normalize",
            code=400,
            msg="Bad Request",
            hdrs={},
            fp=error_body,
        )

        with mock.patch("urllib.request.urlopen", side_effect=http_error):
            body = tpa_mock_client.request(
                "http://127.0.0.1:8080",
                "dev-secret",
                "POST",
                "/api/v1/inbox/claims/normalize",
                {},
                allow_http_error=True,
            )

        self.assertEqual(body["code"], "SOURCE_SYSTEM_MISMATCH")

    def test_normalize_only_prints_error_response_without_idempotency_key(self):
        with (
            mock.patch.object(
                sys,
                "argv",
                [
                    "tpa_mock_client.py",
                    "--inbox-payload-file",
                    "/tmp/req.json",
                    "--normalize-only",
                ],
            ),
            mock.patch.object(tpa_mock_client, "load_json_file", return_value={"systemCode": "AiClaim Core"}),
            mock.patch.object(
                tpa_mock_client,
                "request",
                return_value={
                    "code": "SOURCE_SYSTEM_MISMATCH",
                    "message": "systemCode must match the authenticated API key source system",
                },
            ),
        ):
            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout):
                exit_code = tpa_mock_client.main()

        self.assertEqual(exit_code, 2)
        body = json.loads(stdout.getvalue())
        self.assertEqual(body["code"], "SOURCE_SYSTEM_MISMATCH")


if __name__ == "__main__":
    unittest.main()
