export type SavingAttributionSummary = {
  source_type: string;
  source_id: string;
  action: string;
  saving_amount: string;
  currency: string;
  claim_count: number;
  evidence_refs: string[];
};

export type SavingAttributionRow = {
  key: string;
  sourceKeyLabel: string;
  sourceLabel: string;
  lineageLabel: string;
  lineageStatus: string;
  action: string;
  savingAmount: string;
  currency: string;
  claimCount: number;
  averageSavingPerClaim: string;
  evidenceRefs: string[];
};

const sourceOrder: Record<string, number> = {
  agent: 0,
  rule: 1,
  model: 2,
};

const lineageEvidencePrefixes: Record<string, string[]> = {
  agent: ["agent_run:"],
  rule: ["rule_runs:", "rules:"],
  model: ["model_scores:", "model_versions:"],
};

const sourceKeyLabels: Record<string, string> = {
  agent: "agent_run_id",
  rule: "rule_id",
  model: "model_id",
};

function sourceKeyLabel(sourceType: string) {
  return sourceKeyLabels[sourceType] ?? `${sourceType}_id`;
}

function attributionHasLineageEvidence(attribution: SavingAttributionSummary) {
  const prefixes = lineageEvidencePrefixes[attribution.source_type] ?? [];
  return prefixes.some((prefix) =>
    attribution.evidence_refs.some((reference) => reference.startsWith(prefix)),
  );
}

export function buildSavingAttributionRows(
  attributions: SavingAttributionSummary[],
): SavingAttributionRow[] {
  return [...attributions]
    .sort((left, right) => {
      const leftOrder = sourceOrder[left.source_type] ?? 99;
      const rightOrder = sourceOrder[right.source_type] ?? 99;
      return (
        leftOrder - rightOrder ||
        left.source_id.localeCompare(right.source_id) ||
        left.action.localeCompare(right.action)
      );
    })
    .map((attribution) => {
      const keyLabel = sourceKeyLabel(attribution.source_type);
      return {
        key: `${attribution.source_type}:${attribution.source_id}:${attribution.action}`,
        sourceKeyLabel: keyLabel,
        sourceLabel: `${keyLabel} / ${attribution.source_id}`,
        lineageLabel: `${keyLabel}=${attribution.source_id} -> action=${attribution.action} -> saving=${attribution.currency} ${attribution.saving_amount}`,
        lineageStatus: attributionHasLineageEvidence(attribution)
          ? "lineage_evidence_present"
          : "lineage_evidence_missing",
        action: attribution.action,
        savingAmount: attribution.saving_amount,
        currency: attribution.currency,
        claimCount: attribution.claim_count,
        averageSavingPerClaim:
          attribution.claim_count === 0
            ? "0.00"
            : (Number(attribution.saving_amount) / attribution.claim_count).toFixed(2),
        evidenceRefs: [...attribution.evidence_refs].sort(),
      };
    });
}
