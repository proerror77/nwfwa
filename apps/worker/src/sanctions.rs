use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

use crate::{read_json_report, write_json};

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

pub fn build_sanctions_sync_report(
    source_uri: &str,
    output_dir: impl AsRef<Path>,
    run_date: &str,
    dry_run: bool,
) -> anyhow::Result<SanctionsSyncReport> {
    if !dry_run {
        bail!("sanctions sync currently supports --dry-run only; repository writes are not implemented");
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
