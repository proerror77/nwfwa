use fwa_core::{ClaimContext, ProviderRiskTier};
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClinicalDocumentEvidence {
    pub document_id: String,
    pub document_type: String,
    pub linked_item_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClinicalEvidenceFinding {
    pub item_code: String,
    pub issue_type: String,
    pub required_evidence: Vec<String>,
    pub missing_evidence: Vec<String>,
    pub reason: String,
    pub review_route: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClinicalEvidenceAssessment {
    pub review_required: bool,
    pub review_route: String,
    pub evidence_status: String,
    pub minimum_evidence: Vec<String>,
    pub missing_evidence: Vec<String>,
    pub item_findings: Vec<ClinicalEvidenceFinding>,
    pub evidence_refs: Vec<String>,
}

pub fn assess_clinical_evidence(
    context: &ClaimContext,
    documents: &[ClinicalDocumentEvidence],
) -> ClinicalEvidenceAssessment {
    let mut item_findings = Vec::new();

    for item in &context.items {
        let required = required_evidence_for_item(context, item);
        if required.is_empty() {
            continue;
        }

        let linked_documents = documents
            .iter()
            .filter(|document| {
                document.linked_item_codes.is_empty()
                    || document
                        .linked_item_codes
                        .iter()
                        .any(|code| code.eq_ignore_ascii_case(&item.item_code))
            })
            .collect::<Vec<_>>();
        let supplied_types = linked_documents
            .iter()
            .map(|document| normalize_document_type(&document.document_type))
            .collect::<BTreeSet<_>>();
        let missing_evidence = required
            .iter()
            .filter(|evidence| !supplied_types.contains(*evidence))
            .cloned()
            .collect::<Vec<_>>();
        let evidence_refs = std::iter::once(format!("claim_items:{}", item.item_code))
            .chain(
                linked_documents
                    .iter()
                    .map(|document| format!("documents:{}", document.document_id)),
            )
            .collect::<Vec<_>>();

        item_findings.push(ClinicalEvidenceFinding {
            item_code: item.item_code.clone(),
            issue_type: issue_type_for_item(item),
            required_evidence: required,
            missing_evidence: missing_evidence.clone(),
            reason: reason_for_item(item),
            review_route: review_route_for_item(context, item, &missing_evidence),
            evidence_refs,
        });
    }

    let minimum_evidence = collect_unique(item_findings.iter().flat_map(|finding| {
        finding
            .required_evidence
            .iter()
            .map(std::string::String::as_str)
    }));
    let missing_evidence = collect_unique(item_findings.iter().flat_map(|finding| {
        finding
            .missing_evidence
            .iter()
            .map(std::string::String::as_str)
    }));
    let evidence_refs = collect_unique(item_findings.iter().flat_map(|finding| {
        finding
            .evidence_refs
            .iter()
            .map(std::string::String::as_str)
    }));
    let review_required = !missing_evidence.is_empty();
    let evidence_status = if item_findings.is_empty() {
        "no_clinical_evidence_required"
    } else if review_required {
        "missing_required_evidence"
    } else {
        "sufficient_for_basic_review"
    };

    // Graduated assessment-level route: escalate to fraud investigation when
    // provider risk is High and medical necessity gaps exist; fall back to
    // documentation review when only non-clinical records are missing.
    let assessment_route = if review_required {
        review_route_for_assessment(context, &item_findings)
    } else {
        "none".into()
    };

    ClinicalEvidenceAssessment {
        review_required,
        review_route: assessment_route,
        evidence_status: evidence_status.into(),
        minimum_evidence,
        missing_evidence,
        item_findings,
        evidence_refs,
    }
}

fn required_evidence_for_item(context: &ClaimContext, item: &fwa_core::ClaimItem) -> Vec<String> {
    let item_type = item.item_type.to_ascii_lowercase();
    let description = item.description.to_ascii_lowercase();
    let item_code = item.item_code.to_ascii_uppercase();
    if item_type == "dental"
        || description.contains("dental")
        || description.contains("implant")
        || description.contains("tooth")
        || item_code.starts_with("DEN")
    {
        return vec!["dental_xray".into(), "medical_record".into()];
    }
    if item_type == "surgery"
        || description.contains("surgery")
        || description.contains("surgical")
        || description.contains("operation")
        || item_code.starts_with("SURG")
        || item_code.starts_with("OP")
    {
        return vec![
            "operation_record".into(),
            "medical_record".into(),
            "invoice".into(),
        ];
    }
    if item_type == "procedure"
        && (description.contains("imaging")
            || description.contains("radiology")
            || description.contains("ct")
            || description.contains("mri")
            || item_code.starts_with("IMG"))
    {
        return vec![
            "clinical_order".into(),
            "medical_record".into(),
            "radiology_report".into(),
        ];
    }
    if item_type == "drug"
        || item_type == "medication"
        || item_type == "pharmacy"
        || description.contains("prescription")
    {
        return vec!["medication_order".into(), "prescription_detail".into()];
    }
    if item_type == "lab" || description.contains("laboratory") || description.contains("lab") {
        return vec!["lab_order".into(), "lab_result".into()];
    }
    if high_value_item_ratio(context, item) >= 0.5 {
        return vec!["invoice".into(), "medical_record".into()];
    }
    Vec::new()
}

fn issue_type_for_item(item: &fwa_core::ClaimItem) -> String {
    let item_type = item.item_type.to_ascii_lowercase();
    if item_type == "drug" || item_type == "medication" || item_type == "pharmacy" {
        "drug_reasonableness_review_required".into()
    } else if item_type == "lab" {
        "lab_evidence_review_required".into()
    } else {
        "medical_necessity_review_required".into()
    }
}

/// Determine the review route for a single item finding.
///
/// Routing tiers (in priority order):
/// 1. `fraud_investigation_review` — provider is HIGH risk and there are
///    medical necessity gaps; the combination of elevated provider risk and
///    missing clinical evidence is a strong indicator of potential fraud.
/// 2. `medical_review` — medical necessity evidence is missing (default for
///    most items with gaps).
/// 3. `documentation_review` — only non-clinical administrative records are
///    missing (invoice, prescription_detail) — no medical necessity concern.
fn review_route_for_item(
    context: &ClaimContext,
    item: &fwa_core::ClaimItem,
    missing: &[String],
) -> String {
    if missing.is_empty() {
        return "none".into();
    }
    let is_high_risk_provider = context.provider.risk_tier == ProviderRiskTier::High;
    let has_medical_necessity_gap = missing.iter().any(|evidence| {
        matches!(
            evidence.as_str(),
            "medical_record"
                | "operation_record"
                | "radiology_report"
                | "lab_result"
                | "dental_xray"
                | "clinical_order"
                | "lab_order"
        )
    });
    let documentation_only = missing.iter().all(|evidence| {
        matches!(
            evidence.as_str(),
            "invoice" | "prescription_detail" | "medication_order"
        )
    });
    // Suppress unused-variable lint for item — item type could add more routing
    // logic in future; keep it in the signature for forward-compatibility.
    let _ = item;
    if is_high_risk_provider && has_medical_necessity_gap {
        "fraud_investigation_review".into()
    } else if documentation_only {
        "documentation_review".into()
    } else {
        "medical_review".into()
    }
}

/// Determine the top-level assessment review route from all item findings.
///
/// If any item was routed to `fraud_investigation_review` the assessment
/// inherits that route.  Otherwise the most severe individual route wins.
fn review_route_for_assessment(
    context: &ClaimContext,
    item_findings: &[ClinicalEvidenceFinding],
) -> String {
    let has_fraud_route = item_findings
        .iter()
        .any(|f| !f.missing_evidence.is_empty() && f.review_route == "fraud_investigation_review");
    if has_fraud_route {
        return "fraud_investigation_review".into();
    }
    let has_medical_route = item_findings
        .iter()
        .any(|f| !f.missing_evidence.is_empty() && f.review_route == "medical_review");
    if has_medical_route {
        return "medical_review".into();
    }
    // Only documentation gaps remain.
    let _ = context; // reserved for future context-driven escalation
    "documentation_review".into()
}

fn reason_for_item(item: &fwa_core::ClaimItem) -> String {
    let item_type = item.item_type.to_ascii_lowercase();
    if item_type == "drug" || item_type == "medication" || item_type == "pharmacy" {
        "药品或处方项目需要医嘱和处方证据支持".into()
    } else if item_type == "dental" {
        "牙科项目需要 X 光片和病历证据支持".into()
    } else if item_type == "surgery" {
        "手术项目需要手术记录、病历和发票证据支持".into()
    } else if item_type == "lab" {
        "检验项目需要医嘱和检验结果证据支持".into()
    } else {
        "高价值诊疗项目需要医嘱、病历和报告支持".into()
    }
}

fn high_value_item_ratio(context: &ClaimContext, item: &fwa_core::ClaimItem) -> f64 {
    if context.claim.amount.amount.is_zero() {
        return 0.0;
    }
    (item.total_amount.amount / context.claim.amount.amount)
        .to_f64()
        .unwrap_or(0.0)
}

fn normalize_document_type(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace([' ', '-'], "_")
}

fn collect_unique<'a>(values: impl Iterator<Item = &'a str>) -> Vec<String> {
    values
        .map(std::string::ToString::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use fwa_core::*;
    use rust_decimal::Decimal;

    fn context() -> ClaimContext {
        let member_id = MemberId::from_external("MBR-1");
        let policy_id = PolicyId::from_external("POL-1");
        let provider_id = ProviderId::from_external("PRV-1");
        ClaimContext {
            claim: Claim {
                id: ClaimId::from_external("CLM-1"),
                external_claim_id: "CLM-1".into(),
                member_id: member_id.clone(),
                policy_id: policy_id.clone(),
                provider_id: provider_id.clone(),
                diagnosis_code: "J10".into(),
                diagnosis_codes: vec![],
                service_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 6).unwrap(),
                amount: Money::new(Decimal::new(12000, 0), "CNY"),
            },
            items: vec![ClaimItem {
                item_code: "IMG-900".into(),
                item_type: "procedure".into(),
                description: "High cost imaging".into(),
                quantity: 1,
                unit_amount: Money::new(Decimal::new(12000, 0), "CNY"),
                total_amount: Money::new(Decimal::new(12000, 0), "CNY"),
            }],
            member: Member {
                id: member_id.clone(),
                external_member_id: "MBR-1".into(),
                dob: None,
                gender: None,
            },
            policy: Policy {
                id: policy_id,
                external_policy_id: "POL-1".into(),
                member_id,
                product_code: "MED".into(),
                coverage_start_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                coverage_end_date: chrono::NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
                coverage_limit: Money::new(Decimal::new(15000, 0), "CNY"),
            },
            provider: Provider {
                id: provider_id,
                external_provider_id: "PRV-1".into(),
                name: "Demo Hospital".into(),
                provider_type: "hospital".into(),
                region: "SH".into(),
                risk_tier: ProviderRiskTier::High,
            },
        }
    }

    fn context_with_item(item_code: &str, item_type: &str, description: &str) -> ClaimContext {
        let mut context = context();
        context.items[0] = ClaimItem {
            item_code: item_code.into(),
            item_type: item_type.into(),
            description: description.into(),
            quantity: 1,
            unit_amount: Money::new(Decimal::new(12000, 0), "CNY"),
            total_amount: Money::new(Decimal::new(12000, 0), "CNY"),
        };
        context
    }

    #[test]
    fn flags_missing_medical_evidence_for_imaging() {
        let assessment = assess_clinical_evidence(&context(), &[]);

        assert!(assessment.review_required);
        // High-risk provider (ProviderRiskTier::High in test fixture) + missing
        // medical_record → escalated to fraud_investigation_review.
        assert_eq!(assessment.review_route, "fraud_investigation_review");
        assert!(assessment
            .missing_evidence
            .contains(&"medical_record".to_string()));
        assert_eq!(
            assessment.item_findings[0].issue_type,
            "medical_necessity_review_required"
        );
    }

    #[test]
    fn flags_missing_dental_xray_for_dental_items() {
        let assessment = assess_clinical_evidence(
            &context_with_item("DEN-100", "dental", "Dental implant"),
            &[],
        );

        assert!(assessment.review_required);
        assert!(assessment
            .missing_evidence
            .contains(&"dental_xray".to_string()));
        assert!(assessment
            .missing_evidence
            .contains(&"medical_record".to_string()));
    }

    #[test]
    fn flags_missing_prescription_detail_for_drug_items() {
        let assessment = assess_clinical_evidence(
            &context_with_item("DRUG-100", "drug", "Prescription medication"),
            &[],
        );

        assert!(assessment.review_required);
        assert!(assessment
            .missing_evidence
            .contains(&"medication_order".to_string()));
        assert!(assessment
            .missing_evidence
            .contains(&"prescription_detail".to_string()));
    }

    #[test]
    fn flags_missing_operation_record_for_surgery_items() {
        let assessment = assess_clinical_evidence(
            &context_with_item("SURG-100", "surgery", "Complex operation"),
            &[],
        );

        assert!(assessment.review_required);
        assert!(assessment
            .missing_evidence
            .contains(&"operation_record".to_string()));
        assert!(assessment
            .missing_evidence
            .contains(&"medical_record".to_string()));
    }
}
