#!/usr/bin/env python3
"""Build a public-data MVP training manifest.

The generated dataset is for schema, pipeline, demo, and anomaly-flow
validation only. It must not be used as customer production model evidence.
"""

from __future__ import annotations

import argparse
import json
import re
from datetime import date, timedelta
from pathlib import Path
from typing import Any


PUBLIC_SOURCES = [
    {
        "source_key": "cms_medicare_synpuf",
        "display_name": "CMS Medicare Claims Synthetic Public Use Files",
        "official_url": "https://www.cms.gov/data-research/statistics-trends-and-reports/medicare-claims-synthetic-public-use-files",
        "mvp_use": [
            "claims schema familiarization",
            "synthetic claim-level demo data",
            "training and profiling pipeline validation",
        ],
        "limitations": [
            "synthetic data",
            "not customer production evidence",
            "limited inferential research value for Medicare beneficiaries",
        ],
    },
    {
        "source_key": "cms_physician_other_practitioners",
        "display_name": "CMS Medicare Physician & Other Practitioners by Provider",
        "official_url": "https://data.cms.gov/provider-summary-by-type-of-service/medicare-physician-other-practitioners/medicare-physician-other-practitioners-by-provider",
        "mvp_use": [
            "provider utilization baseline",
            "payment peer comparison",
            "provider profile features",
        ],
        "limitations": [
            "provider-level summary",
            "not individual claim truth",
            "no complete fraud label",
        ],
    },
    {
        "source_key": "hhs_oig_leie",
        "display_name": "HHS-OIG List of Excluded Individuals/Entities",
        "official_url": "https://oig.hhs.gov/exclusions/exclusions-list/",
        "mvp_use": [
            "excluded provider and entity screening",
            "deterministic provider risk feature",
        ],
        "limitations": [
            "screening list only",
            "not full FWA detection",
        ],
    },
    {
        "source_key": "cms_coverage_policy",
        "display_name": "CMS Medicare Coverage Database",
        "official_url": "https://www.cms.gov/medicare-coverage-database/search.aspx",
        "mvp_use": [
            "policy citation corpus",
            "medical necessity RAG grounding",
        ],
        "limitations": [
            "must be adapted to customer payer policy",
            "coverage policy is not a fraud label",
        ],
    },
]

PUBLIC_DATA_LABEL_POLICY = "weak_public_data_pipeline_label_not_production_evidence"


def main() -> None:
    args = parse_args()
    pandas = require_pandas()
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    if args.synthetic_fixture:
        claims = synthetic_claim_frame(pandas)
    elif args.synpuf_claims_csv:
        claims = public_claim_frame(
            pandas=pandas,
            claims_csv=Path(args.synpuf_claims_csv),
            provider_summary_csv=Path(args.provider_summary_csv)
            if args.provider_summary_csv
            else None,
            leie_csv=Path(args.leie_csv) if args.leie_csv else None,
        )
    else:
        raise SystemExit("provide --synthetic-fixture or --synpuf-claims-csv")

    write_dataset(pandas, claims, output_dir, args.dataset_version)
    write_sources(output_dir)
    if args.policy_corpus_dir:
        write_policy_corpus(Path(args.policy_corpus_dir), output_dir)

    print(
        json.dumps(
            {
                "manifest_uri": str(output_dir / "manifest.json"),
                "sources_uri": str(output_dir / "sources.json"),
                "dataset_key": "public_data_mvp_claims",
                "dataset_version": args.dataset_version,
                "label_policy": PUBLIC_DATA_LABEL_POLICY,
            },
            indent=2,
            sort_keys=True,
        )
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Materialize a public-data MVP Parquet manifest."
    )
    parser.add_argument(
        "--output-dir",
        default="data/public-mvp",
        help="Directory for manifest, source metadata, and Parquet splits.",
    )
    parser.add_argument(
        "--dataset-version",
        default=date.today().isoformat(),
        help="Dataset version written into manifest.json.",
    )
    parser.add_argument(
        "--synthetic-fixture",
        action="store_true",
        help="Generate a small deterministic fixture without public CSV downloads.",
    )
    parser.add_argument(
        "--synpuf-claims-csv",
        help="Local CMS SynPUF-style claims CSV extract.",
    )
    parser.add_argument(
        "--provider-summary-csv",
        help="Local CMS Physician & Other Practitioners provider summary CSV.",
    )
    parser.add_argument(
        "--leie-csv",
        help="Local HHS-OIG LEIE CSV extract.",
    )
    parser.add_argument(
        "--policy-corpus-dir",
        help="Directory of CMS or payer policy text files to convert to JSONL.",
    )
    return parser.parse_args()


def require_pandas() -> Any:
    try:
        import pandas as pd
    except ImportError as error:
        raise SystemExit(
            "pandas and pyarrow are required; run through apps/ml-service dev environment"
        ) from error
    return pd


def synthetic_claim_frame(pd: Any) -> Any:
    rows = []
    start = date(2026, 1, 1)
    for index in range(30):
        provider_index = index % 6
        member_index = index % 10
        service_date = start + timedelta(days=index * 4)
        amount_ratio = round(0.12 + (index % 9) * 0.11, 4)
        provider_profile_score = 25 + provider_index * 11
        high_cost_item_ratio = round(0.05 + (index % 5) * 0.17, 4)
        leie_flag = 1 if provider_index == 5 and index >= 18 else 0
        peer_zscore = round((provider_profile_score - 45) / 18, 4)
        label = int(
            amount_ratio >= 0.78
            or provider_profile_score >= 70
            or high_cost_item_ratio >= 0.55
            or leie_flag == 1
        )
        rows.append(
            {
                "claim_id": f"PUB-CLM-{index + 1:04d}",
                "member_id": f"PUB-MBR-{member_index + 1:04d}",
                "policy_id": f"PUB-POL-{member_index % 4 + 1:04d}",
                "provider_id": f"PUB-PRV-{provider_index + 1:04d}",
                "service_date": service_date.isoformat(),
                "service_date_ord": service_date.toordinal(),
                "claim_amount_to_limit_ratio": amount_ratio,
                "provider_profile_score": float(provider_profile_score),
                "high_cost_item_ratio": high_cost_item_ratio,
                "provider_peer_payment_zscore": peer_zscore,
                "leie_excluded_provider": leie_flag,
                "confirmed_fwa": label,
            }
        )
    return pd.DataFrame(rows)


def public_claim_frame(
    pandas: Any,
    claims_csv: Path,
    provider_summary_csv: Path | None,
    leie_csv: Path | None,
) -> Any:
    claims = pandas.read_csv(claims_csv, low_memory=False)
    provider_baseline = (
        load_provider_baseline(pandas, provider_summary_csv)
        if provider_summary_csv
        else {}
    )
    leie_npis = load_leie_npis(pandas, leie_csv) if leie_csv else set()
    rows = []
    for index, row in claims.iterrows():
        claim_id = value_from(row, ["clm_id", "claim_id", "claimid"], f"PUBLIC-{index + 1}")
        member_id = value_from(row, ["desynpuf_id", "member_id", "bene_id"], "unknown_member")
        provider_id = value_from(
            row,
            ["prvdr_num", "provider_id", "rndrng_npi", "npi", "at_npi", "op_npi"],
            "unknown_provider",
        )
        service_date = date_value(
            value_from(row, ["clm_from_dt", "service_date", "from_dt"], "2026-01-01")
        )
        claim_amount = numeric_value(
            value_from(row, ["clm_pmt_amt", "payment", "submitted_charge", "charge"], 0)
        )
        provider_score = provider_baseline.get(str(provider_id), 50.0)
        amount_ratio = min(claim_amount / max(provider_score * 100.0, 1.0), 2.0)
        leie_flag = 1 if str(provider_id) in leie_npis else 0
        high_cost_item_ratio = min(claim_amount / 10_000.0, 2.0)
        peer_zscore = (provider_score - 50.0) / 20.0
        weak_label = int(
            amount_ratio >= 0.75
            or provider_score >= 75
            or high_cost_item_ratio >= 0.7
            or leie_flag == 1
        )
        rows.append(
            {
                "claim_id": str(claim_id),
                "member_id": str(member_id),
                "policy_id": f"PUBLIC-POL-{index % 17:04d}",
                "provider_id": str(provider_id),
                "service_date": service_date.isoformat(),
                "service_date_ord": service_date.toordinal(),
                "claim_amount_to_limit_ratio": round(float(amount_ratio), 6),
                "provider_profile_score": round(float(provider_score), 6),
                "high_cost_item_ratio": round(float(high_cost_item_ratio), 6),
                "provider_peer_payment_zscore": round(float(peer_zscore), 6),
                "leie_excluded_provider": leie_flag,
                "confirmed_fwa": weak_label,
            }
        )
    if not rows:
        raise SystemExit(f"claims CSV has no rows: {claims_csv}")
    return pandas.DataFrame(rows)


def load_provider_baseline(pandas: Any, path: Path | None) -> dict[str, float]:
    if not path:
        return {}
    frame = pandas.read_csv(path, low_memory=False)
    baseline: dict[str, float] = {}
    for _, row in frame.iterrows():
        provider_id = value_from(row, ["rndrng_npi", "npi", "provider_id"], "")
        if not provider_id:
            continue
        services = numeric_value(value_from(row, ["tot_srvcs", "total_services"], 0))
        payment = numeric_value(
            value_from(
                row,
                ["avg_mdcr_pymt_amt", "average_medicare_payment_amount", "payment"],
                0,
            )
        )
        score = min(100.0, max(0.0, services / 100.0 + payment / 10.0))
        baseline[str(provider_id)] = score
    return baseline


def load_leie_npis(pandas: Any, path: Path | None) -> set[str]:
    if not path:
        return set()
    frame = pandas.read_csv(path, low_memory=False)
    npis = set()
    for _, row in frame.iterrows():
        npi = value_from(row, ["npi", "provider_id"], "")
        if npi:
            npis.add(str(npi))
    return npis


def write_dataset(pd: Any, claims: Any, output_dir: Path, dataset_version: str) -> None:
    claims = claims.sort_values(["service_date_ord", "claim_id"]).reset_index(drop=True)
    split_frames = split_claims(pd, claims)
    for split_name, frame in split_frames.items():
        split_dir = output_dir / f"split={split_name}"
        split_dir.mkdir(parents=True, exist_ok=True)
        frame.to_parquet(split_dir / "part-00000.parquet", index=False)
    manifest = {
        "dataset_key": "public_data_mvp_claims",
        "dataset_version": dataset_version,
        "business_domain": "health_fwa",
        "sample_grain": "claim",
        "label_column": "confirmed_fwa",
        "label_policy": PUBLIC_DATA_LABEL_POLICY,
        "entity_keys": ["claim_id", "member_id", "policy_id", "provider_id"],
        "time_split_field": "service_date_ord",
        "group_split_fields": ["member_id", "policy_id", "provider_id"],
        "public_data_boundary": {
            "purpose": "schema_pipeline_demo_anomaly_validation",
            "not_valid_for": [
                "customer_production_model_evidence",
                "fraud_accusation",
                "automatic_claim_denial",
            ],
        },
        "splits": [
            {"split_name": "train", "data_uri": "split=train/"},
            {"split_name": "validation", "data_uri": "split=validation/"},
            {"split_name": "out_of_time", "data_uri": "split=out_of_time/"},
        ],
    }
    (output_dir / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True),
        encoding="utf-8",
    )


def split_claims(pd: Any, claims: Any) -> dict[str, Any]:
    if len(claims) < 6:
        raise SystemExit("public-data MVP needs at least 6 claim rows")
    train_end = max(2, int(len(claims) * 0.6))
    validation_end = max(train_end + 1, int(len(claims) * 0.8))
    return {
        "train": claims.iloc[:train_end].copy(),
        "validation": claims.iloc[train_end:validation_end].copy(),
        "out_of_time": claims.iloc[validation_end:].copy(),
    }


def write_sources(output_dir: Path) -> None:
    (output_dir / "sources.json").write_text(
        json.dumps(
            {
                "source_bundle": "public_data_mvp",
                "label_policy": PUBLIC_DATA_LABEL_POLICY,
                "sources": PUBLIC_SOURCES,
            },
            indent=2,
            sort_keys=True,
        ),
        encoding="utf-8",
    )


def write_policy_corpus(policy_dir: Path, output_dir: Path) -> None:
    output_path = output_dir / "policy_corpus.jsonl"
    with output_path.open("w", encoding="utf-8") as handle:
        for path in sorted(policy_dir.glob("*")):
            if not path.is_file() or path.suffix.lower() not in {".txt", ".md"}:
                continue
            handle.write(
                json.dumps(
                    {
                        "document_id": path.stem,
                        "source": str(path),
                        "text": path.read_text(encoding="utf-8"),
                    },
                    sort_keys=True,
                )
                + "\n"
            )


def value_from(row: Any, candidates: list[str], default: Any) -> Any:
    normalized = {normalize_key(key): key for key in row.index}
    for candidate in candidates:
        key = normalized.get(normalize_key(candidate))
        if key is None:
            continue
        value = row[key]
        if value == value and str(value).strip():
            return value
    return default


def normalize_key(value: str) -> str:
    return re.sub(r"[^a-z0-9]+", "", value.lower())


def numeric_value(value: Any) -> float:
    try:
        return float(str(value).replace(",", "").strip())
    except (TypeError, ValueError):
        return 0.0


def date_value(value: Any) -> date:
    raw = str(value).strip()
    for fmt in ("%Y-%m-%d", "%Y%m%d", "%m/%d/%Y"):
        try:
            from datetime import datetime

            return datetime.strptime(raw, fmt).date()
        except ValueError:
            continue
    return date(2026, 1, 1)


if __name__ == "__main__":
    main()
