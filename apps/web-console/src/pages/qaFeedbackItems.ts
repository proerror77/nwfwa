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

export function filterQaFeedbackItems(items: QaFeedbackItem[], target: string) {
  return items.filter((item) => item.feedback_target === target);
}

export function summarizeQaFeedbackItems(items: QaFeedbackItem[]) {
  return {
    openCount: items.filter((item) => item.status === "open").length,
    highestPriority: highestPriority(items),
    evidenceBackedCount: items.filter((item) => item.evidence_refs.length > 0).length,
  };
}

function highestPriority(items: QaFeedbackItem[]) {
  const priorities = ["critical", "high", "medium", "low"];
  return priorities.find((priority) => items.some((item) => item.priority === priority)) ?? "none";
}
