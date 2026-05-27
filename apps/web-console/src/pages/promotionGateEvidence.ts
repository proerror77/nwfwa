export type PromotionGate = {
  label: string;
  passed: boolean;
  blocker: string;
  evidence_source: string;
};

export type PromotionGateEvidenceRow = {
  label: string;
  status: string;
  evidenceSource: string;
  evidenceClassName: string;
};

const evidenceSourceLabels: Record<string, string> = {
  runtime: "Runtime",
  backtest: "Backtest",
  approval: "Approval",
  evaluation: "Evaluation",
  metadata: "Metadata",
  missing: "Missing",
};

export function buildPromotionGateEvidenceRows(
  gates: PromotionGate[],
): PromotionGateEvidenceRow[] {
  return gates.map((gate) => {
    const evidenceSource = gate.evidence_source || "missing";
    return {
      label: gate.label,
      status: gate.passed ? "passed" : gate.blocker,
      evidenceSource: evidenceSourceLabels[evidenceSource] ?? evidenceSource,
      evidenceClassName: `source-${evidenceSource}`,
    };
  });
}
