import sys
import unittest
from decimal import Decimal
from pathlib import Path
from unittest import mock


sys.path.insert(0, str(Path(__file__).resolve().parent))
import tpa_realtime_fwa_demo  # noqa: E402


class TpaRealtimeFwaDemoTest(unittest.TestCase):
    def test_claim_amount_prefers_claim_amount(self):
        payload = {
            "reportCase": {
                "claimAmount": 100000,
                "policyList": [{"invoiceList": [{"feeAmount": 200}]}],
            }
        }

        self.assertEqual(
            tpa_realtime_fwa_demo.claim_amount_from_payload(payload),
            Decimal("100000"),
        )

    def test_claim_amount_falls_back_to_invoice_total(self):
        payload = {
            "reportCase": {
                "policyList": [
                    {"invoiceList": [{"feeAmount": "120.50"}, {"feeAmount": 30}]},
                    {"invoiceList": [{"feeAmount": "9.50"}]},
                ]
            }
        }

        self.assertEqual(
            tpa_realtime_fwa_demo.claim_amount_from_payload(payload),
            Decimal("160.00"),
        )

    def test_make_repeatable_payload_changes_copy_only(self):
        payload = {"transNo": "txn-1", "reportCase": {"reportNo": "CLM-1"}}

        updated = tpa_realtime_fwa_demo.make_repeatable_payload(payload, "123")

        self.assertEqual(updated["transNo"], "txn-1-RT-123")
        self.assertEqual(updated["reportCase"]["reportNo"], "CLM-1-RT-123")
        self.assertEqual(payload["transNo"], "txn-1")
        self.assertEqual(payload["reportCase"]["reportNo"], "CLM-1")

    def test_apply_source_system_changes_copy_only(self):
        payload = {"systemCode": "tpa-demo", "reportCase": {"reportNo": "CLM-1"}}

        updated = tpa_realtime_fwa_demo.apply_source_system(payload, "AiClaim Core")

        self.assertEqual(updated["systemCode"], "AiClaim Core")
        self.assertEqual(payload["systemCode"], "tpa-demo")

    def test_string_evidence_refs_ignores_structured_refs(self):
        refs = [
            "audit:score-1",
            {"type": "rule", "id": "HIGH_AMOUNT"},
            "",
            123,
        ]

        self.assertEqual(
            tpa_realtime_fwa_demo.string_evidence_refs(refs),
            ["audit:score-1"],
        )

    def test_run_demo_completes_business_chain(self):
        args = argparse_namespace(
            base_url="http://api.test",
            web_url="http://web.test",
            api_key="dev-secret",
            source_system=None,
            payload_file="/tmp/payload.json",
            inbox_correction_file=None,
            unique_message=False,
        )
        payload = {
            "systemCode": "tpa-demo",
            "transNo": "txn-1",
            "reportCase": {"reportNo": "CLM-1", "claimAmount": 100000},
        }
        observed = {
            "fwa_response": "scored_and_routed",
            "normalize": {"run_id": "inbox:1"},
            "score": {
                "run_id": "score-1",
                "audit_id": "audit-score-1",
                "claim_id": "CLM-1",
                "risk_score": 90,
                "rag": "Red",
                "recommended_action": "ManualReview",
                "decision_outcome": "manual_review",
                "evidence_refs": ["rule_runs:HIGH_AMOUNT"],
            },
        }
        dashboard = {
            "suspected_claims": 1,
            "confirmed_fwa": 1,
            "saving_amount": "100000.00",
            "value_measurement": {
                "prevented_payment": "100000.00",
                "recovered_amount": "0.00",
                "estimated_impact": "0.00",
                "net_value": "99970.00",
                "evidence_caveat": "observed",
            },
        }
        lead = {
            "lead_id": "lead-1",
            "claim_id": "CLM-1",
            "run_id": "score-1",
            "evidence_refs": ["leads:lead-1"],
        }
        responses = {
            ("GET", "/api/v1/ops/dashboard/summary"): dashboard,
            ("GET", "/api/v1/ops/leads"): {"leads": [lead]},
            ("POST", "/api/v1/ops/leads/lead-1/triage"): {
                "audit_id": "audit-triage-1",
                "case": {"case_id": "case-1"},
            },
            ("POST", "/api/v1/ops/cases/case-1/status"): {
                "case": {"status": "investigating"}
            },
            ("POST", "/api/v1/investigations/results"): {
                "audit_id": "audit-investigation-1",
                "idempotency_key": "tpa-writeback:investigation.result.received:audit-investigation-1",
            },
        }

        def fake_request(_base_url, _api_key, method, path, payload=None):
            if path == "/api/v1/investigations/results":
                self.assertEqual(payload["financial_impact_type"], "prevented_payment")
                self.assertEqual(payload["saving_amount"], "100000.00")
            return responses[(method, path)]

        with (
            mock.patch.object(tpa_realtime_fwa_demo, "load_payload", return_value=payload),
            mock.patch.object(tpa_realtime_fwa_demo, "observe_fwa_response", return_value=observed),
            mock.patch.object(tpa_realtime_fwa_demo, "request", side_effect=fake_request),
            mock.patch.object(tpa_realtime_fwa_demo.time, "time", return_value=1770000000),
        ):
            summary = tpa_realtime_fwa_demo.run_demo(args)

        self.assertEqual(summary["status"], "completed")
        self.assertEqual(summary["business_story"]["claim_amount"], "100000.00")
        self.assertEqual(summary["workflow"]["case_id"], "case-1")
        self.assertEqual(summary["dashboard_after"]["prevented_payment"], "100000.00")
        self.assertEqual(summary["ui_targets"]["cases"], "http://web.test/#leads-cases")

    def test_format_story_reads_like_customer_demo_cue_card(self):
        summary = {
            "status": "completed",
            "business_story": {
                "claim_amount": "100000.00",
                "financial_impact_type": "prevented_payment",
            },
            "claim": {
                "claim_id": "CLM-1",
                "risk_score": 90,
                "rag": "Red",
                "recommended_action": "ManualReview",
                "decision_outcome": "manual_review",
            },
            "workflow": {
                "inbox_run_id": "inbox:1",
                "score_run_id": "score-1",
                "lead_id": "lead-1",
                "case_id": "case-1",
                "case_status": "investigating",
                "investigation_audit_id": "audit-investigation-1",
                "writeback_idempotency_key": "writeback-1",
            },
            "dashboard_before": {"prevented_payment": "0.00"},
            "dashboard_after": {
                "prevented_payment": "100000.00",
                "saving_amount": "100000.00",
            },
            "ui_targets": {
                "intake": "http://web.test/#intake-ops",
                "cases": "http://web.test/#leads-cases",
                "dashboard": "http://web.test/#dashboard",
            },
        }

        story = tpa_realtime_fwa_demo.format_story(summary)

        self.assertIn("Live TPA FWA demo complete", story)
        self.assertIn("TPA packet received", story)
        self.assertIn("Lead became an investigation case", story)
        self.assertIn("Prevented payment: 0.00 -> 100000.00", story)
        self.assertIn("http://web.test/#leads-cases", story)


def argparse_namespace(**kwargs):
    return type("Args", (), kwargs)()


if __name__ == "__main__":
    unittest.main()
