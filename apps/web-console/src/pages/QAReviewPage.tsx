import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  listFwaSchemes,
  listQaFeedbackItems,
  listQaQueue,
  listQaQueueSummary,
  submitQaResult,
  updateQaFeedbackStatus,
} from "../api";
import {
  buildFwaSchemeLabelMap,
  formatFwaSchemeLabel,
  type FwaSchemeDefinition,
} from "./fwaSchemeOptions";

type QaQueueItem = {
  qa_case_id: string;
  sample_id: string;
  lead_id: string;
  claim_id: string;
  scheme_family: string;
  rag: string;
  risk_score: number;
  reviewer: string;
  assignment_queue: string;
  status: string;
  qa_conclusion?: string | null;
  issue_type?: string | null;
  feedback_target?: string | null;
  evidence_refs: string[];
};

type QaFeedbackItem = {
  feedback_id: string;
  qa_case_id: string;
  claim_id: string;
  feedback_target: string;
  issue_type: string;
  priority: string;
  status: string;
  summary: string;
  note_present: boolean;
  evidence_refs: string[];
  status_updated_by?: string | null;
  status_audit_id?: string | null;
  status_updated_at?: string | null;
  status_evidence_refs?: string[];
};

type QaQueueSummary = {
  open_count: number;
  in_progress_count: number;
  resolved_count: number;
  dismissed_count: number;
  unresolved_count: number;
  rules_feedback_count: number;
  models_feedback_count: number;
  features_feedback_count: number;
  provider_profile_feedback_count: number;
  workflow_feedback_count: number;
  tpa_feedback_count: number;
  high_priority_count: number;
  evidence_backed_count: number;
  highest_priority: string;
};

type QaQueueListResponse = {
  items: QaQueueItem[];
};

export const QA_SUMMARY_FEEDBACK_ROWS = [
  { field: "rules_feedback_count", label: "Rules" },
  { field: "models_feedback_count", label: "Models" },
  { field: "features_feedback_count", label: "Features" },
  { field: "provider_profile_feedback_count", label: "Provider Profile" },
  { field: "workflow_feedback_count", label: "Workflow" },
  { field: "tpa_feedback_count", label: "TPA" },
] as const;

export const QA_CONCLUSION_OPTIONS = [
  { value: "pass", label: "Pass" },
  { value: "issue_found_return", label: "Issue Found - Return" },
  { value: "issue_found_escalate", label: "Issue Found - Escalate" },
] as const;

export const QA_ISSUE_TYPE_OPTIONS = [
  { value: "none", label: "None" },
  { value: "qa_review_completed", label: "QA Review Completed" },
  { value: "alert_handling_incomplete", label: "Alert Handling Incomplete" },
  { value: "medical_reasonableness", label: "Medical Reasonableness" },
  { value: "medical_necessity_issue", label: "Medical Necessity Issue" },
  { value: "provider_pattern", label: "Provider Pattern" },
  { value: "model_under_scored_confirmed_issue", label: "Model Under-Scored Confirmed Issue" },
  { value: "workflow_missing_evidence", label: "Workflow Missing Evidence" },
] as const;

export const QA_FEEDBACK_TARGET_OPTIONS = [
  { value: "rules", label: "Rules" },
  { value: "models", label: "Models" },
  { value: "features", label: "Features" },
  { value: "provider_profile", label: "Provider Profile" },
  { value: "workflow", label: "Workflow" },
  { value: "tpa", label: "TPA" },
] as const;

export function selectQaQueueItem(queue: QaQueueItem[], selectedCaseId: string) {
  return queue.find((item) => item.qa_case_id === selectedCaseId) ?? queue[0] ?? null;
}

export function canSubmitQaQueueItem(item: QaQueueItem | null) {
  return item?.status === "open";
}

export function canUpdateQaFeedbackItem(item: QaFeedbackItem) {
  return item.status === "open" || item.status === "in_progress";
}

export function buildQaEvidenceRefs(item: QaQueueItem | null) {
  if (!item) {
    return "";
  }
  return [
    `qa_queue:${item.qa_case_id}`,
    `audit_sample:${item.sample_id}`,
    `lead:${item.lead_id}`,
    ...item.evidence_refs,
  ]
    .filter((value, index, refs) => refs.indexOf(value) === index)
    .join("\n");
}

export function QAReviewPage() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const [selectedCaseId, setSelectedCaseId] = useState("");
  const queueQuery = useQuery({
    queryKey: ["qa-queue", apiKey],
    queryFn: () => listQaQueue(apiKey) as Promise<QaQueueListResponse>,
  });
  const selectedCase = useMemo(
    () => selectQaQueueItem(queueQuery.data?.items ?? [], selectedCaseId),
    [queueQuery.data?.items, selectedCaseId],
  );
  const [qaConclusion, setQaConclusion] = useState("issue_found_escalate");
  const [issueType, setIssueType] = useState("alert_handling_incomplete");
  const [feedbackTarget, setFeedbackTarget] = useState("rules");
  const [notes, setNotes] = useState("Reviewer should attach provider history evidence.");
  const [evidenceRefs, setEvidenceRefs] = useState(
    "audit:scoring.completed\nrule_runs:EARLY_CLAIM",
  );
  const queryClient = useQueryClient();
  const feedbackQuery = useQuery({
    queryKey: ["qa-feedback-items", apiKey],
    queryFn: () => listQaFeedbackItems(apiKey) as Promise<{ items: QaFeedbackItem[] }>,
  });
  const queueSummaryQuery = useQuery({
    queryKey: ["qa-queue-summary", apiKey],
    queryFn: () => listQaQueueSummary(apiKey) as Promise<QaQueueSummary>,
  });
  const schemesQuery = useQuery({
    queryKey: ["fwa-schemes", apiKey],
    queryFn: () => listFwaSchemes(apiKey) as Promise<{ schemes: FwaSchemeDefinition[] }>,
  });
  const schemeLabelMap = buildFwaSchemeLabelMap(schemesQuery.data?.schemes);

  useEffect(() => {
    setEvidenceRefs(buildQaEvidenceRefs(selectedCase));
  }, [selectedCase?.qa_case_id]);

  const submitMutation = useMutation({
    mutationFn: () => {
      if (!canSubmitQaQueueItem(selectedCase)) {
        throw new Error("No QA queue item selected");
      }
      return submitQaResult(
        {
          qa_case_id: selectedCase.qa_case_id,
          claim_id: selectedCase.claim_id,
          qa_conclusion: qaConclusion,
          issue_type: issueType,
          feedback_target: feedbackTarget,
          notes,
          evidence_refs: evidenceRefs
            .split(/\n|,/)
            .map((value) => value.trim())
            .filter(Boolean),
        },
        apiKey,
      );
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["qa-queue"] });
      queryClient.invalidateQueries({ queryKey: ["qa-feedback-items"] });
      queryClient.invalidateQueries({ queryKey: ["qa-queue-summary"] });
    },
  });
  const feedbackStatusMutation = useMutation({
    mutationFn: ({ item, status }: { item: QaFeedbackItem; status: string }) =>
      updateQaFeedbackStatus(
        item.feedback_id,
        {
          status,
          actor_id: "qa-ops",
          notes: `QA feedback marked ${status} from QA Review.`,
          evidence_refs: [`qa_feedback:${item.feedback_id}`, ...item.evidence_refs],
        },
        apiKey,
      ),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["qa-feedback-items"] });
      queryClient.invalidateQueries({ queryKey: ["qa-queue-summary"] });
    },
  });

  return (
    <section className="ops-grid">
      <div className="panel">
        <h2>QA Queue</h2>
        <label>
          API Key
          <input value={apiKey} onChange={(event) => setApiKey(event.target.value)} />
        </label>
        {queueQuery.error ? <pre className="error">{String(queueQuery.error.message)}</pre> : null}
        {schemesQuery.error ? (
          <pre className="error">{String(schemesQuery.error.message)}</pre>
        ) : null}
        <div className="table-list">
          {queueQuery.data?.items.map((item) => (
            <button
              className={
                item.qa_case_id === selectedCase?.qa_case_id ? "row-button active" : "row-button"
              }
              key={item.qa_case_id}
              onClick={() => setSelectedCaseId(item.qa_case_id)}
            >
              <span>{item.claim_id}</span>
              <strong>{item.rag}</strong>
              <small>{formatFwaSchemeLabel(item.scheme_family, schemeLabelMap)}</small>
              <small>{item.status}</small>
            </button>
          ))}
        </div>
        {queueQuery.data?.items.length === 0 ? <p className="empty">No QA queue items</p> : null}
        {queueSummaryQuery.error ? (
          <pre className="error">{String(queueSummaryQuery.error.message)}</pre>
        ) : null}
        {queueSummaryQuery.data ? (
          <dl className="result-grid">
            <div>
              <dt>Open Feedback</dt>
              <dd>{queueSummaryQuery.data.open_count}</dd>
            </div>
            <div>
              <dt>In Progress</dt>
              <dd>{queueSummaryQuery.data.in_progress_count}</dd>
            </div>
            <div>
              <dt>Resolved</dt>
              <dd>{queueSummaryQuery.data.resolved_count}</dd>
            </div>
            <div>
              <dt>Dismissed</dt>
              <dd>{queueSummaryQuery.data.dismissed_count}</dd>
            </div>
            <div>
              <dt>Unresolved</dt>
              <dd>{queueSummaryQuery.data.unresolved_count}</dd>
            </div>
            <div>
              <dt>High Priority</dt>
              <dd>{queueSummaryQuery.data.high_priority_count}</dd>
            </div>
            <div>
              <dt>Evidence Backed</dt>
              <dd>{queueSummaryQuery.data.evidence_backed_count}</dd>
            </div>
            <div>
              <dt>Highest Priority</dt>
              <dd>{queueSummaryQuery.data.highest_priority}</dd>
            </div>
            {QA_SUMMARY_FEEDBACK_ROWS.map((row) => (
              <div key={row.field}>
                <dt>{row.label}</dt>
                <dd>{queueSummaryQuery.data[row.field]}</dd>
              </div>
            ))}
          </dl>
        ) : null}
      </div>

      <div className="panel">
        <h2>Review Detail</h2>
        {selectedCase ? (
          <dl className="result-grid">
            <div>
              <dt>QA Case</dt>
              <dd>{selectedCase.qa_case_id}</dd>
            </div>
            <div>
              <dt>Claim</dt>
              <dd>{selectedCase.claim_id}</dd>
            </div>
            <div>
              <dt>Sample</dt>
              <dd>{selectedCase.sample_id}</dd>
            </div>
            <div>
              <dt>Lead</dt>
              <dd>{selectedCase.lead_id}</dd>
            </div>
            <div>
              <dt>Risk</dt>
              <dd>{selectedCase.risk_score}</dd>
            </div>
            <div>
              <dt>Scheme</dt>
              <dd>{formatFwaSchemeLabel(selectedCase.scheme_family, schemeLabelMap)}</dd>
            </div>
            <div>
              <dt>Reviewer</dt>
              <dd>{selectedCase.reviewer}</dd>
            </div>
            <div>
              <dt>Evidence</dt>
              <dd>{selectedCase.evidence_refs.length}</dd>
            </div>
            <div>
              <dt>Status</dt>
              <dd>{selectedCase.status}</dd>
            </div>
            <div>
              <dt>Conclusion</dt>
              <dd>{selectedCase.qa_conclusion ?? "pending"}</dd>
            </div>
          </dl>
        ) : (
          <p className="empty">No QA queue item selected</p>
        )}

        <div className="form-grid">
          <label>
            Conclusion
            <select value={qaConclusion} onChange={(event) => setQaConclusion(event.target.value)}>
              {QA_CONCLUSION_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
          <label>
            Issue Type
            <select value={issueType} onChange={(event) => setIssueType(event.target.value)}>
              {QA_ISSUE_TYPE_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
          <label>
            Feedback
            <select
              value={feedbackTarget}
              onChange={(event) => setFeedbackTarget(event.target.value)}
            >
              {QA_FEEDBACK_TARGET_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
        </div>

        <label>
          Notes
          <textarea value={notes} onChange={(event) => setNotes(event.target.value)} />
        </label>
        <label>
          Evidence Refs
          <textarea value={evidenceRefs} onChange={(event) => setEvidenceRefs(event.target.value)} />
        </label>
        <button
          disabled={!canSubmitQaQueueItem(selectedCase)}
          onClick={() => submitMutation.mutate()}
          type="button"
        >
          Submit QA Result
        </button>

        {submitMutation.error ? (
          <pre className="error">{String(submitMutation.error.message)}</pre>
        ) : null}
        {feedbackStatusMutation.error ? (
          <pre className="error">{String(feedbackStatusMutation.error.message)}</pre>
        ) : null}
        {submitMutation.data ? (
          <pre>{JSON.stringify(submitMutation.data, null, 2)}</pre>
        ) : null}
      </div>

      <div className="panel wide-panel">
        <h2>Feedback Items</h2>
        {feedbackQuery.error ? (
          <pre className="error">{String(feedbackQuery.error.message)}</pre>
        ) : null}
        <div className="table-list">
          {feedbackQuery.data?.items.map((item) => (
            <div className="metric-row compact-metric-row" key={item.feedback_id}>
              <span>{item.summary}</span>
              <strong>{item.feedback_target}</strong>
              <small>{item.issue_type}</small>
              <small>
                {item.priority} · {item.status}
              </small>
              {item.status_updated_by || item.status_audit_id ? (
                <small>
                  Updated by {item.status_updated_by ?? "unknown"}
                  {item.status_audit_id ? ` · ${item.status_audit_id}` : ""}
                </small>
              ) : null}
              {item.status_evidence_refs?.length ? (
                <small>{item.status_evidence_refs.join(", ")}</small>
              ) : null}
              {canUpdateQaFeedbackItem(item) ? (
                <div className="button-row">
                  <button
                    disabled={feedbackStatusMutation.isPending}
                    onClick={() =>
                      feedbackStatusMutation.mutate({ item, status: "in_progress" })
                    }
                    type="button"
                  >
                    In Progress
                  </button>
                  <button
                    disabled={feedbackStatusMutation.isPending}
                    onClick={() => feedbackStatusMutation.mutate({ item, status: "resolved" })}
                    type="button"
                  >
                    Resolve
                  </button>
                  <button
                    disabled={feedbackStatusMutation.isPending}
                    onClick={() => feedbackStatusMutation.mutate({ item, status: "dismissed" })}
                    type="button"
                  >
                    Dismiss
                  </button>
                </div>
              ) : null}
            </div>
          ))}
        </div>
        {feedbackQuery.data?.items.length === 0 ? (
          <p className="empty">No QA feedback items</p>
        ) : null}
      </div>
    </section>
  );
}
