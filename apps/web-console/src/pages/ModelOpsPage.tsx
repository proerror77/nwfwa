import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  completeModelRetrainingJob,
  createModelRetrainingJob,
  getModelPerformance,
  getModelPromotionGates,
  getModelRetrainingReadiness,
  listAuditEvents,
  listModelRetrainingJobs,
  listOutcomeLabels,
  listQaFeedbackItems,
  listModels,
  rollbackModel,
  submitModelPromotionReview,
  updateModelRetrainingJobStatus,
} from "../api";
import {
  filterQaFeedbackItems,
  QaFeedbackItem,
  summarizeQaFeedbackItems,
} from "./qaFeedbackItems";
import {
  buildPromotionGateEvidenceRows,
  type PromotionGate,
} from "./promotionGateEvidence";
import { formatReviewModeLabel } from "./reviewMode";

type ModelVersion = {
  model_key: string;
  version: string;
  model_type: string;
  runtime_kind: string;
  execution_provider: string;
  status: string;
  review_mode: string;
  artifact_uri: string | null;
  endpoint_url: string | null;
};

type ModelPromotionGatesResponse = {
  review_mode: string;
  decision: string;
  passed_count: number;
  total_count: number;
  latest_evaluation_id: string;
  source_dataset_id: string;
  source_data_quality_score: number | null;
  source_data_quality_status: string;
  data_status: string;
  scored_runs: number;
  open_model_feedback_count: number;
  approved_label_count: number;
  needs_review_label_count: number;
  blockers: string[];
  gates: PromotionGate[];
};

type ModelPerformanceResponse = {
  data_status: string;
  scored_runs: number;
  average_score: number;
  high_risk_count: number;
  score_psi: number | null;
  drift_status: string;
};

type ModelRetrainingReadinessResponse = {
  recommendation: string;
  latest_evaluation_id: string;
  drift_status: string;
  source_dataset_id: string;
  source_data_quality_score: number | null;
  source_data_quality_status: string;
  open_model_feedback_count: number;
  approved_label_count: number;
  needs_review_label_count: number;
  retraining_triggers: string[];
  blockers: string[];
};

type ModelRetrainingJob = {
  job_id: string;
  model_key: string;
  model_version: string;
  status: string;
  requested_by: string;
  request_notes: string;
  status_note: string;
  updated_by: string;
  readiness_recommendation: string;
  trigger_summary: string[];
  blocker_summary: string[];
  candidate_model_version: string | null;
  candidate_artifact_uri: string | null;
  candidate_endpoint_url: string | null;
  validation_report_uri: string | null;
  output_evaluation_id: string | null;
  created_at: string | null;
  updated_at: string | null;
};

type OutcomeLabel = {
  label_id: string;
  claim_id: string;
  label_name: string;
  label_value: string;
  source_type: string;
  source_id: string;
  governance_status: string;
  feedback_target: string;
  currency?: string | null;
  evidence_refs: string[];
};

type AuditEvent = {
  audit_id: string;
  run_id: string;
  event_type: string;
  event_status: string;
  summary: string;
  evidence_refs: string[];
  created_at?: string | null;
};

export function buildModelLabelReadinessSummary(labels: OutcomeLabel[] = []) {
  const modelLabels = labels.filter((label) => label.feedback_target === "models");
  return {
    modelLabelCount: modelLabels.length,
    approvedForTrainingCount: modelLabels.filter(
      (label) => label.governance_status === "approved_for_training",
    ).length,
    needsReviewCount: modelLabels.filter((label) => label.governance_status === "needs_review")
      .length,
    evidenceBackedCount: modelLabels.filter((label) => label.evidence_refs.length > 0).length,
    confirmedFwaCount: modelLabels.filter(
      (label) => label.label_name === "confirmed_fwa" && label.label_value === "true",
    ).length,
  };
}

export function buildModelAuditFilters(model: ModelVersion, limit = 25) {
  return {
    limit,
    model_key: model.model_key,
    model_version: model.version,
  };
}

export function formatSourceDataQuality(score?: number | null) {
  return score == null ? "-" : `${(score * 100).toFixed(1)}%`;
}

export function buildModelRetrainingSummary(
  readiness?: ModelRetrainingReadinessResponse | null,
) {
  return {
    recommendation: readiness?.recommendation ?? "not_loaded",
    triggerCount: readiness?.retraining_triggers.length ?? 0,
    blockerCount: readiness?.blockers.length ?? 0,
    openFeedbackCount: readiness?.open_model_feedback_count ?? 0,
    approvedLabelCount: readiness?.approved_label_count ?? 0,
    sourceDataQualityLabel: formatSourceDataQuality(readiness?.source_data_quality_score),
    sourceDataQualityStatus: readiness?.source_data_quality_status ?? "missing",
  };
}

export function buildModelRetrainingJobSummary(jobs: ModelRetrainingJob[] = []) {
  return {
    jobCount: jobs.length,
    queuedCount: jobs.filter((job) => job.status === "queued").length,
    runningCount: jobs.filter((job) => job.status === "running").length,
    completedCount: jobs.filter((job) => job.status === "completed").length,
    latestStatus: jobs[0]?.status ?? "none",
  };
}

export function ModelOpsPage() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const [selectedModelKey, setSelectedModelKey] = useState("baseline_fwa");
  const [reviewer, setReviewer] = useState("model-governance");
  const [notes, setNotes] = useState("Approved for continued shadow evaluation only.");
  const [retrainingRequester, setRetrainingRequester] = useState("model-ops");
  const [retrainingNotes, setRetrainingNotes] = useState("Queue retraining from readiness.");
  const [candidateVersion, setCandidateVersion] = useState("0.2.0-candidate");
  const [candidateArtifactUri, setCandidateArtifactUri] = useState(
    "s3://fwa-models/baseline_fwa/0.2.0-candidate/model.onnx",
  );
  const [validationReportUri, setValidationReportUri] = useState(
    "s3://fwa-models/baseline_fwa/0.2.0-candidate/validation.json",
  );
  const [evaluationRunId, setEvaluationRunId] = useState(
    "eval_baseline_fwa_0_2_0_candidate",
  );
  const queryClient = useQueryClient();
  const modelsQuery = useQuery({
    queryKey: ["models", apiKey],
    queryFn: () => listModels(apiKey) as Promise<{ models: ModelVersion[] }>,
  });
  const selectedModel = useMemo(
    () =>
      modelsQuery.data?.models.find((model) => model.model_key === selectedModelKey) ??
      modelsQuery.data?.models[0],
    [modelsQuery.data?.models, selectedModelKey],
  );
  const performanceQuery = useQuery({
    queryKey: ["model-performance", selectedModel?.model_key, apiKey],
    queryFn: () =>
      getModelPerformance(
        selectedModel!.model_key,
        apiKey,
      ) as Promise<ModelPerformanceResponse>,
    enabled: Boolean(selectedModel?.model_key),
  });
  const promotionQuery = useQuery({
    queryKey: ["model-promotion-gates", selectedModel?.model_key, apiKey],
    queryFn: () =>
      getModelPromotionGates(
        selectedModel!.model_key,
        apiKey,
      ) as Promise<ModelPromotionGatesResponse>,
    enabled: Boolean(selectedModel?.model_key),
  });
  const auditQuery = useQuery({
    queryKey: [
      "model-audit-events",
      selectedModel?.model_key,
      selectedModel?.version,
      apiKey,
    ],
    queryFn: () =>
      listAuditEvents(
        apiKey,
        buildModelAuditFilters(selectedModel!),
      ) as Promise<{ events: AuditEvent[] }>,
    enabled: Boolean(selectedModel?.model_key),
  });
  const retrainingQuery = useQuery({
    queryKey: ["model-retraining-readiness", selectedModel?.model_key, apiKey],
    queryFn: () =>
      getModelRetrainingReadiness(
        selectedModel!.model_key,
        apiKey,
      ) as Promise<ModelRetrainingReadinessResponse>,
    enabled: Boolean(selectedModel?.model_key),
  });
  const retrainingJobsQuery = useQuery({
    queryKey: ["model-retraining-jobs", selectedModel?.model_key, apiKey],
    queryFn: () =>
      listModelRetrainingJobs(
        selectedModel!.model_key,
        apiKey,
      ) as Promise<{ jobs: ModelRetrainingJob[] }>,
    enabled: Boolean(selectedModel?.model_key),
  });
  const qaFeedbackQuery = useQuery({
    queryKey: ["qa-feedback-items", "models", apiKey],
    queryFn: () =>
      listQaFeedbackItems(apiKey, { feedbackTarget: "models" }) as Promise<{
        items: QaFeedbackItem[];
      }>,
  });
  const outcomeLabelsQuery = useQuery({
    queryKey: ["outcome-labels", apiKey],
    queryFn: () => listOutcomeLabels(apiKey) as Promise<{ labels: OutcomeLabel[] }>,
  });
  const modelFeedbackItems = useMemo(
    () => filterQaFeedbackItems(qaFeedbackQuery.data?.items ?? [], "models"),
    [qaFeedbackQuery.data?.items],
  );
  const modelFeedbackSummary = useMemo(
    () => summarizeQaFeedbackItems(modelFeedbackItems),
    [modelFeedbackItems],
  );
  const modelLabelSummary = useMemo(
    () => buildModelLabelReadinessSummary(outcomeLabelsQuery.data?.labels),
    [outcomeLabelsQuery.data?.labels],
  );
  const reviewMutation = useMutation({
    mutationFn: (decision: "approved" | "rejected") => {
      if (!selectedModel) throw new Error("No model selected");
      return submitModelPromotionReview(
        selectedModel.model_key,
        { decision, reviewer, notes },
        apiKey,
      );
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["model-promotion-gates"] });
      queryClient.invalidateQueries({ queryKey: ["model-audit-events"] });
    },
  });
  const rollbackMutation = useMutation({
    mutationFn: () => {
      if (!selectedModel) throw new Error("No model selected");
      return rollbackModel(selectedModel.model_key, apiKey);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["models"] });
      queryClient.invalidateQueries({ queryKey: ["model-promotion-gates"] });
      queryClient.invalidateQueries({ queryKey: ["model-audit-events"] });
    },
  });
  const retrainingCreateMutation = useMutation({
    mutationFn: () => {
      if (!selectedModel) throw new Error("No model selected");
      return createModelRetrainingJob(
        selectedModel.model_key,
        { requested_by: retrainingRequester, notes: retrainingNotes },
        apiKey,
      );
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["model-retraining-jobs"] });
      queryClient.invalidateQueries({ queryKey: ["model-retraining-readiness"] });
      queryClient.invalidateQueries({ queryKey: ["model-audit-events"] });
    },
  });
  const retrainingStatusMutation = useMutation({
    mutationFn: ({ jobId, status }: { jobId: string; status: string }) =>
      updateModelRetrainingJobStatus(
        jobId,
        { status, actor: retrainingRequester, notes: `Marked ${status}` },
        apiKey,
      ),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["model-retraining-jobs"] });
      queryClient.invalidateQueries({ queryKey: ["model-audit-events"] });
    },
  });
  const retrainingCompleteMutation = useMutation({
    mutationFn: ({ jobId }: { jobId: string }) =>
      completeModelRetrainingJob(
        jobId,
        {
          actor: retrainingRequester,
          notes: "Candidate model and validation report registered.",
          candidate_model_version: candidateVersion,
          artifact_uri: candidateArtifactUri,
          validation_report_uri: validationReportUri,
          evaluation_run_id: evaluationRunId,
          auc: "0.84",
          ks: "0.45",
          precision: "0.76",
          recall: "0.70",
          f1: "0.73",
          accuracy: "0.78",
          threshold: "0.50",
          confusion_matrix_json: { tp: 24, fp: 6, tn: 52, fn: 8 },
          feature_importance_uri:
            "s3://fwa-models/baseline_fwa/0.2.0-candidate/feature_importance.parquet",
          metrics_json: {
            score_psi: 0.04,
            review_capacity_threshold_status: "passed",
            shadow_comparison_status: "passed",
          },
        },
        apiKey,
      ),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["models"] });
      queryClient.invalidateQueries({ queryKey: ["model-retraining-jobs"] });
      queryClient.invalidateQueries({ queryKey: ["model-evaluations"] });
      queryClient.invalidateQueries({ queryKey: ["model-audit-events"] });
    },
  });
  const promotionGateRows = promotionQuery.data
    ? buildPromotionGateEvidenceRows(promotionQuery.data.gates)
    : [];
  const retrainingSummary = buildModelRetrainingSummary(retrainingQuery.data);
  const retrainingJobSummary = buildModelRetrainingJobSummary(retrainingJobsQuery.data?.jobs);

  return (
    <section className="ops-grid">
      <div className="panel">
        <h2>Models</h2>
        <label>
          API Key
          <input value={apiKey} onChange={(event) => setApiKey(event.target.value)} />
        </label>
        {modelsQuery.error ? <pre className="error">{String(modelsQuery.error.message)}</pre> : null}
        <div className="table-list">
          {modelsQuery.data?.models.map((model) => (
            <button
              className={
                model.model_key === selectedModel?.model_key ? "row-button active" : "row-button"
              }
              key={`${model.model_key}-${model.version}`}
              onClick={() => setSelectedModelKey(model.model_key)}
            >
              <span>{model.model_key}</span>
              <strong>{model.status}</strong>
              <small>
                {model.runtime_kind} · {formatReviewModeLabel(model.review_mode)}
              </small>
            </button>
          ))}
        </div>
      </div>
      <div className="panel">
        <h2>Model Detail</h2>
        {selectedModel ? (
          <dl className="result-grid">
            <div>
              <dt>Model</dt>
              <dd>{selectedModel.model_key}</dd>
            </div>
            <div>
              <dt>Version</dt>
              <dd>{selectedModel.version}</dd>
            </div>
            <div>
              <dt>Type</dt>
              <dd>{selectedModel.model_type}</dd>
            </div>
            <div>
              <dt>Runtime</dt>
              <dd>{selectedModel.runtime_kind}</dd>
            </div>
            <div>
              <dt>Provider</dt>
              <dd>{selectedModel.execution_provider}</dd>
            </div>
            <div>
              <dt>Status</dt>
              <dd>{selectedModel.status}</dd>
            </div>
            <div>
              <dt>Review Mode</dt>
              <dd>{formatReviewModeLabel(selectedModel.review_mode)}</dd>
            </div>
          </dl>
        ) : (
          <p className="empty">No models available</p>
        )}
      </div>
      <div className="panel wide-panel">
        <h2>Performance</h2>
        {performanceQuery.error ? (
          <pre className="error">{String(performanceQuery.error.message)}</pre>
        ) : null}
        {performanceQuery.data ? (
          <dl className="result-grid">
            <div>
              <dt>Data Status</dt>
              <dd>{performanceQuery.data.data_status}</dd>
            </div>
            <div>
              <dt>Scored Runs</dt>
              <dd>{performanceQuery.data.scored_runs}</dd>
            </div>
            <div>
              <dt>Average Score</dt>
              <dd>{performanceQuery.data.average_score}</dd>
            </div>
            <div>
              <dt>High Risk</dt>
              <dd>{performanceQuery.data.high_risk_count}</dd>
            </div>
            <div>
              <dt>Score PSI</dt>
              <dd>{performanceQuery.data.score_psi ?? "-"}</dd>
            </div>
            <div>
              <dt>Drift Status</dt>
              <dd>{performanceQuery.data.drift_status}</dd>
            </div>
          </dl>
        ) : (
          <p className="empty">No performance data loaded</p>
        )}
      </div>
      <div className="panel wide-panel">
        <h2>Model Audit Trail</h2>
        {auditQuery.error ? (
          <pre className="error">{String(auditQuery.error.message)}</pre>
        ) : null}
        {auditQuery.data?.events.length ? (
          <ol className="audit-timeline">
            {auditQuery.data.events.map((event) => (
              <li key={event.audit_id}>
                <div>
                  <strong>{event.event_type}</strong>
                  <span>{event.event_status}</span>
                </div>
                <small>{event.created_at || event.run_id}</small>
                <p>{event.summary}</p>
                <ul className="result-list">
                  {event.evidence_refs.map((reference) => (
                    <li key={reference}>{reference}</li>
                  ))}
                </ul>
              </li>
            ))}
          </ol>
        ) : (
          <p className="empty">No model audit events loaded</p>
        )}
      </div>
      <div className="panel wide-panel">
        <h2>Retraining Readiness</h2>
        {retrainingQuery.error ? (
          <pre className="error">{String(retrainingQuery.error.message)}</pre>
        ) : null}
        {retrainingQuery.data ? (
          <>
            <div className="summary-grid">
              <div>
                <span>Recommendation</span>
                <strong>{retrainingSummary.recommendation}</strong>
              </div>
              <div>
                <span>Triggers</span>
                <strong>{retrainingSummary.triggerCount}</strong>
              </div>
              <div>
                <span>Blockers</span>
                <strong>{retrainingSummary.blockerCount}</strong>
              </div>
              <div>
                <span>Open Feedback</span>
                <strong>{retrainingSummary.openFeedbackCount}</strong>
              </div>
              <div>
                <span>Training Labels</span>
                <strong>{retrainingSummary.approvedLabelCount}</strong>
              </div>
              <div>
                <span>Source DQ</span>
                <strong>{retrainingSummary.sourceDataQualityLabel}</strong>
              </div>
              <div>
                <span>Source DQ Status</span>
                <strong>{retrainingSummary.sourceDataQualityStatus}</strong>
              </div>
            </div>
            <div className="table-list">
              {retrainingQuery.data.retraining_triggers.map((trigger) => (
                <div className="metric-row compact-metric-row" key={trigger}>
                  <span>{trigger}</span>
                  <strong>trigger</strong>
                </div>
              ))}
              {retrainingQuery.data.blockers.map((blocker) => (
                <div className="metric-row compact-metric-row" key={blocker}>
                  <span>{blocker}</span>
                  <strong>blocker</strong>
                </div>
              ))}
            </div>
          </>
        ) : (
          <p className="empty">No retraining readiness loaded</p>
        )}
      </div>
      <div className="panel wide-panel">
        <h2>Retraining Jobs</h2>
        {retrainingJobsQuery.error ? (
          <pre className="error">{String(retrainingJobsQuery.error.message)}</pre>
        ) : null}
        <div className="summary-grid">
          <div>
            <span>Jobs</span>
            <strong>{retrainingJobSummary.jobCount}</strong>
          </div>
          <div>
            <span>Queued</span>
            <strong>{retrainingJobSummary.queuedCount}</strong>
          </div>
          <div>
            <span>Running</span>
            <strong>{retrainingJobSummary.runningCount}</strong>
          </div>
          <div>
            <span>Completed</span>
            <strong>{retrainingJobSummary.completedCount}</strong>
          </div>
          <div>
            <span>Latest Status</span>
            <strong>{retrainingJobSummary.latestStatus}</strong>
          </div>
        </div>
        <div className="result-stack">
          <label>
            Requester
            <input
              value={retrainingRequester}
              onChange={(event) => setRetrainingRequester(event.target.value)}
            />
          </label>
          <label>
            Retraining Note
            <textarea
              value={retrainingNotes}
              onChange={(event) => setRetrainingNotes(event.target.value)}
            />
          </label>
          <button
            onClick={() => retrainingCreateMutation.mutate()}
            disabled={retrainingCreateMutation.isPending}
          >
            Queue Retraining
          </button>
          {retrainingCreateMutation.error ? (
            <pre className="error">{String(retrainingCreateMutation.error.message)}</pre>
          ) : null}
        </div>
        <div className="result-stack">
          <label>
            Candidate Version
            <input
              value={candidateVersion}
              onChange={(event) => setCandidateVersion(event.target.value)}
            />
          </label>
          <label>
            Artifact URI
            <input
              value={candidateArtifactUri}
              onChange={(event) => setCandidateArtifactUri(event.target.value)}
            />
          </label>
          <label>
            Validation Report URI
            <input
              value={validationReportUri}
              onChange={(event) => setValidationReportUri(event.target.value)}
            />
          </label>
          <label>
            Evaluation Run
            <input
              value={evaluationRunId}
              onChange={(event) => setEvaluationRunId(event.target.value)}
            />
          </label>
          {retrainingCompleteMutation.error ? (
            <pre className="error">{String(retrainingCompleteMutation.error.message)}</pre>
          ) : null}
        </div>
        <div className="table-list">
          {(retrainingJobsQuery.data?.jobs ?? []).map((job) => (
            <div className="metric-row compact-metric-row" key={job.job_id}>
              <span>{job.job_id}</span>
              <strong>{job.status}</strong>
              <small>
                {job.model_version} · {job.requested_by}
              </small>
              <small>{job.trigger_summary.length} triggers</small>
              {job.candidate_model_version ? (
                <small>
                  {job.candidate_model_version} · {job.output_evaluation_id ?? "no eval"}
                </small>
              ) : null}
              <div className="button-row">
                <button
                  onClick={() =>
                    retrainingStatusMutation.mutate({
                      jobId: job.job_id,
                      status: "running",
                    })
                  }
                  disabled={retrainingStatusMutation.isPending || job.status !== "queued"}
                >
                  Start
                </button>
                <button
                  onClick={() =>
                    retrainingStatusMutation.mutate({
                      jobId: job.job_id,
                      status: "validation",
                    })
                  }
                  disabled={retrainingStatusMutation.isPending || job.status !== "running"}
                >
                  Validate
                </button>
                <button
                  onClick={() =>
                    retrainingCompleteMutation.mutate({
                      jobId: job.job_id,
                    })
                  }
                  disabled={retrainingCompleteMutation.isPending || job.status !== "validation"}
                >
                  Complete
                </button>
              </div>
            </div>
          ))}
        </div>
        {(retrainingJobsQuery.data?.jobs ?? []).length === 0 ? (
          <p className="empty">No retraining jobs</p>
        ) : null}
      </div>
      <div className="panel wide-panel">
        <h2>QA Feedback</h2>
        {qaFeedbackQuery.error ? (
          <pre className="error">{String(qaFeedbackQuery.error.message)}</pre>
        ) : null}
        <div className="summary-grid">
          <div>
            <span>Open Items</span>
            <strong>{modelFeedbackSummary.openCount}</strong>
          </div>
          <div>
            <span>Highest Priority</span>
            <strong>{modelFeedbackSummary.highestPriority}</strong>
          </div>
          <div>
            <span>Evidence Backed</span>
            <strong>{modelFeedbackSummary.evidenceBackedCount}</strong>
          </div>
        </div>
        <div className="table-list">
          {modelFeedbackItems.map((item) => (
            <div className="metric-row compact-metric-row" key={item.feedback_id}>
              <span>{item.summary}</span>
              <strong>{item.issue_type}</strong>
              <small>
                {item.priority} · {item.status}
              </small>
              <small>{item.evidence_refs.length} evidence refs</small>
            </div>
          ))}
        </div>
        {modelFeedbackItems.length === 0 ? <p className="empty">No model feedback items</p> : null}
      </div>
      <div className="panel wide-panel">
        <h2>Label Readiness</h2>
        {outcomeLabelsQuery.error ? (
          <pre className="error">{String(outcomeLabelsQuery.error.message)}</pre>
        ) : null}
        <div className="summary-grid">
          <div>
            <span>Model Labels</span>
            <strong>{modelLabelSummary.modelLabelCount}</strong>
          </div>
          <div>
            <span>Training Ready</span>
            <strong>{modelLabelSummary.approvedForTrainingCount}</strong>
          </div>
          <div>
            <span>Needs Review</span>
            <strong>{modelLabelSummary.needsReviewCount}</strong>
          </div>
          <div>
            <span>Evidence Backed</span>
            <strong>{modelLabelSummary.evidenceBackedCount}</strong>
          </div>
          <div>
            <span>Confirmed FWA</span>
            <strong>{modelLabelSummary.confirmedFwaCount}</strong>
          </div>
        </div>
        <div className="table-list">
          {(outcomeLabelsQuery.data?.labels ?? [])
            .filter((label) => label.feedback_target === "models")
            .map((label) => (
              <div className="metric-row compact-metric-row" key={label.label_id}>
                <span>{label.label_name}</span>
                <strong>{label.governance_status}</strong>
                <small>
                  {label.claim_id} · {label.source_type}:{label.source_id}
                </small>
                <small>{label.evidence_refs.length} evidence refs</small>
              </div>
            ))}
        </div>
        {modelLabelSummary.modelLabelCount === 0 ? (
          <p className="empty">No model outcome labels</p>
        ) : null}
      </div>
      <div className="panel wide-panel">
        <h2>Promotion Gates</h2>
        {promotionQuery.error ? (
          <pre className="error">{String(promotionQuery.error.message)}</pre>
        ) : null}
        {promotionQuery.data ? (
          <>
            <div className="summary-grid">
              <div>
                <span>Routing Decision</span>
                <strong>{promotionQuery.data.decision}</strong>
              </div>
              <div>
                <span>Review Mode</span>
                <strong>{formatReviewModeLabel(promotionQuery.data.review_mode)}</strong>
              </div>
              <div>
                <span>Gates Passed</span>
                <strong>
                  {promotionQuery.data.passed_count}/{promotionQuery.data.total_count}
                </strong>
              </div>
              <div>
                <span>Latest Evaluation</span>
                <strong>{promotionQuery.data.latest_evaluation_id}</strong>
              </div>
              <div>
                <span>Source Dataset</span>
                <strong>{promotionQuery.data.source_dataset_id}</strong>
              </div>
              <div>
                <span>Source DQ</span>
                <strong>
                  {formatSourceDataQuality(promotionQuery.data.source_data_quality_score)}
                </strong>
              </div>
              <div>
                <span>Source DQ Status</span>
                <strong>{promotionQuery.data.source_data_quality_status}</strong>
              </div>
              <div>
                <span>Runtime Data</span>
                <strong>{promotionQuery.data.data_status}</strong>
              </div>
              <div>
                <span>Scored Runs</span>
                <strong>{promotionQuery.data.scored_runs}</strong>
              </div>
              <div>
                <span>Open Feedback</span>
                <strong>{promotionQuery.data.open_model_feedback_count}</strong>
              </div>
              <div>
                <span>Approved Labels</span>
                <strong>{promotionQuery.data.approved_label_count}</strong>
              </div>
              <div>
                <span>Labels Need Review</span>
                <strong>{promotionQuery.data.needs_review_label_count}</strong>
              </div>
              <div>
                <span>Blockers</span>
                <strong>{promotionQuery.data.blockers.length}</strong>
              </div>
            </div>
            <div className="table-list">
              {promotionGateRows.map((gate) => (
                <div className="metric-row compact-metric-row" key={gate.label}>
                  <span>{gate.label}</span>
                  <strong>{gate.status}</strong>
                  <small className={gate.evidenceClassName}>{gate.evidenceSource}</small>
                </div>
              ))}
            </div>
            <div className="result-stack">
              <label>
                Reviewer
                <input value={reviewer} onChange={(event) => setReviewer(event.target.value)} />
              </label>
              <label>
                Governance Note
                <textarea value={notes} onChange={(event) => setNotes(event.target.value)} />
              </label>
              <div className="button-row">
                <button
                  onClick={() => reviewMutation.mutate("approved")}
                  disabled={reviewMutation.isPending}
                >
                  Approve
                </button>
                <button
                  onClick={() => reviewMutation.mutate("rejected")}
                  disabled={reviewMutation.isPending}
                >
                  Reject
                </button>
                <button
                  onClick={() => rollbackMutation.mutate()}
                  disabled={rollbackMutation.isPending}
                >
                  Rollback
                </button>
              </div>
              {reviewMutation.error ? (
                <pre className="error">{String(reviewMutation.error.message)}</pre>
              ) : null}
              {rollbackMutation.error ? (
                <pre className="error">{String(rollbackMutation.error.message)}</pre>
              ) : null}
            </div>
          </>
        ) : (
          <p className="empty">No promotion gate data loaded</p>
        )}
      </div>
    </section>
  );
}
