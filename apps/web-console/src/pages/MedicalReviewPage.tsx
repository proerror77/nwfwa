import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { listMedicalReviewQueue, submitMedicalReviewResult } from "../api";
import { invalidateWritebackQueries } from "./writebackInvalidation";

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

type MedicalReviewResultResponse = {
  claim_id: string;
  event_type: string;
  event_status: string;
  audit_id: string;
  run_id: string;
  review_status: string;
  evidence_refs: string[];
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

function countBy(values: string[]) {
  return values.reduce<Record<string, number>>((counts, value) => {
    counts[value] = (counts[value] ?? 0) + 1;
    return counts;
  }, {});
}

function topCount(counts: Record<string, number>) {
  return (
    Object.entries(counts).sort(
      ([leftValue, leftCount], [rightValue, rightCount]) =>
        rightCount - leftCount || leftValue.localeCompare(rightValue),
    )[0] ?? ["none", 0]
  );
}

export function buildMedicalReviewClinicalSignalSummary(items: MedicalReviewQueueItem[]) {
  const issueCounts = countBy(items.flatMap((item) => item.first_issue_type ?? []));
  const missingEvidenceCounts = countBy(items.flatMap((item) => item.missing_evidence));
  const [topMissingEvidence, topMissingEvidenceCount] = topCount(missingEvidenceCounts);
  return {
    medicalNecessityIssueCount: issueCounts.medical_necessity_review_required ?? 0,
    drugReasonablenessIssueCount: issueCounts.drug_reasonableness_review_required ?? 0,
    labEvidenceIssueCount: issueCounts.lab_evidence_review_required ?? 0,
    clinicalOrderMissingCount: missingEvidenceCounts.clinical_order ?? 0,
    medicalRecordMissingCount: missingEvidenceCounts.medical_record ?? 0,
    topMissingEvidence,
    topMissingEvidenceCount,
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

export function buildMedicalReviewDecisionSummary(item: MedicalReviewQueueItem | null) {
  return {
    decision: item?.review_decision ?? "pending",
    reviewer: item?.reviewer ?? "unassigned",
    reviewedAt: item?.reviewed_at ?? "not reviewed",
  };
}

export function buildSelectedMedicalReviewSignal(item: MedicalReviewQueueItem | null) {
  return {
    medicalScore: item?.medical_reasonableness_score ?? "none",
    evidenceStatus: item?.evidence_status ?? "none",
    issueType: item?.first_issue_type ?? "none",
    itemCode: item?.first_item_code ?? "none",
    itemFindingCount: item?.item_finding_count ?? 0,
    missingEvidence: item?.missing_evidence.join(", ") || "none",
    evidenceRefCount: item?.evidence_refs.length ?? 0,
  };
}

export function buildMedicalReviewSubmitSummary(response?: MedicalReviewResultResponse | null) {
  if (!response) {
    return null;
  }
  return {
    claimId: response.claim_id,
    eventType: response.event_type,
    eventStatus: response.event_status,
    auditId: response.audit_id,
    runId: response.run_id,
    reviewStatus: response.review_status,
    evidenceCount: response.evidence_refs.length,
    evidenceRefs: response.evidence_refs,
  };
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
  const clinicalSignals = buildMedicalReviewClinicalSignalSummary(items);
  const selectedItem = selectedMedicalReviewItem(items, selectedAuditId);
  const selectedSignal = buildSelectedMedicalReviewSignal(selectedItem);
  const decisionSummary = buildMedicalReviewDecisionSummary(selectedItem);
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
      ) as Promise<MedicalReviewResultResponse>;
    },
    onSuccess: () => {
      invalidateWritebackQueries(queryClient, "medical_review");
    },
  });
  const submitSummary = buildMedicalReviewSubmitSummary(submitMutation.data);

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
            <div>
              <dt>Decision</dt>
              <dd>{decisionSummary.decision}</dd>
            </div>
            <div>
              <dt>Reviewer</dt>
              <dd>{decisionSummary.reviewer}</dd>
            </div>
            <div>
              <dt>Reviewed At</dt>
              <dd>{decisionSummary.reviewedAt}</dd>
            </div>
            <div>
              <dt>Medical Score</dt>
              <dd>{selectedSignal.medicalScore}</dd>
            </div>
            <div>
              <dt>Evidence Status</dt>
              <dd>{selectedSignal.evidenceStatus}</dd>
            </div>
            <div>
              <dt>Issue Type</dt>
              <dd>{selectedSignal.issueType}</dd>
            </div>
            <div>
              <dt>Item Code</dt>
              <dd>{selectedSignal.itemCode}</dd>
            </div>
            <div>
              <dt>Item Findings</dt>
              <dd>{selectedSignal.itemFindingCount}</dd>
            </div>
            <div>
              <dt>Missing Evidence</dt>
              <dd>{selectedSignal.missingEvidence}</dd>
            </div>
            <div>
              <dt>Evidence Ref Count</dt>
              <dd>{selectedSignal.evidenceRefCount}</dd>
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
        {submitSummary ? (
          <>
            <dl className="result-grid">
              <div>
                <dt>Review Claim</dt>
                <dd>{submitSummary.claimId}</dd>
              </div>
              <div>
                <dt>Event Type</dt>
                <dd>{submitSummary.eventType}</dd>
              </div>
              <div>
                <dt>Event Status</dt>
                <dd>{submitSummary.eventStatus}</dd>
              </div>
              <div>
                <dt>Review Status</dt>
                <dd>{submitSummary.reviewStatus}</dd>
              </div>
              <div>
                <dt>Audit ID</dt>
                <dd>{submitSummary.auditId}</dd>
              </div>
              <div>
                <dt>Run ID</dt>
                <dd>{submitSummary.runId}</dd>
              </div>
              <div>
                <dt>Evidence Refs</dt>
                <dd>{submitSummary.evidenceCount}</dd>
              </div>
            </dl>
            {submitSummary.evidenceRefs.length > 0 ? (
              <ol className="audit-timeline">
                {submitSummary.evidenceRefs.map((reference) => (
                  <li key={reference}>
                    <span>{reference}</span>
                  </li>
                ))}
              </ol>
            ) : null}
          </>
        ) : null}
      </div>

      <div className="panel">
        <h2>Clinical Signals</h2>
        <div className="summary-grid">
          <div>
            <span>Medical Necessity</span>
            <strong>{clinicalSignals.medicalNecessityIssueCount}</strong>
          </div>
          <div>
            <span>Drug Reasonableness</span>
            <strong>{clinicalSignals.drugReasonablenessIssueCount}</strong>
          </div>
          <div>
            <span>Lab Evidence</span>
            <strong>{clinicalSignals.labEvidenceIssueCount}</strong>
          </div>
          <div>
            <span>Missing Orders</span>
            <strong>{clinicalSignals.clinicalOrderMissingCount}</strong>
          </div>
          <div>
            <span>Missing Records</span>
            <strong>{clinicalSignals.medicalRecordMissingCount}</strong>
          </div>
          <div>
            <span>Top Gap</span>
            <strong>
              {clinicalSignals.topMissingEvidence} ({clinicalSignals.topMissingEvidenceCount})
            </strong>
          </div>
        </div>
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
