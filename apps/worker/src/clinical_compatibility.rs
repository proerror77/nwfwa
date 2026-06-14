use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, path::Path};

use crate::{read_json_report, write_json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClinicalCompatibilityReferenceInput {
    pub reference_version: String,
    pub effective_date: String,
    pub source_authority: String,
    #[serde(default)]
    pub records: Vec<ClinicalCompatibilityReferenceRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClinicalCompatibilityReferenceRow {
    pub diagnosis_code_prefix: String,
    pub procedure_code: String,
    pub compatibility_score: f64,
    pub policy_authority_ref: String,
    pub rationale: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClinicalCompatibilityRecord {
    pub compatibility_key: String,
    pub diagnosis_code_prefix: String,
    pub procedure_code: String,
    pub diagnosis_procedure_match_score: f64,
    pub data_source: String,
    pub policy_authority_ref: String,
    pub rationale: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClinicalCompatibilityReviewTask {
    pub task_type: String,
    pub compatibility_key: String,
    pub reason: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClinicalCompatibilityReferenceReport {
    pub report_kind: String,
    pub report_version: u8,
    pub reference_version: String,
    pub effective_date: String,
    pub source_authority: String,
    pub source_uri: String,
    pub record_count: usize,
    pub records: Vec<ClinicalCompatibilityRecord>,
    pub review_tasks: Vec<ClinicalCompatibilityReviewTask>,
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
}

pub fn build_clinical_compatibility_reference_report(
    reference_uri: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<ClinicalCompatibilityReferenceReport> {
    let input: ClinicalCompatibilityReferenceInput =
        serde_json::from_value(read_json_report(reference_uri)?)
            .context("parse clinical compatibility reference input")?;
    validate_header(&input)?;

    let mut seen_keys = BTreeSet::new();
    let mut records = Vec::with_capacity(input.records.len());
    let mut review_tasks = Vec::new();
    for row in input.records {
        let record = normalize_record(&input.reference_version, row)?;
        if !seen_keys.insert(record.compatibility_key.clone()) {
            bail!(
                "duplicate clinical compatibility record {}",
                record.compatibility_key
            );
        }
        if record.diagnosis_procedure_match_score <= 0.4 {
            review_tasks.push(ClinicalCompatibilityReviewTask {
                task_type: "clinical_policy_review_candidate".into(),
                compatibility_key: record.compatibility_key.clone(),
                reason: "low compatibility score should be reviewed before production activation"
                    .into(),
                evidence_refs: record.evidence_refs.clone(),
            });
        }
        records.push(record);
    }

    let evidence_refs = vec![
        format!("clinical_compatibility_reference:{reference_uri}"),
        format!(
            "clinical_policy_authority:{}",
            input.source_authority.trim()
        ),
    ];
    let report = ClinicalCompatibilityReferenceReport {
        report_kind: "clinical_compatibility_reference".into(),
        report_version: 1,
        reference_version: input.reference_version,
        effective_date: input.effective_date,
        source_authority: input.source_authority,
        source_uri: reference_uri.into(),
        record_count: records.len(),
        records,
        review_tasks,
        evidence_refs,
        governance_boundary: "clinical compatibility reference data can feed ClinicalCompatibilityFeatureContext; it must not deny claims or replace medical review without customer-approved policy authority".into(),
    };

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create clinical compatibility output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir
            .as_ref()
            .join("clinical_compatibility_reference_report.json"),
        &report,
    )?;
    write_json(
        output_dir
            .as_ref()
            .join("clinical_compatibility_records.json"),
        &report.records,
    )?;
    Ok(report)
}

fn validate_header(input: &ClinicalCompatibilityReferenceInput) -> anyhow::Result<()> {
    if input.reference_version.trim().is_empty() {
        bail!("clinical compatibility reference requires reference_version");
    }
    if input.effective_date.trim().is_empty() {
        bail!("clinical compatibility reference requires effective_date");
    }
    if input.source_authority.trim().is_empty() {
        bail!("clinical compatibility reference requires source_authority");
    }
    Ok(())
}

fn normalize_record(
    reference_version: &str,
    row: ClinicalCompatibilityReferenceRow,
) -> anyhow::Result<ClinicalCompatibilityRecord> {
    let diagnosis_code_prefix = row.diagnosis_code_prefix.trim().to_ascii_uppercase();
    let procedure_code = row.procedure_code.trim().to_ascii_uppercase();
    if diagnosis_code_prefix.is_empty() {
        bail!("clinical compatibility record missing diagnosis_code_prefix");
    }
    if procedure_code.is_empty() {
        bail!("clinical compatibility record missing procedure_code");
    }
    if !row.compatibility_score.is_finite()
        || row.compatibility_score < 0.0
        || row.compatibility_score > 1.0
    {
        bail!(
            "clinical compatibility record {}|{} has invalid compatibility_score",
            diagnosis_code_prefix,
            procedure_code
        );
    }
    if row.policy_authority_ref.trim().is_empty() {
        bail!(
            "clinical compatibility record {}|{} missing policy_authority_ref",
            diagnosis_code_prefix,
            procedure_code
        );
    }
    let evidence_refs = row
        .evidence_refs
        .into_iter()
        .map(|reference| reference.trim().to_string())
        .filter(|reference| !reference.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if evidence_refs.is_empty() {
        bail!(
            "clinical compatibility record {}|{} requires evidence_refs",
            diagnosis_code_prefix,
            procedure_code
        );
    }

    let compatibility_key = format!("{diagnosis_code_prefix}|{procedure_code}");
    Ok(ClinicalCompatibilityRecord {
        compatibility_key,
        diagnosis_code_prefix,
        procedure_code,
        diagnosis_procedure_match_score: row.compatibility_score,
        data_source: format!(
            "worker.icd_cpt_compatibility_reference:{}",
            reference_version.trim()
        ),
        policy_authority_ref: row.policy_authority_ref.trim().into(),
        rationale: row.rationale.trim().into(),
        evidence_refs,
    })
}
