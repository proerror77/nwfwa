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
  sourceLabel: string;
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
    .map((attribution) => ({
      key: `${attribution.source_type}:${attribution.source_id}:${attribution.action}`,
      sourceLabel: `${attribution.source_type} / ${attribution.source_id}`,
      action: attribution.action,
      savingAmount: attribution.saving_amount,
      currency: attribution.currency,
      claimCount: attribution.claim_count,
      averageSavingPerClaim:
        attribution.claim_count === 0
          ? "0.00"
          : (Number(attribution.saving_amount) / attribution.claim_count).toFixed(2),
      evidenceRefs: [...attribution.evidence_refs].sort(),
    }));
}
