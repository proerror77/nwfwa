import { describe, expect, it } from "vitest";
import { writebackInvalidationKeys } from "./writebackInvalidation";

describe("writebackInvalidationKeys", () => {
  it("invalidates QA result writeback surfaces across QA Governance and operations", () => {
    expect(writebackInvalidationKeys("qa_result")).toEqual([
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
    ]);
  });

  it("invalidates medical review and investigation writeback surfaces without refreshing unrelated pages", () => {
    expect(writebackInvalidationKeys("medical_review")).toEqual([
      ["medical-review-queue"],
      ["dashboard-summary"],
      ["outcome-labels"],
      ["claim-audit-history"],
      ["global-audit-events"],
      ["ops-alerts"],
      ["webhook-events"],
    ]);
    expect(writebackInvalidationKeys("investigation_result")).toEqual([
      ["cases"],
      ["dashboard-summary"],
      ["outcome-labels"],
      ["claim-audit-history"],
      ["global-audit-events"],
      ["api-calls"],
      ["ops-alerts"],
      ["webhook-events"],
    ]);
  });
});
