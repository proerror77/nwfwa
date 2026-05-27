export type PromotionModelVersion = {
  model_key: string;
  version: string;
  status: string;
};

export type PromotionModelPerformance = {
  data_status: string;
  scored_runs: number;
};

export type PromotionModelEvaluation = {
  evaluation_run_id: string;
  model_key: string;
  model_version: string;
  model_dataset_id: string;
  auc: string | null;
  precision: string | null;
  recall: string | null;
  threshold: string | null;
  feature_importance_uri?: string | null;
  metrics_json?: Record<string, unknown> | null;
};

type Gate = {
  label: string;
  passed: boolean;
  blocker: string;
};

export function buildModelPromotionGateSummary(
  model: PromotionModelVersion | undefined,
  performance: PromotionModelPerformance | undefined,
  evaluations: PromotionModelEvaluation[],
) {
  const latestEvaluation = model
    ? evaluations.find(
        (evaluation) =>
          evaluation.model_key === model.model_key && evaluation.model_version === model.version,
      )
    : undefined;
  const metrics = latestEvaluation?.metrics_json ?? {};
  const hasOutOfTimeMetric =
    metrics.out_of_time_auc !== undefined ||
    metrics.out_of_time_precision !== undefined ||
    metrics.out_of_time_recall !== undefined;

  const gates: Gate[] = [
    {
      label: "Immutable dataset",
      passed: Boolean(latestEvaluation?.model_dataset_id),
      blocker: "dataset version missing",
    },
    {
      label: "Holdout metrics",
      passed: Boolean(latestEvaluation?.auc && latestEvaluation.precision && latestEvaluation.recall),
      blocker: "holdout metrics missing",
    },
    {
      label: "Out-of-time evidence",
      passed: hasOutOfTimeMetric,
      blocker: "out-of-time metrics missing",
    },
    {
      label: "Review-capacity threshold",
      passed: Boolean(
        latestEvaluation?.threshold &&
          metrics.review_capacity_threshold_status === "passed",
      ),
      blocker: "review-capacity threshold missing",
    },
    {
      label: "Explanation artifact",
      passed: Boolean(latestEvaluation?.feature_importance_uri),
      blocker: "feature importance missing",
    },
    {
      label: "Leakage check",
      passed: metrics.leakage_check_status === "passed",
      blocker: "leakage check missing",
    },
    {
      label: "Shadow comparison",
      passed: metrics.shadow_comparison_status === "passed",
      blocker: "shadow comparison missing",
    },
    {
      label: "Approval",
      passed: metrics.approval_status === "approved",
      blocker: "approval missing",
    },
    {
      label: "Active version",
      passed: model?.status === "active",
      blocker: "model is not active",
    },
  ];

  const blockers = gates.filter((gate) => !gate.passed).map((gate) => gate.blocker);

  return {
    decision: blockers.length === 0 ? "routing_allowed" : "routing_blocked",
    passedCount: gates.length - blockers.length,
    totalCount: gates.length,
    latestEvaluationId: latestEvaluation?.evaluation_run_id ?? "none",
    dataStatus: performance?.data_status ?? "unknown",
    scoredRuns: performance?.scored_runs ?? 0,
    gates,
    blockers,
  };
}
