use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FwaSchemeDefinition {
    pub scheme_family: String,
    pub display_name: String,
    pub risk_domain: String,
    pub description: String,
    pub minimum_evidence: Vec<String>,
    pub default_review_route: String,
    pub primary_layers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceSufficiency {
    pub scheme_family: String,
    pub status: String,
    pub minimum_evidence: Vec<String>,
    pub present_evidence: Vec<String>,
    pub missing_evidence: Vec<String>,
}

pub fn fwa_scheme_taxonomy() -> Vec<FwaSchemeDefinition> {
    vec![
        scheme(
            "duplicate_billing",
            "Duplicate billing",
            "Claim",
            "Repeated billing for the same member, provider, service date, procedure, and amount.",
            &[
                "same_member",
                "provider",
                "service_date",
                "procedure",
                "amount",
                "claim_lineage",
            ],
            "manual_review",
            &["L2_RULE_DETECTION", "L7_RISK_FUSION_ROUTING"],
        ),
        scheme(
            "upcoding",
            "Upcoding",
            "Medical coding",
            "Billed code appears higher complexity than diagnosis and records support.",
            &[
                "diagnosis",
                "billed_code",
                "lower_complexity_comparator",
                "medical_record",
                "coding_rationale",
            ],
            "medical_review",
            &["L5_MEDICAL_REASONABLENESS", "L7_RISK_FUSION_ROUTING"],
        ),
        scheme(
            "unbundling",
            "Unbundling",
            "Billing pattern",
            "Separate component codes appear billed where a bundled code should apply.",
            &[
                "component_codes",
                "bundled_code_comparator",
                "same_episode",
                "billing_timeline",
            ],
            "manual_review",
            &["L2_RULE_DETECTION", "L5_MEDICAL_REASONABLENESS"],
        ),
        scheme(
            "medically_unnecessary_service",
            "Medically unnecessary service",
            "Clinical",
            "Claimed service lacks enough diagnosis, order, note, or treatment-context support.",
            &[
                "diagnosis",
                "order",
                "chart_note",
                "treatment_context",
                "reviewer_finding",
                "policy_rule",
            ],
            "medical_review",
            &["L5_MEDICAL_REASONABLENESS", "L7_RISK_FUSION_ROUTING"],
        ),
        scheme(
            "excessive_utilization",
            "Excessive utilization",
            "Utilization",
            "Member, provider, or service frequency exceeds expected peer or policy patterns.",
            &[
                "member_history",
                "service_frequency",
                "peer_benchmark",
                "time_window",
                "clinical_rationale",
            ],
            "qa_review",
            &["L1_PEER_BENCHMARK", "L3_UNSUPERVISED_ANOMALY"],
        ),
        scheme(
            "diagnosis_procedure_mismatch",
            "Diagnosis-procedure mismatch",
            "Clinical",
            "Billed procedure is weakly supported by diagnosis, documentation, or policy criteria.",
            &[
                "diagnosis",
                "procedure",
                "medical_record",
                "clinical_rationale",
                "policy_rule",
            ],
            "medical_review",
            &["L2_RULE_DETECTION", "L5_MEDICAL_REASONABLENESS"],
        ),
        scheme(
            "laboratory_testing_abuse",
            "Laboratory testing abuse",
            "Laboratory",
            "Ordering volume, diagnosis match, or lab/provider pattern indicates testing abuse risk.",
            &[
                "ordering_pattern",
                "diagnosis_match",
                "frequency",
                "peer_benchmark",
                "ordering_provider",
            ],
            "provider_review",
            &["L1_PEER_BENCHMARK", "L6_PROVIDER_GRAPH_RISK"],
        ),
        scheme(
            "telehealth_abuse",
            "Telehealth abuse",
            "Telehealth",
            "Visit mode, location, frequency, or documentation pattern indicates telehealth abuse risk.",
            &[
                "visit_mode",
                "provider_member_location",
                "visit_frequency",
                "documentation",
                "policy_rule",
            ],
            "manual_review",
            &["L2_RULE_DETECTION", "L6_PROVIDER_GRAPH_RISK"],
        ),
        scheme(
            "genetic_testing_abuse",
            "Genetic testing abuse",
            "Laboratory",
            "Genetic test order, diagnosis, policy, or lab pattern indicates abuse risk.",
            &[
                "test_order",
                "diagnosis",
                "policy_rule",
                "medical_record",
                "lab_provider",
            ],
            "medical_review",
            &["L5_MEDICAL_REASONABLENESS", "L6_PROVIDER_GRAPH_RISK"],
        ),
        scheme(
            "pharmacy_controlled_substance_abuse",
            "Pharmacy or controlled-substance abuse",
            "Pharmacy",
            "Prescription, fill, dosage, prescriber, or member-history pattern indicates pharmacy risk.",
            &[
                "prescription",
                "prescriber",
                "fill_pattern",
                "dosage",
                "member_history",
                "policy_rule",
            ],
            "medical_review",
            &["L5_MEDICAL_REASONABLENESS", "L6_PROVIDER_GRAPH_RISK"],
        ),
        scheme(
            "dme_home_health_hospice_rehab_risk",
            "DME, home health, hospice, or rehabilitation risk",
            "Provider",
            "Supplier, facility, proof-of-service, or policy pattern indicates DME or care-setting risk.",
            &[
                "order",
                "supplier_provider",
                "medical_record",
                "delivery_or_service_proof",
                "policy_rule",
            ],
            "provider_review",
            &["L2_RULE_DETECTION", "L6_PROVIDER_GRAPH_RISK"],
        ),
        scheme(
            "provider_peer_outlier",
            "Provider peer outlier",
            "Provider",
            "Provider behavior materially deviates from specialty, region, or service peer group.",
            &[
                "peer_group_definition",
                "time_window",
                "specialty",
                "region",
                "statistical_deviation",
            ],
            "provider_review",
            &["L1_PEER_BENCHMARK", "L6_PROVIDER_GRAPH_RISK"],
        ),
        scheme(
            "relationship_concentration",
            "Relationship concentration",
            "Network",
            "Provider, member, referral, ownership, or affiliation graph is unusually concentrated.",
            &[
                "relationship_graph",
                "provider_member_link",
                "referral_pattern",
                "ownership_or_affiliation",
                "time_window",
            ],
            "investigation",
            &["L6_PROVIDER_GRAPH_RISK", "L7_RISK_FUSION_ROUTING"],
        ),
        scheme(
            "early_high_value_claim",
            "Early high-value claim",
            "Policy",
            "High-value claim occurs shortly after policy start and needs policy and clinical support.",
            &[
                "policy_start_date",
                "service_date",
                "claim_amount",
                "coverage_limit",
                "medical_record",
            ],
            "manual_review",
            &["L1_PEER_BENCHMARK", "L2_RULE_DETECTION"],
        ),
        scheme(
            "high_risk_claim",
            "High-risk claim",
            "General",
            "Fallback classification for high-risk leads before a specific scheme family is assigned.",
            &["claim_context", "risk_reason", "evidence_refs"],
            "manual_review",
            &["L7_RISK_FUSION_ROUTING"],
        ),
    ]
}

pub fn minimum_evidence_for_scheme(scheme_family: &str) -> Vec<String> {
    let canonical = canonical_scheme_family(scheme_family);
    fwa_scheme_taxonomy()
        .into_iter()
        .find(|scheme| Some(scheme.scheme_family.as_str()) == canonical.as_deref())
        .map(|scheme| scheme.minimum_evidence)
        .unwrap_or_else(|| {
            vec![
                "claim_context".into(),
                "risk_reason".into(),
                "evidence_refs".into(),
            ]
        })
}

pub fn assess_evidence_sufficiency(
    scheme_family: &str,
    evidence_text: &str,
) -> EvidenceSufficiency {
    let scheme_family =
        canonical_scheme_family(scheme_family).unwrap_or_else(|| "high_risk_claim".into());
    let minimum_evidence = minimum_evidence_for_scheme(&scheme_family);
    let evidence_text = evidence_text.to_ascii_lowercase();
    let present_evidence = minimum_evidence
        .iter()
        .filter(|item| evidence_item_present(item, &evidence_text))
        .cloned()
        .collect::<Vec<_>>();
    let missing_evidence = minimum_evidence
        .iter()
        .filter(|item| !present_evidence.contains(item))
        .cloned()
        .collect::<Vec<_>>();
    let status = if missing_evidence.is_empty() {
        "sufficient"
    } else {
        "needs_more_evidence"
    };

    EvidenceSufficiency {
        scheme_family,
        status: status.into(),
        minimum_evidence,
        present_evidence,
        missing_evidence,
    }
}

pub fn canonical_scheme_family(scheme_family: &str) -> Option<String> {
    let canonical = match scheme_family {
        "medical_necessity" => "medically_unnecessary_service",
        "lab_overuse" => "laboratory_testing_abuse",
        "pharmacy_or_opioid_abuse" => "pharmacy_controlled_substance_abuse",
        "provider_outlier" => "provider_peer_outlier",
        value => value,
    };
    fwa_scheme_taxonomy()
        .iter()
        .any(|scheme| scheme.scheme_family == canonical)
        .then(|| canonical.to_string())
}

fn evidence_item_present(item: &str, evidence_text: &str) -> bool {
    match item {
        "amount" | "claim_amount" => {
            evidence_text.contains("amount") || evidence_text.contains("金额")
        }
        "billed_code" | "component_codes" | "procedure" => {
            evidence_text.contains("code")
                || evidence_text.contains("procedure")
                || evidence_text.contains("项目")
        }
        "diagnosis" | "diagnosis_match" => {
            evidence_text.contains("diagnosis") || evidence_text.contains("诊断")
        }
        "documentation" | "medical_record" | "chart_note" => {
            evidence_text.contains("document")
                || evidence_text.contains("medical_record")
                || evidence_text.contains("病历")
        }
        "evidence_refs" => evidence_text.contains(':'),
        "peer_benchmark" | "peer_group_definition" => {
            evidence_text.contains("peer")
                || evidence_text.contains("同病种")
                || evidence_text.contains("同地区")
        }
        "policy_rule" => evidence_text.contains("policy") || evidence_text.contains("rule"),
        "provider" | "ordering_provider" | "prescriber" | "supplier_provider" | "lab_provider" => {
            evidence_text.contains("provider")
        }
        "region" | "provider_member_location" => {
            evidence_text.contains("region:") || evidence_text.contains("地区")
        }
        "risk_reason" => !evidence_text.trim().is_empty(),
        "statistical_deviation" => {
            evidence_text.contains("p99")
                || evidence_text.contains("percentile")
                || evidence_text.contains("zscore")
                || evidence_text.contains("偏离")
                || evidence_text.contains("高于")
        }
        "time_window" | "billing_timeline" => {
            evidence_text.contains("30d")
                || evidence_text.contains("90d")
                || evidence_text.contains("window")
                || evidence_text.contains("近")
        }
        other => evidence_text.contains(other),
    }
}

fn scheme(
    scheme_family: &str,
    display_name: &str,
    risk_domain: &str,
    description: &str,
    minimum_evidence: &[&str],
    default_review_route: &str,
    primary_layers: &[&str],
) -> FwaSchemeDefinition {
    FwaSchemeDefinition {
        scheme_family: scheme_family.into(),
        display_name: display_name.into(),
        risk_domain: risk_domain.into(),
        description: description.into(),
        minimum_evidence: minimum_evidence.iter().map(|item| (*item).into()).collect(),
        default_review_route: default_review_route.into(),
        primary_layers: primary_layers.iter().map(|item| (*item).into()).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taxonomy_includes_prd_scheme_families() {
        let families = fwa_scheme_taxonomy()
            .into_iter()
            .map(|scheme| scheme.scheme_family)
            .collect::<Vec<_>>();

        for expected in [
            "duplicate_billing",
            "upcoding",
            "unbundling",
            "medically_unnecessary_service",
            "excessive_utilization",
            "diagnosis_procedure_mismatch",
            "laboratory_testing_abuse",
            "telehealth_abuse",
            "genetic_testing_abuse",
            "pharmacy_controlled_substance_abuse",
            "dme_home_health_hospice_rehab_risk",
            "provider_peer_outlier",
            "relationship_concentration",
            "early_high_value_claim",
            "high_risk_claim",
        ] {
            assert!(
                families.contains(&expected.to_string()),
                "missing {expected}"
            );
        }
    }

    #[test]
    fn aliases_return_canonical_minimum_evidence() {
        assert_eq!(
            minimum_evidence_for_scheme("medical_necessity"),
            minimum_evidence_for_scheme("medically_unnecessary_service")
        );
        assert_eq!(
            canonical_scheme_family("lab_overuse"),
            Some("laboratory_testing_abuse".into())
        );
        assert!(minimum_evidence_for_scheme("provider_peer_outlier")
            .contains(&"peer_group_definition".into()));
    }

    #[test]
    fn assesses_evidence_sufficiency_from_scheme_taxonomy() {
        let assessment = assess_evidence_sufficiency(
            "provider_outlier",
            "provider peer group 同地区 P99 statistical_deviation",
        );

        assert_eq!(assessment.scheme_family, "provider_peer_outlier");
        assert_eq!(assessment.status, "needs_more_evidence");
        assert!(assessment
            .present_evidence
            .contains(&"peer_group_definition".into()));
        assert!(assessment.present_evidence.contains(&"region".into()));
        assert!(assessment
            .present_evidence
            .contains(&"statistical_deviation".into()));
        assert!(assessment.missing_evidence.contains(&"specialty".into()));
    }
}
