use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

use crate::{api_url, read_json_report, required_non_empty, write_json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanctionsSourceSnapshot {
    pub source_date: Option<String>,
    #[serde(default)]
    pub records: Vec<SanctionsSourceRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanctionsSourceRecord {
    pub list: String,
    pub provider_id: Option<String>,
    pub npi: Option<String>,
    pub provider_name: String,
    pub sanction_type: Option<String>,
    pub effective_date: Option<String>,
    pub source_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanctionsProviderUpsert {
    pub sanction_key: String,
    pub list: String,
    pub provider_id: Option<String>,
    pub npi: Option<String>,
    pub provider_name: String,
    pub sanction_type: Option<String>,
    pub effective_date: Option<String>,
    pub source_ref: Option<String>,
    pub risk_feature: String,
    pub risk_score: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanctionsSyncReviewTask {
    pub task_kind: String,
    pub priority: String,
    pub reason: String,
    pub source_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanctionsSyncReport {
    pub report_kind: String,
    pub report_version: u8,
    pub run_date: String,
    pub source_uri: String,
    pub source_date: Option<String>,
    pub dry_run: bool,
    pub execution_mode: String,
    pub sync_status: String,
    pub source_record_count: usize,
    pub valid_record_count: usize,
    pub invalid_record_count: usize,
    pub provider_upserts: Vec<SanctionsProviderUpsert>,
    pub review_tasks: Vec<SanctionsSyncReviewTask>,
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanctionsSyncReportSubmission {
    pub actor: String,
    pub notes: String,
    pub source_report_uri: String,
    pub report_kind: String,
    pub run_date: String,
    pub source_uri: String,
    pub source_date: Option<String>,
    pub sync_status: String,
    pub source_record_count: usize,
    pub valid_record_count: usize,
    pub invalid_record_count: usize,
    pub provider_upserts: Vec<SanctionsProviderUpsert>,
    pub review_tasks: Vec<serde_json::Value>,
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
}

pub async fn fetch_oig_sam_sanctions_snapshot(
    oig_url: Option<&str>,
    sam_url: Option<&str>,
    output_dir: impl AsRef<Path>,
    source_date: Option<&str>,
) -> anyhow::Result<SanctionsSourceSnapshot> {
    if oig_url
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
        && sam_url
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
    {
        bail!("at least one of oig_url or sam_url is required");
    }

    let client = reqwest::Client::new();
    let mut records = Vec::new();
    if let Some(url) = oig_url.map(str::trim).filter(|value| !value.is_empty()) {
        records.extend(fetch_sanctions_records(&client, url, "OIG").await?);
    }
    if let Some(url) = sam_url.map(str::trim).filter(|value| !value.is_empty()) {
        records.extend(fetch_sanctions_records(&client, url, "SAM").await?);
    }

    let snapshot = SanctionsSourceSnapshot {
        source_date: source_date
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        records,
    };
    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create sanctions snapshot output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir.as_ref().join("oig_sam_sanctions_snapshot.json"),
        &snapshot,
    )?;
    Ok(snapshot)
}

pub fn build_sanctions_sync_report(
    source_uri: &str,
    output_dir: impl AsRef<Path>,
    run_date: &str,
    dry_run: bool,
) -> anyhow::Result<SanctionsSyncReport> {
    if !dry_run {
        bail!(
            "direct sanctions sync currently supports --dry-run only; use submit-sanctions-sync-report for approved API writes"
        );
    }
    if run_date.trim().is_empty() {
        bail!("run_date is required");
    }

    let snapshot: SanctionsSourceSnapshot = serde_json::from_value(read_json_report(source_uri)?)
        .context("parse sanctions snapshot")?;
    let mut provider_upserts = Vec::new();
    let mut review_tasks = Vec::new();

    for record in &snapshot.records {
        if let Some(reason) = sanctions_record_validation_error(record) {
            review_tasks.push(SanctionsSyncReviewTask {
                task_kind: "sanctions_source_record_review".into(),
                priority: "high".into(),
                reason,
                source_ref: record.source_ref.clone(),
            });
            continue;
        }
        provider_upserts.push(SanctionsProviderUpsert {
            sanction_key: sanctions_key(record),
            list: record.list.trim().to_ascii_uppercase(),
            provider_id: trimmed_optional(record.provider_id.as_deref()),
            npi: trimmed_optional(record.npi.as_deref()),
            provider_name: record.provider_name.trim().to_string(),
            sanction_type: trimmed_optional(record.sanction_type.as_deref()),
            effective_date: trimmed_optional(record.effective_date.as_deref()),
            source_ref: trimmed_optional(record.source_ref.as_deref()),
            risk_feature: "provider_sanctions_excluded".into(),
            risk_score: 100,
        });
    }

    let report = SanctionsSyncReport {
        report_kind: "oig_sam_sanctions_sync_report".into(),
        report_version: 1,
        run_date: run_date.trim().into(),
        source_uri: source_uri.into(),
        source_date: snapshot.source_date,
        dry_run,
        execution_mode: "dry_run_contract_only".into(),
        sync_status: if review_tasks.is_empty() {
            "ready_to_apply".into()
        } else {
            "completed_with_source_record_warnings".into()
        },
        source_record_count: snapshot.records.len(),
        valid_record_count: provider_upserts.len(),
        invalid_record_count: review_tasks.len(),
        provider_upserts,
        review_tasks,
        evidence_refs: vec![format!("sanctions_source_snapshot:{source_uri}")],
        governance_boundary: "dry-run produces sanctions upsert evidence only; it must not write provider sanctions, assign fraud labels, or alter scoring policy".into(),
    };

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create sanctions sync output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir.as_ref().join("sanctions_sync_report.json"),
        &report,
    )?;
    write_json(
        output_dir.as_ref().join("sanctions_provider_upserts.json"),
        &report.provider_upserts,
    )?;
    Ok(report)
}

pub fn build_sanctions_sync_report_submission(
    report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<SanctionsSyncReportSubmission> {
    let report_uri = required_non_empty("report_uri", report_uri)?;
    let actor = required_non_empty("actor", actor)?;
    let notes = required_non_empty("notes", notes)?;
    let report: SanctionsSyncReport = serde_json::from_value(read_json_report(report_uri)?)
        .context("parse sanctions sync report")?;
    if report.report_kind != "oig_sam_sanctions_sync_report" {
        bail!("report_kind must be oig_sam_sanctions_sync_report");
    }
    if report.provider_upserts.is_empty() {
        bail!("sanctions sync report requires provider_upserts before API submission");
    }
    let required_ref = format!("sanctions_source_snapshot:{}", report.source_uri);
    if !report
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == required_ref)
    {
        bail!("sanctions sync report requires {required_ref} evidence");
    }
    let mut evidence_refs = report.evidence_refs;
    evidence_refs.push(format!("sanctions_sync_reports:{report_uri}"));
    evidence_refs.sort();
    evidence_refs.dedup();
    Ok(SanctionsSyncReportSubmission {
        actor: actor.into(),
        notes: notes.into(),
        source_report_uri: report_uri.into(),
        report_kind: report.report_kind,
        run_date: report.run_date,
        source_uri: report.source_uri,
        source_date: report.source_date,
        sync_status: report.sync_status,
        source_record_count: report.source_record_count,
        valid_record_count: report.valid_record_count,
        invalid_record_count: report.invalid_record_count,
        provider_upserts: report.provider_upserts,
        review_tasks: report
            .review_tasks
            .into_iter()
            .map(serde_json::to_value)
            .collect::<Result<Vec<_>, _>>()?,
        evidence_refs,
        governance_boundary: report.governance_boundary,
    })
}

pub async fn submit_sanctions_sync_report(
    api_base_url: &str,
    api_key: &str,
    report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<serde_json::Value> {
    let payload = build_sanctions_sync_report_submission(report_uri, actor, notes)?;
    let response = reqwest::Client::new()
        .post(api_url(
            api_base_url,
            "/api/v1/ops/providers/sanctions-sync-reports",
        ))
        .header("x-api-key", api_key)
        .json(&payload)
        .send()
        .await
        .context("submit sanctions sync report")?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("submit sanctions sync report failed with {status}: {body}");
    }
    response
        .json::<serde_json::Value>()
        .await
        .context("parse sanctions sync report response")
}

async fn fetch_sanctions_records(
    client: &reqwest::Client,
    url: &str,
    default_list: &str,
) -> anyhow::Result<Vec<SanctionsSourceRecord>> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("fetch sanctions snapshot {url}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .with_context(|| format!("read sanctions snapshot {url}"))?;
    if !status.is_success() {
        bail!("fetch sanctions snapshot {url} failed with {status}: {body}");
    }
    let value: serde_json::Value =
        serde_json::from_str(&body).with_context(|| format!("parse sanctions snapshot {url}"))?;
    let mut records = sanctions_records_from_value(value, default_list)
        .with_context(|| format!("parse sanctions records from {url}"))?;
    for record in &mut records {
        if record.list.trim().is_empty() {
            record.list = default_list.into();
        }
    }
    Ok(records)
}

fn sanctions_records_from_value(
    value: serde_json::Value,
    default_list: &str,
) -> anyhow::Result<Vec<SanctionsSourceRecord>> {
    if value.is_array() {
        let mut records: Vec<SanctionsSourceRecord> = serde_json::from_value(value)?;
        for record in &mut records {
            if record.list.trim().is_empty() {
                record.list = default_list.into();
            }
        }
        return Ok(records);
    }
    let snapshot: SanctionsSourceSnapshot = serde_json::from_value(value)?;
    Ok(snapshot
        .records
        .into_iter()
        .map(|mut record| {
            if record.list.trim().is_empty() {
                record.list = default_list.into();
            }
            record
        })
        .collect())
}

fn sanctions_record_validation_error(record: &SanctionsSourceRecord) -> Option<String> {
    if record.list.trim().is_empty() {
        return Some("sanctions record missing list".into());
    }
    if record.provider_name.trim().is_empty() {
        return Some("sanctions record missing provider_name".into());
    }
    if trimmed_optional(record.provider_id.as_deref()).is_none()
        && trimmed_optional(record.npi.as_deref()).is_none()
    {
        return Some("sanctions record requires provider_id or npi".into());
    }
    None
}

fn sanctions_key(record: &SanctionsSourceRecord) -> String {
    let identifier = trimmed_optional(record.provider_id.as_deref())
        .or_else(|| trimmed_optional(record.npi.as_deref()))
        .unwrap_or_else(|| "unknown".into());
    format!("{}:{identifier}", record.list.trim().to_ascii_uppercase())
}

fn trimmed_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}
