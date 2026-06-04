import json
import sys
import tempfile
import unittest
from pathlib import Path


sys.path.insert(0, str(Path(__file__).resolve().parent))
import build_tpa_demo_dataset  # noqa: E402


class TpaDemoDatasetTest(unittest.TestCase):
    def test_builds_rule_funnel_demo_payloads_and_manifest(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            output_dir = Path(tmpdir) / "tpa-demo"
            manifest = build_tpa_demo_dataset.build_dataset(output_dir, overwrite=False)

            self.assertEqual(manifest["dataset_key"], "tpa_rule_funnel_demo")
            self.assertEqual(len(manifest["cases"]), 7)

            expected = json.loads((output_dir / "expected_outcomes.json").read_text())
            self.assertEqual(
                expected["pending_dental_xray"]["required_evidence"],
                ["dental_xray", "medical_record"],
            )
            self.assertEqual(
                expected["pending_prescription_detail"]["required_evidence"],
                ["medication_order", "prescription_detail"],
            )
            self.assertEqual(
                expected["pending_operation_record"]["required_evidence"],
                ["operation_record", "medical_record", "invoice"],
            )
            self.assertFalse(
                expected["inbox_missing_coverage_limit"]["normalize_scoring_ready"]
            )

            dental_payload = json.loads(
                (
                    output_dir
                    / "direct_scoring_payloads"
                    / "pending_dental_xray.json"
                ).read_text()
            )
            self.assertEqual(dental_payload["items"][0]["item_type"], "dental")

            inbox_blocker = json.loads(
                (
                    output_dir
                    / "inbox_payloads"
                    / "inbox_missing_coverage_limit.json"
                ).read_text()
            )
            self.assertNotIn(
                "coverageLimit",
                inbox_blocker["reportCase"]["policyList"][0],
            )

    def test_rejects_non_empty_output_without_overwrite(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            output_dir = Path(tmpdir) / "tpa-demo"
            output_dir.mkdir()
            (output_dir / "existing.txt").write_text("existing", encoding="utf-8")

            with self.assertRaises(FileExistsError):
                build_tpa_demo_dataset.build_dataset(output_dir, overwrite=False)


if __name__ == "__main__":
    unittest.main()
