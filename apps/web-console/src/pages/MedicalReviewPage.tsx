import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { listMedicalReviewQueue, submitMedicalReviewResult } from "../api";

type MedicalReviewQueueItem = {
  claim_id: string;
  run_id: string;
  audit_id: string;
  medical_reasonableness_score: number;
  review_route: string;
  evidence_status: string;
  missing_evidence: string[];
  item_finding_count: number;
  first_item_code?: string | null;
  first_issue_type?: string | null;
  evidence_refs: string[];
  created_at?: string | null;
  review_status: string;
  review_audit_id?: string | null;
  review_decision?: string | null;
  reviewer?: string | null;
  reviewed_at?: string | null;
};

type MedicalReviewQueueResponse = {
  items: MedicalReviewQueueItem[];
};

export function buildMedicalReviewQueueSummary(items: MedicalReviewQueueItem[]) {
  const highScoreCount = items.filter((item) => item.medical_reasonableness_score >= 80).length;
  const missingEvidenceCount = items.filter((item) => item.missing_evidence.length > 0).length;
  const evidenceBackedCount = items.filter((item) => item.evidence_refs.length > 0).length;
  const pendingEvidenceCount = items.filter(
    (item) => item.review_status === "pending_evidence",
  ).length;
  const completedCount = items.filter((item) => item.review_status.startsWith("completed")).length;
  return {
    queueCount: items.length,
    highScoreCount,
    missingEvidenceCount,
    evidenceBackedCount,
    pendingEvidenceCount,
    completedCount,
    topClaimId: items[0]?.claim_id ?? "none",
  };
}

export function buildMedicalReviewEvidenceRefs(item: MedicalReviewQueueItem | null) {
  if (!item) {
    return "";
  }
  return [`audit:${item.audit_id}`, ...item.evidence_refs]
    .filter((value, index, refs) => refs.indexOf(value) === index)
    .join("\n");
}

function selectedMedicalReviewItem(items: MedicalReviewQueueItem[], selectedAuditId: string) {
  return items.find((item) => item.audit_id === selectedAuditId) ?? items[0] ?? null;
}

export function MedicalReviewPage() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const [limit, setLimit] = useState(100);
  const [selectedAuditId, setSelectedAuditId] = useState("");
  const [reviewer, setReviewer] = useState("medical-reviewer-1");
  const [decision, setDecision] = useState("request_more_evidence");
  const [notes, setNotes] = useState("Medical record is required before necessity can be confirmed.");
  const [evidenceRefs, setEvidenceRefs] = useState("");
  const queryClient = useQueryClient();
  const queueQuery = useQuery({
    queryKey: ["medical-review-queue", apiKey, limit],
    queryFn: () => listMedicalReviewQueue(apiKey, limit) as Promise<MedicalReviewQueueResponse>,
  });
  const items = queueQuery.data?.items ?? [];
  const summary = buildMedicalReviewQueueSummary(items);
  const selectedItem = selectedMedicalReviewItem(items, selectedAuditId);
  const submitMutation = useMutation({
    mutationFn: () => {
      if (!selectedItem) {
        throw new Error("No medical review item selected");
      }
      return submitMedicalReviewResult(
        {
          claim_id: selectedItem.claim_id,
          scoring_audit_id: selectedItem.audit_id,
          reviewer,
          decision,
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
      queryClient.invalidateQueries({ queryKey: ["medical-review-queue"] });
    },
  });

  useEffect(() => {
    if (selectedItem && evidenceRefs.length === 0) {
      setEvidenceRefs(buildMedicalReviewEvidenceRefs(selectedItem));
    }
  }, [selectedItem?.audit_id, evidenceRefs.length]);

  return (
    <section className="ops-grid">
      <div className="panel dashboard-header">
        <div>
          <h2>Medical Review</h2>
          <p>Clinical evidence gaps and medical necessity review queue from scoring audit.</p>
        </div>
        <label>
          API Key
          <input value={apiKey} onChange={(event) => setApiKey(event.target.value)} />
        </label>
      </div>

      <div className="panel">
        <h2>Queue Summary</h2>
        {queueQuery.error ? <pre className="error">{String(queueQuery.error.message)}</pre> : null}
        <label>
          Limit
          <input
            min={1}
            max={200}
            type="number"
            value={limit}
            onChange={(event) => setLimit(Number(event.target.value))}
          />
        </label>
        <div className="summary-grid">
          <div>
            <span>Queue</span>
            <strong>{summary.queueCount}</strong>
          </div>
          <div>
            <span>High Score</span>
            <strong>{summary.highScoreCount}</strong>
          </div>
          <div>
            <span>Missing Evidence</span>
            <strong>{summary.missingEvidenceCount}</strong>
          </div>
          <div>
            <span>Evidence Backed</span>
            <strong>{summary.evidenceBackedCount}</strong>
          </div>
          <div>
            <span>Pending Evidence</span>
            <strong>{summary.pendingEvidenceCount}</strong>
          </div>
          <div>
            <span>Completed</span>
            <strong>{summary.completedCount}</strong>
          </div>
          <div>
            <span>Top Claim</span>
            <strong>{summary.topClaimId}</strong>
          </div>
        </div>
      </div>

      <div className="panel">
        <h2>Review Result</h2>
        {selectedItem ? (
          <dl className="result-grid">
            <div>
              <dt>Claim</dt>
              <dd>{selectedItem.claim_id}</dd>
            </div>
            <div>
              <dt>Scoring Audit</dt>
              <dd>{selectedItem.audit_id}</dd>
            </div>
            <div>
              <dt>Status</dt>
              <dd>{selectedItem.review_status}</dd>
            </div>
            <div>
              <dt>Latest Review</dt>
              <dd>{selectedItem.review_audit_id ?? "none"}</dd>
            </div>
          </dl>
        ) : (
          <p className="empty">No medical review item selected</p>
        )}
        <div className="form-grid">
          <label>
            Reviewer
            <input value={reviewer} onChange={(event) => setReviewer(event.target.value)} />
          </label>
          <label>
            Decision
            <select value={decision} onChange={(event) => setDecision(event.target.value)}>
              <option value="request_more_evidence">Request More Evidence</option>
              <option value="evidence_sufficient">Evidence Sufficient</option>
              <option value="medical_necessity_issue">Medical Necessity Issue</option>
              <option value="no_medical_issue">No Medical Issue</option>
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
          disabled={!selectedItem || submitMutation.isPending}
          onClick={() => submitMutation.mutate()}
          type="button"
        >
          Record Review
        </button>
        {submitMutation.error ? (
          <pre className="error">{String(submitMutation.error.message)}</pre>
        ) : null}
        {submitMutation.data ? <pre>{JSON.stringify(submitMutation.data, null, 2)}</pre> : null}
      </div>

      <div className="panel wide-panel">
        <h2>Review Queue</h2>
        <div className="table-list">
          {items.map((item) => (
            <button
              className={item.audit_id === selectedItem?.audit_id ? "row-button active" : "row-button"}
              key={item.audit_id}
              onClick={() => {
                setSelectedAuditId(item.audit_id);
                setEvidenceRefs(buildMedicalReviewEvidenceRefs(item));
              }}
              type="button"
            >
              <span>{item.claim_id}</span>
              <strong>{item.medical_reasonableness_score}</strong>
              <small>{item.review_route}</small>
              <small>{item.evidence_status}</small>
              <small>{item.review_status}</small>
              <small>{item.first_issue_type ?? "no issue type"}</small>
              <small>{item.first_item_code ?? "no item"}</small>
              <small>{item.missing_evidence.join(", ") || "no missing evidence"}</small>
              <small>{item.evidence_refs.join(", ") || "no evidence refs"}</small>
              <small>{item.audit_id}</small>
            </button>
          ))}
        </div>
        {!queueQuery.isLoading && items.length === 0 ? (
          <p className="empty">No medical review items</p>
        ) : null}
      </div>
    </section>
  );
}
