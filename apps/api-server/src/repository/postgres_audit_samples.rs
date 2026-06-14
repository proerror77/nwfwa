use super::case_rows::{load_audit_sample_strata_contexts, load_control_audit_population};
use super::*;

pub(super) async fn create_audit_sample(
    repository: &PostgresScoringRepository,
    input: CreateAuditSampleInput,
) -> anyhow::Result<AuditSampleRecord> {
    let sample_id = format!("sample_{}", AuditEventId::new());
    let customer_scope_filter = input.customer_scope_id.clone();
    let customer_scope_id = customer_scope_filter.as_deref();
    let leads = if input.sample_mode == "random_control" {
        load_control_audit_population(&repository.pool, customer_scope_id).await?
    } else {
        repository.list_leads(customer_scope_id).await?
    };
    let strata_contexts = load_audit_sample_strata_contexts(&repository.pool).await?;
    let existing_samples = list_audit_samples(repository, customer_scope_id).await?;
    let reviewer_history = reviewer_lead_sample_counts(existing_samples.iter(), &input.reviewer);
    let sample = build_audit_sample(
        sample_id,
        input,
        leads,
        &strata_contexts,
        &reviewer_history,
        None,
    );
    sqlx::query(
        "INSERT INTO audit_samples
         (sample_id, customer_scope_id, sample_mode, population_definition, inclusion_criteria_json, deterministic_seed, selection_method, sample_size, reviewer, assignment_queue, selected_leads_json, outcome_distribution_json)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
    )
    .bind(&sample.sample_id)
    .bind(&sample.customer_scope_id)
    .bind(&sample.sample_mode)
    .bind(&sample.population_definition)
    .bind(&sample.inclusion_criteria)
    .bind(&sample.deterministic_seed)
    .bind(&sample.selection_method)
    .bind(sample.sample_size as i32)
    .bind(&sample.reviewer)
    .bind(&sample.assignment_queue)
    .bind(serde_json::to_value(&sample.selected_leads)?)
    .bind(&sample.outcome_distribution)
    .execute(&repository.pool)
    .await?;
    list_audit_samples(repository, customer_scope_id)
        .await?
        .into_iter()
        .find(|record| record.sample_id == sample.sample_id)
        .ok_or_else(|| anyhow::anyhow!("created audit sample was not found"))
}

pub(super) async fn list_audit_samples(
    repository: &PostgresScoringRepository,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Vec<AuditSampleRecord>> {
    let rows: Vec<(
        String,
        String,
        String,
        String,
        Value,
        Option<String>,
        String,
        i32,
        String,
        String,
        Value,
        Value,
        chrono::DateTime<chrono::Utc>,
    )> = sqlx::query_as(
        "SELECT sample_id, customer_scope_id, sample_mode, population_definition, inclusion_criteria_json, deterministic_seed, selection_method, sample_size, reviewer, assignment_queue, selected_leads_json, outcome_distribution_json, created_at
         FROM audit_samples
         WHERE ($1::text IS NULL OR customer_scope_id = $1)
         ORDER BY created_at, sample_id",
    )
    .bind(customer_scope_id)
    .fetch_all(&repository.pool)
    .await?;
    let samples = rows
        .into_iter()
        .map(
            |(
                sample_id,
                customer_scope_id,
                sample_mode,
                population_definition,
                inclusion_criteria,
                deterministic_seed,
                selection_method,
                sample_size,
                reviewer,
                assignment_queue,
                selected_leads,
                outcome_distribution,
                created_at,
            )| AuditSampleRecord {
                sample_id,
                customer_scope_id,
                sample_mode,
                population_definition,
                inclusion_criteria,
                deterministic_seed,
                selection_method,
                sample_size: sample_size.max(0) as usize,
                reviewer,
                assignment_queue,
                selected_leads: serde_json::from_value(selected_leads).unwrap_or_default(),
                outcome_distribution,
                created_at: Some(created_at.to_rfc3339()),
            },
        )
        .collect::<Vec<_>>();
    let reviews = repository.list_qa_reviews(customer_scope_id).await?;
    Ok(with_sample_outcome_distributions(samples, &reviews))
}
