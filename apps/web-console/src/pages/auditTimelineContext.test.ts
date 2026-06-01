import { describe, expect, it } from "vitest";
import {
  buildAuditTimelineContext,
  buildGovernanceAuditFiltersFromContext,
  buildGovernanceClaimIdFromContext,
} from "./auditTimelineContext";

describe("audit timeline context", () => {
  it("builds claim audit navigation context from a scoring result", () => {
    expect(
      buildAuditTimelineContext({
        claim_id: "CLM-0287",
        audit_id: "audit_score_CLM-0287",
        run_id: "run_CLM-0287",
      }),
    ).toEqual({
      claimId: "CLM-0287",
      auditId: "audit_score_CLM-0287",
      runId: "run_CLM-0287",
      source: "runtime_scoring",
    });
  });

  it("derives Governance claim and global audit filters from scoring context", () => {
    const context = {
      claimId: "CLM-0287",
      auditId: "audit_score_CLM-0287",
      runId: "run_CLM-0287",
      source: "runtime_scoring" as const,
    };

    expect(buildGovernanceClaimIdFromContext(context, "CLM-DEFAULT")).toBe("CLM-0287");
    expect(buildGovernanceClaimIdFromContext(undefined, "CLM-DEFAULT")).toBe("CLM-DEFAULT");
    expect(buildGovernanceAuditFiltersFromContext(context)).toMatchObject({
      eventType: "scoring.completed",
      runId: "run_CLM-0287",
      claimId: "CLM-0287",
      limit: "50",
    });
  });
});
