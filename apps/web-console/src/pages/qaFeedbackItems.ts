export type QaFeedbackItem = {
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

export function canonicalFeedbackTarget(target: string) {
  return target === "models" ? "model" : target;
}

export function filterQaFeedbackItems(items: QaFeedbackItem[], target: string) {
  const canonicalTarget = canonicalFeedbackTarget(target);
  return items.filter((item) => canonicalFeedbackTarget(item.feedback_target) === canonicalTarget);
}

export function summarizeQaFeedbackItems(items: QaFeedbackItem[]) {
  return {
    openCount: items.filter((item) => item.status === "open").length,
    highestPriority: highestPriority(items),
    evidenceBackedCount: items.filter((item) => item.evidence_refs.length > 0).length,
  };
}

export function buildQaFeedbackStatusAuditLabel(item: QaFeedbackItem) {
  if (!item.status_updated_by && !item.status_audit_id && !item.status_updated_at) {
    return null;
  }
  return [
    `Updated by ${item.status_updated_by ?? "unknown"}`,
    item.status_audit_id,
    item.status_updated_at,
  ]
    .filter(Boolean)
    .join(" · ");
}

export function buildQaFeedbackStatusEvidenceLabel(item: QaFeedbackItem) {
  if (!item.status_evidence_refs?.length) {
    return null;
  }
  return item.status_evidence_refs.join(", ");
}

function highestPriority(items: QaFeedbackItem[]) {
  const priorities = ["critical", "high", "medium", "low"];
  return priorities.find((priority) => items.some((item) => item.priority === priority)) ?? "none";
}
