use super::*;

pub(super) async fn list_knowledge_cases(
    repository: &PostgresScoringRepository,
) -> anyhow::Result<Vec<KnowledgeCaseRecord>> {
    ensure_default_knowledge_cases_seeded(&repository.pool).await?;
    let rows: Vec<(
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        Value,
        Value,
    )> = sqlx::query_as(
        "SELECT case_id, title, fwa_type, scheme_family, diagnosis_code, provider_region, provider_type, summary, outcome, tags, evidence_refs
             FROM knowledge_cases
             ORDER BY case_id",
    )
    .fetch_all(&repository.pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(
                case_id,
                title,
                fwa_type,
                scheme_family,
                diagnosis_code,
                provider_region,
                provider_type,
                summary,
                outcome,
                tags,
                evidence_refs,
            )| KnowledgeCaseRecord {
                case_id,
                title,
                fwa_type,
                scheme_family,
                diagnosis_code,
                provider_region,
                provider_type,
                summary,
                outcome,
                tags: json_array_to_strings(tags),
                evidence_refs: json_array_to_strings(evidence_refs),
            },
        )
        .collect())
}

pub(super) async fn save_knowledge_case(
    repository: &PostgresScoringRepository,
    record: KnowledgeCaseRecord,
) -> anyhow::Result<KnowledgeCaseRecord> {
    sqlx::query(
        "INSERT INTO knowledge_cases
             (case_id, title, fwa_type, scheme_family, diagnosis_code, provider_region, provider_type, summary, outcome, tags, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             ON CONFLICT (case_id) DO UPDATE
             SET title = EXCLUDED.title,
                 fwa_type = EXCLUDED.fwa_type,
                 scheme_family = EXCLUDED.scheme_family,
                 diagnosis_code = EXCLUDED.diagnosis_code,
                 provider_region = EXCLUDED.provider_region,
                 provider_type = EXCLUDED.provider_type,
                 summary = EXCLUDED.summary,
                 outcome = EXCLUDED.outcome,
                 tags = EXCLUDED.tags,
                 evidence_refs = EXCLUDED.evidence_refs,
                 updated_at = now()",
    )
    .bind(&record.case_id)
    .bind(&record.title)
    .bind(&record.fwa_type)
    .bind(&record.scheme_family)
    .bind(&record.diagnosis_code)
    .bind(&record.provider_region)
    .bind(&record.provider_type)
    .bind(&record.summary)
    .bind(&record.outcome)
    .bind(serde_json::json!(record.tags))
    .bind(serde_json::json!(record.evidence_refs))
    .execute(&repository.pool)
    .await?;
    Ok(record)
}

pub(super) async fn search_similar_cases(
    repository: &PostgresScoringRepository,
    query: SimilarCaseQuery,
) -> anyhow::Result<Vec<SimilarCaseRecord>> {
    let cases = list_knowledge_cases(repository).await?;
    Ok(search_cases(cases, &query))
}
