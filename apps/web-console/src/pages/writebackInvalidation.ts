import type { QueryClient } from "@tanstack/react-query";

export type WritebackInvalidationKind =
  | "qa_result"
  | "qa_feedback_status"
  | "medical_review"
  | "investigation_result"
  | "case_status";

export function writebackInvalidationKeys(kind: WritebackInvalidationKind): string[][] {
  switch (kind) {
    case "qa_result":
      return [
        ["qa-queue"],
        ["qa-feedback-items"],
        ["qa-queue-summary"],
        ["dashboard-summary"],
        ["outcome-labels"],
        ["claim-audit-history"],
        ["global-audit-events"],
        ["api-calls"],
        ["ops-alerts"],
        ["webhook-events"],
      ];
    case "qa_feedback_status":
      return [
        ["qa-feedback-items"],
        ["qa-queue-summary"],
        ["dashboard-summary"],
        ["outcome-labels"],
        ["global-audit-events"],
      ];
    case "medical_review":
      return [
        ["medical-review-queue"],
        ["dashboard-summary"],
        ["outcome-labels"],
        ["claim-audit-history"],
        ["global-audit-events"],
        ["ops-alerts"],
        ["webhook-events"],
      ];
    case "investigation_result":
      return [
        ["cases"],
        ["dashboard-summary"],
        ["outcome-labels"],
        ["claim-audit-history"],
        ["global-audit-events"],
        ["api-calls"],
        ["ops-alerts"],
        ["webhook-events"],
      ];
    case "case_status":
      return [
        ["cases"],
        ["dashboard-summary"],
        ["outcome-labels"],
        ["claim-audit-history"],
        ["global-audit-events"],
        ["ops-alerts"],
      ];
  }
}

export function invalidateWritebackQueries(
  queryClient: QueryClient,
  kind: WritebackInvalidationKind,
) {
  for (const queryKey of writebackInvalidationKeys(kind)) {
    queryClient.invalidateQueries({ queryKey });
  }
}
