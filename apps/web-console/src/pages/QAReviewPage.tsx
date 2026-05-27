import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { listQaFeedbackItems, submitQaResult } from "../api";

type QaQueueItem = {
  qa_case_id: string;
  claim_id: string;
  rag: string;
  risk_score: number;
  alert: string;
  amount: string;
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

const demoQueue: QaQueueItem[] = [
  {
    qa_case_id: "QA-9001",
    claim_id: "CLM-0287",
    rag: "Red",
    risk_score: 87,
    alert: "EARLY_CLAIM",
    amount: "CNY 8000",
  },
  {
    qa_case_id: "QA-9100",
    claim_id: "CLM-9100",
    rag: "Red",
    risk_score: 82,
    alert: "HIGH_AMOUNT_TO_LIMIT",
    amount: "CNY 12600",
  },
];

export function QAReviewPage() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const [selectedCaseId, setSelectedCaseId] = useState(demoQueue[0].qa_case_id);
  const selectedCase = useMemo(
    () => demoQueue.find((item) => item.qa_case_id === selectedCaseId) ?? demoQueue[0],
    [selectedCaseId],
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

  const submitMutation = useMutation({
    mutationFn: () =>
      submitQaResult(
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
      ),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["qa-feedback-items"] });
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
        <div className="table-list">
          {demoQueue.map((item) => (
            <button
              className={item.qa_case_id === selectedCase.qa_case_id ? "row-button active" : "row-button"}
              key={item.qa_case_id}
              onClick={() => setSelectedCaseId(item.qa_case_id)}
            >
              <span>{item.claim_id}</span>
              <strong>{item.rag}</strong>
              <small>{item.alert}</small>
              <small>{item.amount}</small>
            </button>
          ))}
        </div>
      </div>

      <div className="panel">
        <h2>Review Detail</h2>
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
            <dt>Risk</dt>
            <dd>{selectedCase.risk_score}</dd>
          </div>
          <div>
            <dt>Alert</dt>
            <dd>{selectedCase.alert}</dd>
          </div>
        </dl>

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
        <button onClick={() => submitMutation.mutate()} type="button">
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
