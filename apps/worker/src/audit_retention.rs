use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

use crate::{read_json_report, write_json};

const DEFAULT_RETENTION_YEARS: u16 = 6;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRetentionScanInput {
    pub retention_policy_id: String,
    #[serde(default)]
    pub records: Vec<AuditRetentionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRetentionRecord {
    pub table_name: String,
    pub record_id: String,
    pub created_at: String,
    #[serde(default)]
    pub legal_hold: bool,
    pub retention_policy_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditRetentionCandidate {
    pub table_name: String,
    pub record_id: String,
    pub created_at: String,
    pub retention_policy_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditRetentionReviewTask {
    pub task_kind: String,
    pub priority: String,
    pub reason: String,
    pub table_name: Option<String>,
    pub record_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRetentionScanReport {
    pub report_kind: String,
    pub report_version: u8,
    pub as_of_date: String,
    pub source_uri: String,
    pub retention_policy_id: String,
    pub retention_years: u16,
    pub cutoff_date: String,
    pub scanned_record_count: usize,
    pub archive_candidate_count: usize,
    pub destruction_review_candidate_count: usize,
    pub legal_hold_block_count: usize,
    pub invalid_record_count: usize,
    pub scan_status: String,
    pub archive_candidates: Vec<AuditRetentionCandidate>,
    pub destruction_review_candidates: Vec<AuditRetentionCandidate>,
    pub legal_hold_blocks: Vec<AuditRetentionCandidate>,
    pub review_tasks: Vec<AuditRetentionReviewTask>,
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
}

pub fn build_audit_retention_scan_report(
    source_uri: &str,
    output_dir: impl AsRef<Path>,
    as_of_date: &str,
    retention_years: Option<u16>,
) -> anyhow::Result<AuditRetentionScanReport> {
    let as_of = IsoDate::parse(as_of_date).context("parse as_of_date")?;
    let retention_years = retention_years.unwrap_or(DEFAULT_RETENTION_YEARS);
    if retention_years == 0 {
        bail!("retention_years must be greater than zero");
    }
    let cutoff = as_of.minus_years(retention_years)?;
    let input: AuditRetentionScanInput = serde_json::from_value(read_json_report(source_uri)?)
        .context("parse audit retention scan input")?;
    if input.retention_policy_id.trim().is_empty() {
        bail!("audit retention scan input requires retention_policy_id");
    }

    let mut archive_candidates = Vec::new();
    let mut destruction_review_candidates = Vec::new();
    let mut legal_hold_blocks = Vec::new();
    let mut review_tasks = Vec::new();

    for record in &input.records {
        if let Some(reason) = retention_record_validation_error(record) {
            review_tasks.push(AuditRetentionReviewTask {
                task_kind: "audit_retention_source_record_review".into(),
                priority: "high".into(),
                reason,
                table_name: trimmed_optional(record.table_name.as_str()),
                record_id: trimmed_optional(record.record_id.as_str()),
            });
            continue;
        }

        let created_at = match IsoDate::parse(record.created_at.as_str()) {
            Ok(value) => value,
            Err(error) => {
                review_tasks.push(AuditRetentionReviewTask {
                    task_kind: "audit_retention_source_record_review".into(),
                    priority: "high".into(),
                    reason: format!("invalid created_at: {error}"),
                    table_name: trimmed_optional(record.table_name.as_str()),
                    record_id: trimmed_optional(record.record_id.as_str()),
                });
                continue;
            }
        };

        if created_at <= cutoff {
            let candidate = AuditRetentionCandidate {
                table_name: record.table_name.trim().into(),
                record_id: record.record_id.trim().into(),
                created_at: record.created_at.trim().into(),
                retention_policy_id: record
                    .retention_policy_id
                    .as_deref()
                    .and_then(trimmed_optional)
                    .unwrap_or_else(|| input.retention_policy_id.trim().into()),
                reason: format!("created_at is on or before retention cutoff {cutoff}"),
            };
            archive_candidates.push(candidate.clone());
            if record.legal_hold {
                legal_hold_blocks.push(candidate);
            } else {
                destruction_review_candidates.push(candidate);
            }
        }
    }

    let scan_status = if review_tasks.is_empty() {
        "completed"
    } else {
        "completed_with_source_record_warnings"
    };
    let report = AuditRetentionScanReport {
        report_kind: "audit_retention_scan_report".into(),
        report_version: 1,
        as_of_date: as_of.to_string(),
        source_uri: source_uri.into(),
        retention_policy_id: input.retention_policy_id.trim().into(),
        retention_years,
        cutoff_date: cutoff.to_string(),
        scanned_record_count: input.records.len(),
        archive_candidate_count: archive_candidates.len(),
        destruction_review_candidate_count: destruction_review_candidates.len(),
        legal_hold_block_count: legal_hold_blocks.len(),
        invalid_record_count: review_tasks.len(),
        scan_status: scan_status.into(),
        archive_candidates,
        destruction_review_candidates,
        legal_hold_blocks,
        review_tasks,
        evidence_refs: vec![format!("audit_retention_source:{source_uri}")],
        governance_boundary: "retention scan is dry-run evidence only; it must not delete audit records, bypass legal hold, or shorten customer-approved retention policy".into(),
    };

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create audit retention output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir.as_ref().join("audit_retention_scan_report.json"),
        &report,
    )?;
    write_json(
        output_dir
            .as_ref()
            .join("audit_retention_archive_candidates.json"),
        &report.archive_candidates,
    )?;
    write_json(
        output_dir
            .as_ref()
            .join("audit_retention_destruction_review_candidates.json"),
        &report.destruction_review_candidates,
    )?;
    write_json(
        output_dir
            .as_ref()
            .join("audit_retention_legal_hold_blocks.json"),
        &report.legal_hold_blocks,
    )?;
    Ok(report)
}

fn retention_record_validation_error(record: &AuditRetentionRecord) -> Option<String> {
    if record.table_name.trim().is_empty() {
        return Some("audit retention record missing table_name".into());
    }
    if record.record_id.trim().is_empty() {
        return Some("audit retention record missing record_id".into());
    }
    if record.created_at.trim().is_empty() {
        return Some("audit retention record missing created_at".into());
    }
    None
}

fn trimmed_optional(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.into())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct IsoDate {
    year: i32,
    month: u8,
    day: u8,
}

impl IsoDate {
    fn parse(value: &str) -> anyhow::Result<Self> {
        let date = value
            .trim()
            .get(..10)
            .ok_or_else(|| anyhow::anyhow!("expected YYYY-MM-DD"))?;
        let mut parts = date.split('-');
        let year = parts
            .next()
            .context("missing year")?
            .parse::<i32>()
            .context("invalid year")?;
        let month = parts
            .next()
            .context("missing month")?
            .parse::<u8>()
            .context("invalid month")?;
        let day = parts
            .next()
            .context("missing day")?
            .parse::<u8>()
            .context("invalid day")?;
        if parts.next().is_some() || month == 0 || month > 12 {
            bail!("expected YYYY-MM-DD");
        }
        let max_day = days_in_month(year, month);
        if day == 0 || day > max_day {
            bail!("invalid day for month");
        }
        Ok(Self { year, month, day })
    }

    fn minus_years(self, years: u16) -> anyhow::Result<Self> {
        let year = self
            .year
            .checked_sub(i32::from(years))
            .context("retention cutoff year underflow")?;
        let day = self.day.min(days_in_month(year, self.month));
        Ok(Self {
            year,
            month: self.month,
            day,
        })
    }
}

impl std::fmt::Display for IsoDate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{:04}-{:02}-{:02}",
            self.year, self.month, self.day
        )
    }
}

fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}
