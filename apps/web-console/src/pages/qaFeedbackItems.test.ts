import { describe, expect, it } from "vitest";
import { filterQaFeedbackItems, summarizeQaFeedbackItems } from "./qaFeedbackItems";

const items = [
  {
    feedback_id: "qa_feedback_QA-RULE-1",
    qa_case_id: "QA-RULE-1",
    claim_id: "CLM-1",
    feedback_target: "rules",
    issue_type: "alert_handling_incomplete",
    priority: "high",
    status: "open",
    summary: "Rule feedback",
    note_present: true,
    evidence_refs: ["rule_runs:EARLY_CLAIM"],
  },
  {
    feedback_id: "qa_feedback_QA-MODEL-1",
    qa_case_id: "QA-MODEL-1",
    claim_id: "CLM-2",
    feedback_target: "models",
    issue_type: "model_under_scored_confirmed_issue",
    priority: "medium",
    status: "open",
    summary: "Model feedback",
    note_present: false,
    evidence_refs: ["model_scores:baseline_fwa"],
  },
];

describe("QA feedback item helpers", () => {
  it("filters feedback by operations target", () => {
    expect(filterQaFeedbackItems(items, "rules")).toEqual([items[0]]);
    expect(filterQaFeedbackItems(items, "models")).toEqual([items[1]]);
  });

  it("summarizes open count and highest priority", () => {
    expect(summarizeQaFeedbackItems(items)).toEqual({
      openCount: 2,
      highestPriority: "high",
      evidenceBackedCount: 2,
    });
  });
});
