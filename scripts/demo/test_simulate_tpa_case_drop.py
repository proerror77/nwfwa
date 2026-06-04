import unittest
import sys
from pathlib import Path
from unittest import mock


sys.path.insert(0, str(Path(__file__).resolve().parent))
from simulate_tpa_case_drop import (
    observe_fwa_response,
    payload_brief,
    with_unique_trans_no,
)


class SimulateTpaCaseDropTest(unittest.TestCase):
    def test_payload_brief_summarizes_raw_tpa_case(self):
        payload = {
            "systemCode": "AiClaim Core",
            "transNo": "txn-1",
            "reportCase": {
                "id": 1,
                "reportNo": "RPT-1",
                "calculateRisk": "N",
                "medicalRecordInfoList": [
                    {
                        "hospitalName": "南京同仁医院",
                        "departmentName": "口腔科",
                        "diagnosisName": "牙周炎",
                        "medicalType": "门诊",
                    }
                ],
                "policyList": [
                    {
                        "invoiceList": [
                            {
                                "invoiceNo": "INV-1",
                                "hospitalName": "南京同仁医院",
                                "feeAmount": 397.06,
                            }
                        ]
                    }
                ],
            },
        }

        brief = payload_brief(payload)

        self.assertEqual(brief["system_code"], "AiClaim Core")
        self.assertEqual(brief["report_no"], "RPT-1")
        self.assertEqual(brief["invoice_count"], 1)
        self.assertEqual(brief["invoice_total_amount"], 397.06)

    def test_observe_stops_when_normalize_is_not_scoring_ready(self):
        payload = {
            "systemCode": "AiClaim Core",
            "reportCase": {"reportNo": "RPT-1", "policyList": []},
        }
        normalize_response = {
            "validation_result": "accepted_with_warnings",
            "scoring_ready": False,
            "validation_errors": [
                {
                    "field_path": "reportCase.policyList[0].coverageLimit",
                    "severity": "warning",
                    "remediation": "map coverage limit",
                }
            ],
        }

        with mock.patch(
            "simulate_tpa_case_drop.request",
            return_value=normalize_response,
        ) as request_mock:
            summary = observe_fwa_response(
                "http://127.0.0.1:8080",
                "dev-secret",
                payload,
                include_audit=True,
            )

        self.assertEqual(
            summary["fwa_response"], "accepted_for_audit_but_not_ready_for_scoring"
        )
        self.assertIsNone(summary["score"])
        self.assertEqual(request_mock.call_count, 1)
        self.assertTrue(summary["normalize"]["validation_errors"][0]["blocks_scoring"])

    def test_observe_marks_rejected_normalize_as_intake_rejection(self):
        payload = {
            "systemCode": "AiClaim Core",
            "reportCase": {"reportNo": "RPT-1", "policyList": []},
        }
        normalize_response = {
            "validation_result": "rejected",
            "scoring_ready": False,
            "validation_errors": [
                {
                    "field_path": "systemCode",
                    "severity": "error",
                    "remediation": "source mismatch",
                }
            ],
        }

        with mock.patch(
            "simulate_tpa_case_drop.request",
            return_value=normalize_response,
        ):
            summary = observe_fwa_response(
                "http://127.0.0.1:8080",
                "dev-secret",
                payload,
                include_audit=False,
            )

        self.assertEqual(summary["fwa_response"], "rejected_at_intake")
        self.assertIsNone(summary["score"])

    def test_observe_scores_with_inbox_handoff_when_normalize_is_ready(self):
        payload = {
            "systemCode": "AiClaim Core",
            "reportCase": {"reportNo": "RPT-1", "policyList": []},
        }
        normalize_response = {
            "validation_result": "accepted",
            "scoring_ready": True,
            "run_id": "inbox:abc",
            "audit_id": "audit-inbox",
            "idempotency_key": "inbox-key",
            "validation_errors": [],
        }
        score_response = {
            "run_id": "score-run",
            "audit_id": "audit-score",
            "claim_id": "RPT-1",
            "risk_score": 87,
            "rag": "Red",
            "recommended_action": "ManualReview",
            "decision_outcome": "assistive_routing_only",
            "decision_authority": "human_review_required",
            "top_reasons": ["high amount"],
            "evidence_refs": ["audit:audit-inbox"],
        }
        audit_response = {
            "claim_id": "RPT-1",
            "events": [
                {"event_type": "inbox.claim.normalized"},
                {"event_type": "scoring.completed"},
            ],
        }

        with mock.patch(
            "simulate_tpa_case_drop.request",
            side_effect=[normalize_response, score_response, audit_response],
        ) as request_mock:
            summary = observe_fwa_response(
                "http://127.0.0.1:8080",
                "dev-secret",
                payload,
                include_audit=True,
            )

        self.assertEqual(summary["fwa_response"], "scored_and_routed")
        self.assertEqual(summary["score"]["risk_score"], 87)
        self.assertEqual(summary["audit"]["event_types"], [
            "inbox.claim.normalized",
            "scoring.completed",
        ])
        score_call = request_mock.call_args_list[1].args
        self.assertEqual(score_call[2], "POST")
        self.assertEqual(score_call[3], "/api/v1/claims/score")
        self.assertEqual(
            score_call[4],
            {"source_system": "AiClaim Core", "inbox_run_id": "inbox:abc"},
        )

    def test_with_unique_trans_no_changes_copy_only(self):
        payload = {"transNo": "txn-1", "reportCase": {"reportNo": "RPT-1"}}

        with mock.patch("simulate_tpa_case_drop.time.time", return_value=1770000000):
            updated = with_unique_trans_no(payload)

        self.assertEqual(updated["transNo"], "txn-1-sim-1770000000")
        self.assertEqual(payload["transNo"], "txn-1")


if __name__ == "__main__":
    unittest.main()
