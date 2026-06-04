import io
import json
import sys
import unittest
from pathlib import Path
from unittest import mock


sys.path.insert(0, str(Path(__file__).resolve().parent))
import stream_tpa_demo_cases  # noqa: E402


def base_payload():
    return {
        "systemCode": "AiClaim Core",
        "transNo": "base-txn",
        "reportCase": {
            "id": 1,
            "reportNo": "BASE-RPT",
            "calculateRisk": "N",
            "accidentReason": "outpatient",
            "medicalRecordInfoList": [{"patientName": "", "diagnosisName": "牙周炎"}],
            "policyList": [
                {
                    "coverageLimit": 20000,
                    "invoiceList": [
                        {
                            "invoiceNo": "INV-1",
                            "invoiceBusinessId": "INV-1-BIZ",
                            "accidentPersonName": "王向龙",
                            "feeAmount": 397.06,
                            "redFlag": "N",
                            "feeList": [
                                {
                                    "feeAmount": 300.0,
                                    "feeDetailList": [{"amount": 300.0}],
                                },
                                {
                                    "feeAmount": 97.06,
                                    "feeDetailList": [{"amount": 97.06}],
                                },
                            ],
                        }
                    ],
                }
            ],
        },
    }


class StreamTpaDemoCasesTest(unittest.TestCase):
    def test_build_demo_case_sets_unique_case_identity(self):
        payload = stream_tpa_demo_cases.build_demo_case(
            base_payload(), "request_evidence_dental", 42
        )

        self.assertEqual(payload["systemCode"], "AiClaim Core")
        self.assertEqual(
            payload["reportCase"]["reportNo"], "TPA-DEMO-REQUEST-EVIDENCE-DENTAL-00042"
        )
        self.assertIn("REQUEST-EVIDENCE-DENTAL-00042", payload["transNo"])
        self.assertEqual(
            stream_tpa_demo_cases.first_invoice(payload)["invoiceNo"],
            "TPA-DEMO-INV-00042",
        )

    def test_high_amount_case_raises_amount_and_keeps_coverage(self):
        payload = stream_tpa_demo_cases.build_demo_case(
            base_payload(), "manual_review_high_amount", 7
        )

        policy = payload["reportCase"]["policyList"][0]
        self.assertEqual(policy["coverageLimit"], 20000)
        self.assertEqual(payload["reportCase"]["claimAmount"], 18600.0)
        self.assertEqual(stream_tpa_demo_cases.first_invoice(payload)["feeAmount"], 18600.0)
        self.assertEqual(stream_tpa_demo_cases.first_invoice(payload)["redFlag"], "Y")

    def test_low_amount_case_uses_routine_items_and_claim_amount(self):
        payload = stream_tpa_demo_cases.build_demo_case(
            base_payload(), "low_risk_request_evidence", 5
        )

        invoice = stream_tpa_demo_cases.first_invoice(payload)
        details = stream_tpa_demo_cases.first_fee_details(payload)
        self.assertEqual(payload["reportCase"]["claimAmount"], 180.0)
        self.assertEqual(invoice["feeList"][0]["feeCategory"], "consultation")
        self.assertEqual(len(details), 3)
        self.assertLess(max(detail["amount"] for detail in details) / 180.0, 0.5)

    def test_missing_coverage_case_blocks_intake(self):
        payload = stream_tpa_demo_cases.build_demo_case(
            base_payload(), "intake_block_missing_coverage", 3
        )

        self.assertNotIn("coverageLimit", payload["reportCase"]["policyList"][0])
        self.assertEqual(payload["reportCase"]["calculateRisk"], "N")

    def test_actual_bucket_maps_score_actions(self):
        self.assertEqual(
            stream_tpa_demo_cases.actual_bucket(
                {"fwa_response": "scored_and_routed", "score": {"recommended_action": "ManualReview"}}
            ),
            "manual_review",
        )
        self.assertEqual(
            stream_tpa_demo_cases.actual_bucket(
                {"fwa_response": "accepted_for_audit_but_not_ready_for_scoring"}
            ),
            "intake_blocked",
        )

    def test_run_stream_dry_run_outputs_json_lines(self):
        args = argparse_namespace(
            payload_file="/tmp/req.json",
            inbox_correction_file=None,
            case=["intake_reject_source_mismatch"],
            sequence_start=10,
            iterations=1,
            emit_dir=None,
            dry_run=True,
            interval_seconds=0,
            base_url="http://127.0.0.1:8080",
            api_key="aiclaim-demo-key",
            skip_audit=True,
        )

        with (
            mock.patch.object(stream_tpa_demo_cases, "load_base_payload", return_value=base_payload()),
            mock.patch("sys.stdout", new_callable=io.StringIO) as stdout,
        ):
            results = stream_tpa_demo_cases.run_stream(args)

        self.assertEqual(len(results), 1)
        body = json.loads(stdout.getvalue())
        self.assertEqual(body["case_kind"], "intake_reject_source_mismatch")
        self.assertEqual(body["payload_brief"]["systemCode"], "Wrong TPA")


def argparse_namespace(**kwargs):
    return type("Args", (), kwargs)()


if __name__ == "__main__":
    unittest.main()
