import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  listFwaSchemes,
  listQaFeedbackItems,
  listQaQueue,
  listQaQueueSummary,
  submitQaResult,
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
};

type QaQueueSummary = {
  open_count: number;
  rules_feedback_count: number;
  models_feedback_count: number;
  tpa_feedback_count: number;
  high_priority_count: number;
  evidence_backed_count: number;
  highest_priority: string;
};

type QaQueueListResponse = {
  items: QaQueueItem[];
};

export function selectQaQueueItem(queue: QaQueueItem[], selectedCaseId: string) {
  return queue.find((item) => item.qa_case_id === selectedCaseId) ?? queue[0] ?? null;
}

export function canSubmitQaQueueItem(item: QaQueueItem | null) {
  return item?.status === "open";
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
            <div>
              <dt>Rules</dt>
              <dd>{queueSummaryQuery.data.rules_feedback_count}</dd>
            </div>
            <div>
              <dt>Models</dt>
              <dd>{queueSummaryQuery.data.models_feedback_count}</dd>
            </div>
            <div>
              <dt>TPA</dt>
              <dd>{queueSummaryQuery.data.tpa_feedback_count}</dd>
            </div>
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
              <option value="pass">Pass</option>
              <option value="issue_found_return">Issue Found - Return</option>
              <option value="issue_found_escalate">Issue Found - Escalate</option>
            </select>
          </label>
          <label>
            Issue Type
            <select value={issueType} onChange={(event) => setIssueType(event.target.value)}>
              <option value="alert_handling_incomplete">Alert Handling Incomplete</option>
              <option value="medical_reasonableness">Medical Reasonableness</option>
              <option value="provider_pattern">Provider Pattern</option>
            </select>
          </label>
          <label>
            Feedback
            <select value={feedbackTarget} onChange={(event) => setFeedbackTarget(event.target.value)}>
              <option value="rules">Rules</option>
              <option value="models">Models</option>
              <option value="tpa">TPA</option>
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
