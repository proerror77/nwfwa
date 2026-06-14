use super::*;

impl InMemoryScoringRepository {
    pub(super) async fn in_memory_list_models(&self) -> anyhow::Result<Vec<ModelVersionRecord>> {
        let statuses = self.model_statuses.lock().await;
        let mut models = default_model_versions();
        models.extend(self.model_versions.lock().await.values().cloned());
        models.sort_by(|left, right| {
            left.model_key
                .cmp(&right.model_key)
                .then_with(|| right.version.cmp(&left.version))
        });
        Ok(models
            .into_iter()
            .map(|mut model| {
                if let Some(status) =
                    statuses.get(&model_version_key(&model.model_key, &model.version))
                {
                    model.status = status.clone();
                }
                model
            })
            .collect())
    }

    pub(super) async fn in_memory_save_model_version(
        &self,
        record: ModelVersionRecord,
    ) -> anyhow::Result<ModelVersionRecord> {
        self.model_versions.lock().await.insert(
            model_version_key(&record.model_key, &record.version),
            record.clone(),
        );
        Ok(record)
    }

    pub(super) async fn in_memory_update_model_status(
        &self,
        model_key: &str,
        model_version: &str,
        status: &str,
    ) -> anyhow::Result<Option<ModelVersionRecord>> {
        let mut models = self.in_memory_list_models().await?;
        let Some(model) = models
            .iter_mut()
            .find(|model| model.model_key == model_key && model.version == model_version)
        else {
            return Ok(None);
        };
        model.status = status.to_string();
        self.model_statuses.lock().await.insert(
            model_version_key(model_key, model_version),
            status.to_string(),
        );
        Ok(Some(model.clone()))
    }

    pub(super) async fn in_memory_model_performance(
        &self,
        model_key: &str,
    ) -> anyhow::Result<Option<ModelPerformanceRecord>> {
        if default_model_versions()
            .iter()
            .any(|model| model.model_key == model_key)
        {
            let evaluations = self.model_evaluations.lock().await;
            let drift = evaluations
                .values()
                .filter(|evaluation| evaluation.model_key == model_key)
                .max_by(|left, right| left.evaluation_run_id.cmp(&right.evaluation_run_id))
                .map(|evaluation| drift_summary(&evaluation.metrics_json))
                .unwrap_or_else(|| drift_summary(&Value::Null));
            Ok(Some(model_performance_with_drift(
                empty_model_performance(model_key),
                drift,
            )))
        } else {
            Ok(None)
        }
    }

    pub(super) async fn in_memory_save_model_promotion_review(
        &self,
        mut record: ModelPromotionReviewRecord,
    ) -> anyhow::Result<ModelPromotionReviewRecord> {
        record.created_at = Some(chrono::Utc::now().to_rfc3339());
        self.model_promotion_reviews
            .lock()
            .await
            .push(record.clone());
        Ok(record)
    }

    pub(super) async fn in_memory_latest_model_promotion_review(
        &self,
        model_key: &str,
        model_version: &str,
    ) -> anyhow::Result<Option<ModelPromotionReviewRecord>> {
        Ok(self
            .model_promotion_reviews
            .lock()
            .await
            .iter()
            .rev()
            .find(|review| review.model_key == model_key && review.model_version == model_version)
            .cloned())
    }

    pub(super) async fn in_memory_save_model_retraining_job(
        &self,
        mut record: ModelRetrainingJobRecord,
    ) -> anyhow::Result<ModelRetrainingJobRecord> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut sequence = self.model_retraining_job_sequence.lock().await;
        *sequence += 1;
        record.job_id = format!("model_retraining_job_{}", *sequence);
        record.created_at = Some(now.clone());
        record.updated_at = Some(now);
        self.model_retraining_jobs
            .lock()
            .await
            .insert(record.job_id.clone(), record.clone());
        Ok(record)
    }

    pub(super) async fn in_memory_list_model_retraining_jobs(
        &self,
        model_key: &str,
    ) -> anyhow::Result<Vec<ModelRetrainingJobRecord>> {
        let mut jobs = self
            .model_retraining_jobs
            .lock()
            .await
            .values()
            .filter(|job| job.model_key == model_key)
            .cloned()
            .collect::<Vec<_>>();
        jobs.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        Ok(jobs)
    }

    pub(super) async fn in_memory_get_model_retraining_job(
        &self,
        job_id: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        Ok(self.model_retraining_jobs.lock().await.get(job_id).cloned())
    }

    pub(super) async fn in_memory_claim_next_model_retraining_job(
        &self,
        model_key: Option<&str>,
        actor: &str,
        status_note: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        let mut jobs = self.model_retraining_jobs.lock().await;
        let next_job_id = jobs
            .values()
            .filter(|job| job.status == "queued")
            .filter(|job| model_key.map(|key| job.model_key == key).unwrap_or(true))
            .min_by(|left, right| left.created_at.cmp(&right.created_at))
            .map(|job| job.job_id.clone());
        let Some(job_id) = next_job_id else {
            return Ok(None);
        };
        let Some(job) = jobs.get_mut(&job_id) else {
            return Ok(None);
        };
        job.status = "running".into();
        job.updated_by = actor.to_string();
        job.status_note = status_note.to_string();
        job.updated_at = Some(chrono::Utc::now().to_rfc3339());
        Ok(Some(job.clone()))
    }

    pub(super) async fn in_memory_update_model_retraining_job_status(
        &self,
        job_id: &str,
        status: &str,
        actor: &str,
        status_note: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        let mut jobs = self.model_retraining_jobs.lock().await;
        let Some(job) = jobs.get_mut(job_id) else {
            return Ok(None);
        };
        job.status = status.to_string();
        job.updated_by = actor.to_string();
        job.status_note = status_note.to_string();
        job.updated_at = Some(chrono::Utc::now().to_rfc3339());
        Ok(Some(job.clone()))
    }

    pub(super) async fn in_memory_complete_model_retraining_job(
        &self,
        input: CompleteModelRetrainingJobInput<'_>,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        let mut jobs = self.model_retraining_jobs.lock().await;
        let Some(job) = jobs.get_mut(input.job_id) else {
            return Ok(None);
        };
        job.status = "completed".into();
        job.updated_by = input.actor.to_string();
        job.status_note = input.status_note.to_string();
        job.candidate_model_version = Some(input.candidate_model_version.to_string());
        job.candidate_artifact_uri = Some(input.candidate_artifact_uri.to_string());
        job.candidate_endpoint_url = input.candidate_endpoint_url.map(ToString::to_string);
        job.validation_report_uri = Some(input.validation_report_uri.to_string());
        job.output_evaluation_id = Some(input.output_evaluation_id.to_string());
        job.updated_at = Some(chrono::Utc::now().to_rfc3339());
        Ok(Some(job.clone()))
    }
}
