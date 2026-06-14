use super::{KnowledgeCaseRecord, SimilarCaseQuery, SimilarCaseRecord};
use sqlx::PgPool;

pub(super) fn default_knowledge_cases() -> Vec<KnowledgeCaseRecord> {
    vec![
        KnowledgeCaseRecord {
            case_id: "KC-1001".into(),
            title: "Early high-amount respiratory claim".into(),
            fwa_type: "Abuse".into(),
            scheme_family: "diagnosis_procedure_mismatch".into(),
            diagnosis_code: "J10".into(),
            provider_region: "Shanghai".into(),
            provider_type: "hospital".into(),
            summary: "保单生效早期发生高额呼吸系统相关理赔，项目组合与相似已确认案例接近。".into(),
            outcome: "Manual review confirmed over-treatment pattern".into(),
            tags: vec![
                "early_claim".into(),
                "high_amount".into(),
                "medical_mismatch".into(),
            ],
            evidence_refs: vec![
                "knowledge_cases:KC-1001".into(),
                "rule_runs:EARLY_CLAIM".into(),
            ],
        },
        KnowledgeCaseRecord {
            case_id: "KC-1002".into(),
            title: "Provider repeated high-cost package pattern".into(),
            fwa_type: "Waste".into(),
            scheme_family: "provider_peer_outlier".into(),
            diagnosis_code: "M54".into(),
            provider_region: "Beijing".into(),
            provider_type: "clinic".into(),
            summary: "同一 provider 在短期内重复出现高价项目组合，金额分布显著偏离同地区 peer。"
                .into(),
            outcome: "Provider education and pre-payment review added".into(),
            tags: vec!["provider_pattern".into(), "high_amount".into()],
            evidence_refs: vec![
                "knowledge_cases:KC-1002".into(),
                "feature_values:provider_high_cost_item_ratio_30d".into(),
            ],
        },
    ]
}

pub(super) fn search_cases(
    cases: Vec<KnowledgeCaseRecord>,
    query: &SimilarCaseQuery,
) -> Vec<SimilarCaseRecord> {
    let mut results = cases
        .into_iter()
        .filter_map(|case| {
            let mut score: f64 = 0.0;
            let mut matched_signals = Vec::new();

            if case.diagnosis_code == query.diagnosis_code {
                score += 0.45;
                matched_signals.push(format!("diagnosis:{}", query.diagnosis_code));
            }
            if case.provider_region == query.provider_region {
                score += 0.25;
                matched_signals.push(format!("region:{}", query.provider_region));
            }
            for tag in &query.tags {
                if case.tags.iter().any(|case_tag| case_tag == tag) {
                    score += 0.15;
                    matched_signals.push(format!("tag:{tag}"));
                }
            }

            if score <= 0.0 {
                None
            } else {
                let mut provenance_refs = vec![
                    format!("knowledge_cases:{}", case.case_id),
                    "retrieval:structured_signal_overlap".into(),
                ];
                if let Some(claim_id) = &query.claim_id {
                    provenance_refs.push(format!("query_claim:{claim_id}"));
                }
                provenance_refs.extend(
                    matched_signals
                        .iter()
                        .map(|signal| format!("matched_signal:{signal}")),
                );

                Some(SimilarCaseRecord {
                    case_id: case.case_id,
                    title: case.title,
                    scheme_family: case.scheme_family,
                    similarity_score: score.min(1.0),
                    matched_signals,
                    retrieval_method: "structured_signal_overlap".into(),
                    provenance_refs,
                    summary: case.summary,
                    outcome: case.outcome,
                    evidence_refs: case.evidence_refs,
                })
            }
        })
        .collect::<Vec<_>>();

    results.sort_by(|left, right| {
        right
            .similarity_score
            .partial_cmp(&left.similarity_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results
}

pub(super) async fn ensure_default_knowledge_cases_seeded(pool: &PgPool) -> anyhow::Result<()> {
    for case in default_knowledge_cases() {
        sqlx::query(
            "INSERT INTO knowledge_cases
             (case_id, title, fwa_type, scheme_family, diagnosis_code, provider_region, provider_type, summary, outcome, tags, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             ON CONFLICT (case_id) DO UPDATE SET
               scheme_family = EXCLUDED.scheme_family,
               updated_at = now()",
        )
        .bind(&case.case_id)
        .bind(&case.title)
        .bind(&case.fwa_type)
        .bind(&case.scheme_family)
        .bind(&case.diagnosis_code)
        .bind(&case.provider_region)
        .bind(&case.provider_type)
        .bind(&case.summary)
        .bind(&case.outcome)
        .bind(serde_json::json!(case.tags))
        .bind(serde_json::json!(case.evidence_refs))
        .execute(pool)
        .await?;
    }
    Ok(())
}
