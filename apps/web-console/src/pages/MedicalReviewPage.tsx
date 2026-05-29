import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { listMedicalReviewQueue } from "../api";

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
};

type MedicalReviewQueueResponse = {
  items: MedicalReviewQueueItem[];
};

export function buildMedicalReviewQueueSummary(items: MedicalReviewQueueItem[]) {
  const highScoreCount = items.filter((item) => item.medical_reasonableness_score >= 80).length;
  const missingEvidenceCount = items.filter((item) => item.missing_evidence.length > 0).length;
  const evidenceBackedCount = items.filter((item) => item.evidence_refs.length > 0).length;
  return {
    queueCount: items.length,
    highScoreCount,
    missingEvidenceCount,
    evidenceBackedCount,
    topClaimId: items[0]?.claim_id ?? "none",
  };
}

export function MedicalReviewPage() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const [limit, setLimit] = useState(100);
  const queueQuery = useQuery({
    queryKey: ["medical-review-queue", apiKey, limit],
    queryFn: () => listMedicalReviewQueue(apiKey, limit) as Promise<MedicalReviewQueueResponse>,
  });
  const items = queueQuery.data?.items ?? [];
  const summary = buildMedicalReviewQueueSummary(items);

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
            <span>Top Claim</span>
            <strong>{summary.topClaimId}</strong>
          </div>
        </div>
      </div>

      <div className="panel wide-panel">
        <h2>Review Queue</h2>
        <div className="table-list">
          {items.map((item) => (
            <div className="metric-row compact-metric-row" key={item.audit_id}>
              <span>{item.claim_id}</span>
              <strong>{item.medical_reasonableness_score}</strong>
              <small>{item.review_route}</small>
              <small>{item.evidence_status}</small>
              <small>{item.first_issue_type ?? "no issue type"}</small>
              <small>{item.first_item_code ?? "no item"}</small>
              <small>{item.missing_evidence.join(", ") || "no missing evidence"}</small>
              <small>{item.evidence_refs.join(", ") || "no evidence refs"}</small>
              <small>{item.audit_id}</small>
            </div>
          ))}
        </div>
        {!queueQuery.isLoading && items.length === 0 ? (
          <p className="empty">No medical review items</p>
        ) : null}
      </div>
    </section>
  );
}
