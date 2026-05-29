import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  approveRule,
  backtestRule,
  discoverRules,
  getRule,
  getRulePromotionGates,
  listAuditEvents,
  listFwaSchemes,
  listOutcomeLabels,
  listQaFeedbackItems,
  listRules,
  publishRule,
  rollbackRule,
  saveRuleCandidate,
  submitRule,
  submitRulePromotionReview,
} from "../api";
import {
  buildQaFeedbackStatusAuditLabel,
  buildQaFeedbackStatusEvidenceLabel,
  filterQaFeedbackItems,
  QaFeedbackItem,
  summarizeQaFeedbackItems,
} from "./qaFeedbackItems";
import {
  buildRuleBacktestSummary,
  type RuleBacktestResponse,
} from "./ruleBacktestSummary";
import {
  buildPromotionGateEvidenceRows,
  type PromotionGate,
} from "./promotionGateEvidence";
import { formatReviewModeLabel } from "./reviewMode";
import {
  buildFwaSchemeLabelMap,
  formatFwaSchemeLabel,
  type FwaSchemeDefinition,
} from "./fwaSchemeOptions";

type RuleSummary = {
  rule_id: string;
  name: string;
  status: string;
  owner: string;
  active_version: number | null;
  latest_version: number;
  review_mode: string;
  scheme_family: string;
  score: number;
  alert_code: string;
  recommended_action: string;
};

type RulePromotionGatesResponse = {
  review_mode: string;
  decision: string;
  passed_count: number;
  total_count: number;
  trigger_count: number;
  reviewed_count: number;
  false_positive_rate: number;
  saving_amount: string;
  open_rule_feedback_count: number;
  unresolved_rule_feedback_count: number;
  approved_label_count: number;
  needs_review_label_count: number;
  blockers: string[];
  gates: PromotionGate[];
};

type OutcomeLabel = {
  label_id: string;
  claim_id: string;
  label_name: string;
  label_value: string;
  source_type: string;
  source_id: string;
  governance_status: string;
  feedback_target: string;
  currency?: string | null;
  evidence_refs: string[];
};

type AuditEvent = {
  audit_id: string;
  run_id: string;
  event_type: string;
  event_status: string;
  summary: string;
  evidence_refs: string[];
  created_at?: string | null;
};

type RuleDiscoveryCandidate = {
  rule: {
    rule_id: string;
    name?: string;
  } & Record<string, unknown>;
  support: number;
  precision: number;
  recall: number;
  lift: number;
  estimated_saving: string;
  false_positive_rate: number;
  matched_claim_ids: string[];
  explanation: string;
};

type RuleDiscoveryResponse = {
  sample_count: number;
  positive_count: number;
  candidates: RuleDiscoveryCandidate[];
};

type RuleCandidateSaveResponse = {
  summary: RuleSummary;
  versions: Array<{
    version: number;
    status: string;
    review_mode: string;
    scheme_family: string;
  }>;
  audit_events: AuditEvent[];
};

type RuleVersion = {
  version: number;
  status: string;
  dsl: Record<string, unknown>;
  review_mode: string;
  scheme_family: string;
  score: number;
  alert_code: string;
  recommended_action: string;
  reason: string;
};

type RuleDetailResponse = {
  summary: RuleSummary;
  versions: RuleVersion[];
  audit_events: AuditEvent[];
};

export function buildRuleLabelReadinessSummary(labels: OutcomeLabel[] = []) {
  const ruleLabels = labels.filter((label) => label.feedback_target === "rules");
  return {
    ruleLabelCount: ruleLabels.length,
    approvedForTrainingCount: ruleLabels.filter(
      (label) => label.governance_status === "approved_for_training",
    ).length,
    needsReviewCount: ruleLabels.filter((label) => label.governance_status === "needs_review")
      .length,
    evidenceBackedCount: ruleLabels.filter((label) => label.evidence_refs.length > 0).length,
    confirmedFwaCount: ruleLabels.filter(
      (label) => label.label_name === "confirmed_fwa" && label.label_value === "true",
    ).length,
  };
}

export function buildRuleDiscoverySummary(discovery?: RuleDiscoveryResponse) {
  const candidates = discovery?.candidates ?? [];
  const topCandidate = candidates[0];
  return {
    sampleCount: discovery?.sample_count ?? 0,
    positiveCount: discovery?.positive_count ?? 0,
    candidateCount: candidates.length,
    topRuleId: topCandidate?.rule.rule_id ?? "none",
    topPrecisionLabel: topCandidate ? `${(topCandidate.precision * 100).toFixed(1)}%` : "0.0%",
    topLiftLabel: topCandidate ? `${topCandidate.lift.toFixed(2)}x` : "0.00x",
    topSaving: topCandidate?.estimated_saving ?? "0.00",
  };
}

export function buildRuleDetailSummary(detail?: RuleDetailResponse | null) {
  if (!detail) {
    return null;
  }
  const latestVersion =
    detail.versions.find((version) => version.version === detail.summary.latest_version) ??
    detail.versions[0];
  const conditions = latestVersion?.dsl.conditions;
  return {
    ruleId: detail.summary.rule_id,
    name: detail.summary.name,
    status: detail.summary.status,
    owner: detail.summary.owner,
    activeVersionLabel: detail.summary.active_version ? `v${detail.summary.active_version}` : "none",
    latestVersionLabel: `v${detail.summary.latest_version}`,
    versionCount: detail.versions.length,
    auditEventCount: detail.audit_events.length,
    latestStatus: latestVersion?.status ?? "not_available",
    latestReviewMode: latestVersion?.review_mode ?? "not_available",
    latestSchemeFamily: latestVersion?.scheme_family ?? "not_available",
    latestScore: latestVersion?.score ?? 0,
    latestAlertCode: latestVersion?.alert_code ?? "not_available",
    latestAction: latestVersion?.recommended_action ?? "not_available",
    latestReason: latestVersion?.reason ?? "not_available",
    latestConditionCount: Array.isArray(conditions) ? conditions.length : 0,
  };
}

export function buildRuleAuditFilters(rule: RuleSummary, limit = 25) {
  return {
    limit,
    rule_id: rule.rule_id,
    rule_version: rule.latest_version,
  };
}

export function buildRuleCandidateSaveSummary(response?: RuleCandidateSaveResponse | null) {
  if (!response) {
    return null;
  }
  return {
    ruleId: response.summary.rule_id,
    name: response.summary.name,
    status: response.summary.status,
    owner: response.summary.owner,
    versionLabel: `v${response.summary.latest_version}`,
    reviewMode: response.summary.review_mode,
    schemeFamily: response.summary.scheme_family,
    score: response.summary.score,
    alertCode: response.summary.alert_code,
    recommendedAction: response.summary.recommended_action,
    versionCount: response.versions.length,
    auditEventCount: response.audit_events.length,
  };
}

const defaultBacktest = JSON.stringify(
  {
    rule: {
      rule_id: "candidate_early_claim",
      version: 1,
      name: "Candidate early claim",
      conditions: [
        {
          field: "days_since_policy_start",
          operator: "<=",
          value: 7,
        },
      ],
      action: {
        score: 25,
        alert_code: "EARLY_CLAIM",
        recommended_action: "ManualReview",
        reason: "保单生效后 7 天内发生理赔",
      },
    },
    samples: [
      {
        external_claim_id: "CLM-MATCH",
        claim_amount: "8000",
        currency: "CNY",
        service_date: "2026-01-06",
        policy: {
          external_policy_id: "POL-MATCH",
          coverage_start_date: "2026-01-01",
          coverage_end_date: "2026-12-31",
          coverage_limit: "10000",
        },
      },
    ],
  },
  null,
  2,
);

const defaultDiscovery = JSON.stringify(
  {
    min_support: 1,
    samples: [
      {
        external_claim_id: "CLM-FWA-EARLY-HIGH",
        claim_amount: "9000",
        currency: "CNY",
        service_date: "2026-01-05",
        confirmed_fwa: true,
        policy: {
          external_policy_id: "POL-FWA-EARLY-HIGH",
          coverage_start_date: "2026-01-01",
          coverage_end_date: "2026-12-31",
          coverage_limit: "10000",
        },
      },
      {
        external_claim_id: "CLM-NORMAL-LATE-LOW",
        claim_amount: "500",
        currency: "CNY",
        service_date: "2026-03-01",
        confirmed_fwa: false,
        policy: {
          external_policy_id: "POL-NORMAL-LATE-LOW",
          coverage_start_date: "2026-01-01",
          coverage_end_date: "2026-12-31",
          coverage_limit: "10000",
        },
      },
    ],
  },
  null,
  2,
);

export function RulesStudio() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const [selectedRuleId, setSelectedRuleId] = useState("rule_early_claim");
  const [reviewer, setReviewer] = useState("rule-governance");
  const [reviewNotes, setReviewNotes] = useState("Approved for limited rollout only.");
  const [backtestPayload, setBacktestPayload] = useState(defaultBacktest);
  const [discoveryPayload, setDiscoveryPayload] = useState(defaultDiscovery);
  const queryClient = useQueryClient();
  const rulesQuery = useQuery({
    queryKey: ["rules", apiKey],
    queryFn: () => listRules(apiKey) as Promise<{ rules: RuleSummary[] }>,
  });
  const schemesQuery = useQuery({
    queryKey: ["fwa-schemes", apiKey],
    queryFn: () => listFwaSchemes(apiKey) as Promise<{ schemes: FwaSchemeDefinition[] }>,
  });
  const schemeLabelMap = useMemo(
    () => buildFwaSchemeLabelMap(schemesQuery.data?.schemes),
    [schemesQuery.data?.schemes],
  );
  const selectedRule = useMemo(
    () =>
      rulesQuery.data?.rules.find((rule) => rule.rule_id === selectedRuleId) ??
      rulesQuery.data?.rules[0],
    [rulesQuery.data?.rules, selectedRuleId],
  );
  const detailQuery = useQuery({
    queryKey: ["rule", selectedRule?.rule_id, apiKey],
    queryFn: () => getRule(selectedRule!.rule_id, apiKey) as Promise<RuleDetailResponse>,
    enabled: Boolean(selectedRule?.rule_id),
  });
  const promotionQuery = useQuery({
    queryKey: ["rule-promotion-gates", selectedRule?.rule_id, apiKey],
    queryFn: () =>
      getRulePromotionGates(
        selectedRule!.rule_id,
        apiKey,
      ) as Promise<RulePromotionGatesResponse>,
    enabled: Boolean(selectedRule?.rule_id),
  });
  const auditQuery = useQuery({
    queryKey: ["rule-audit-events", selectedRule?.rule_id, selectedRule?.latest_version, apiKey],
    queryFn: () =>
      listAuditEvents(
        apiKey,
        buildRuleAuditFilters(selectedRule!),
      ) as Promise<{ events: AuditEvent[] }>,
    enabled: Boolean(selectedRule?.rule_id),
  });
  const qaFeedbackQuery = useQuery({
    queryKey: ["qa-feedback-items", "rules", apiKey],
    queryFn: () =>
      listQaFeedbackItems(apiKey, { feedbackTarget: "rules" }) as Promise<{
        items: QaFeedbackItem[];
      }>,
  });
  const outcomeLabelsQuery = useQuery({
    queryKey: ["outcome-labels", apiKey],
    queryFn: () => listOutcomeLabels(apiKey) as Promise<{ labels: OutcomeLabel[] }>,
  });
  const ruleFeedbackItems = useMemo(
    () => filterQaFeedbackItems(qaFeedbackQuery.data?.items ?? [], "rules"),
    [qaFeedbackQuery.data?.items],
  );
  const ruleFeedbackSummary = useMemo(
    () => summarizeQaFeedbackItems(ruleFeedbackItems),
    [ruleFeedbackItems],
  );
  const ruleLabelSummary = useMemo(
    () => buildRuleLabelReadinessSummary(outcomeLabelsQuery.data?.labels),
    [outcomeLabelsQuery.data?.labels],
  );
  const lifecycleMutation = useMutation({
    mutationFn: (action: "submit" | "approve" | "publish" | "rollback") => {
      if (!selectedRule) throw new Error("No rule selected");
      if (action === "submit") return submitRule(selectedRule.rule_id, apiKey, selectedRule.latest_version);
      if (action === "approve") return approveRule(selectedRule.rule_id, apiKey, selectedRule.latest_version);
      if (action === "rollback") return rollbackRule(selectedRule.rule_id, apiKey, selectedRule.latest_version);
      return publishRule(selectedRule.rule_id, apiKey, selectedRule.latest_version);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["rules"] });
      queryClient.invalidateQueries({ queryKey: ["rule"] });
      queryClient.invalidateQueries({ queryKey: ["rule-audit-events"] });
    },
  });
  const reviewMutation = useMutation({
    mutationFn: (decision: "approved" | "rejected") => {
      if (!selectedRule) throw new Error("No rule selected");
      return submitRulePromotionReview(
        selectedRule.rule_id,
        {
          decision,
          reviewer,
          notes: reviewNotes,
          evidence_refs: [`rules:${selectedRule.rule_id}:v${selectedRule.latest_version}`],
        },
        apiKey,
      );
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["rule-promotion-gates"] });
      queryClient.invalidateQueries({ queryKey: ["rule-audit-events"] });
    },
  });
  const backtestMutation = useMutation({
    mutationFn: () => backtestRule(JSON.parse(backtestPayload), apiKey),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["rule-audit-events"] });
    },
  });
  const backtestSummary = backtestMutation.data
    ? buildRuleBacktestSummary(backtestMutation.data as RuleBacktestResponse)
    : null;
  const promotionGateRows = promotionQuery.data
    ? buildPromotionGateEvidenceRows(promotionQuery.data.gates)
    : [];
  const discoveryMutation = useMutation({
    mutationFn: () =>
      discoverRules(JSON.parse(discoveryPayload), apiKey) as Promise<RuleDiscoveryResponse>,
  });
  const discoverySummary = buildRuleDiscoverySummary(discoveryMutation.data);
  const saveCandidateMutation = useMutation({
    mutationFn: () => {
      const rule = discoveryMutation.data?.candidates?.[0]?.rule;
      if (!rule) throw new Error("No candidate rule available");
      return saveRuleCandidate(
        { owner: "rule-discovery", rule },
        apiKey,
      ) as Promise<RuleCandidateSaveResponse>;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["rules"] });
      queryClient.invalidateQueries({ queryKey: ["rule"] });
      queryClient.invalidateQueries({ queryKey: ["rule-audit-events"] });
    },
  });
  const savedCandidateSummary = buildRuleCandidateSaveSummary(saveCandidateMutation.data);
  const ruleDetailSummary = buildRuleDetailSummary(detailQuery.data);

  return (
    <section className="ops-grid">
      <div className="panel">
        <h2>Rules</h2>
        <label>
          API Key
          <input value={apiKey} onChange={(event) => setApiKey(event.target.value)} />
        </label>
        {rulesQuery.error ? <pre className="error">{String(rulesQuery.error.message)}</pre> : null}
        {schemesQuery.error ? (
          <pre className="error">{String(schemesQuery.error.message)}</pre>
        ) : null}
        <div className="table-list">
          {rulesQuery.data?.rules.map((rule) => (
            <button
              className={rule.rule_id === selectedRule?.rule_id ? "row-button active" : "row-button"}
              key={rule.rule_id}
              onClick={() => setSelectedRuleId(rule.rule_id)}
            >
              <span>{rule.name}</span>
              <strong>{rule.status}</strong>
              <small>
                {rule.alert_code} · {formatFwaSchemeLabel(rule.scheme_family, schemeLabelMap)} ·{" "}
                {formatReviewModeLabel(rule.review_mode)}
              </small>
            </button>
          ))}
        </div>
      </div>
      <div className="panel">
        <h2>Rule Detail</h2>
        {selectedRule ? (
          <div className="result-stack">
            <dl className="result-grid">
              <div>
                <dt>Rule</dt>
                <dd>{selectedRule.rule_id}</dd>
              </div>
              <div>
                <dt>Status</dt>
                <dd>{selectedRule.status}</dd>
              </div>
              <div>
                <dt>Owner</dt>
                <dd>{selectedRule.owner}</dd>
              </div>
              <div>
                <dt>Version</dt>
                <dd>{selectedRule.latest_version}</dd>
              </div>
              <div>
                <dt>Review Mode</dt>
                <dd>{formatReviewModeLabel(selectedRule.review_mode)}</dd>
              </div>
              <div>
                <dt>Scheme</dt>
                <dd>{formatFwaSchemeLabel(selectedRule.scheme_family, schemeLabelMap)}</dd>
              </div>
              <div>
                <dt>Score</dt>
                <dd>{selectedRule.score}</dd>
              </div>
              <div>
                <dt>Action</dt>
                <dd>{selectedRule.recommended_action}</dd>
              </div>
            </dl>
            <div className="button-row">
              <button onClick={() => lifecycleMutation.mutate("submit")}>Submit</button>
              <button onClick={() => lifecycleMutation.mutate("approve")}>Approve</button>
              <button onClick={() => lifecycleMutation.mutate("publish")}>Publish</button>
              <button onClick={() => lifecycleMutation.mutate("rollback")}>Rollback</button>
            </div>
            {lifecycleMutation.error ? (
              <pre className="error">{String(lifecycleMutation.error.message)}</pre>
            ) : null}
            {auditQuery.error ? (
              <pre className="error">{String(auditQuery.error.message)}</pre>
            ) : null}
            {detailQuery.error ? (
              <pre className="error">{String(detailQuery.error.message)}</pre>
            ) : null}
            {ruleDetailSummary ? (
              <dl className="result-grid">
                <div>
                  <dt>Loaded Rule</dt>
                  <dd>{ruleDetailSummary.ruleId}</dd>
                </div>
                <div>
                  <dt>Name</dt>
                  <dd>{ruleDetailSummary.name}</dd>
                </div>
                <div>
                  <dt>Active Version</dt>
                  <dd>{ruleDetailSummary.activeVersionLabel}</dd>
                </div>
                <div>
                  <dt>Latest Version</dt>
                  <dd>{ruleDetailSummary.latestVersionLabel}</dd>
                </div>
                <div>
                  <dt>Versions</dt>
                  <dd>{ruleDetailSummary.versionCount}</dd>
                </div>
                <div>
                  <dt>Audit Events</dt>
                  <dd>{ruleDetailSummary.auditEventCount}</dd>
                </div>
                <div>
                  <dt>Latest Status</dt>
                  <dd>{ruleDetailSummary.latestStatus}</dd>
                </div>
                <div>
                  <dt>Latest Review Mode</dt>
                  <dd>{formatReviewModeLabel(ruleDetailSummary.latestReviewMode)}</dd>
                </div>
                <div>
                  <dt>Latest Scheme</dt>
                  <dd>{formatFwaSchemeLabel(ruleDetailSummary.latestSchemeFamily, schemeLabelMap)}</dd>
                </div>
                <div>
                  <dt>Latest Score</dt>
                  <dd>{ruleDetailSummary.latestScore}</dd>
                </div>
                <div>
                  <dt>Alert</dt>
                  <dd>{ruleDetailSummary.latestAlertCode}</dd>
                </div>
                <div>
                  <dt>Action</dt>
                  <dd>{ruleDetailSummary.latestAction}</dd>
                </div>
                <div>
                  <dt>Conditions</dt>
                  <dd>{ruleDetailSummary.latestConditionCount}</dd>
                </div>
                <div>
                  <dt>Reason</dt>
                  <dd>{ruleDetailSummary.latestReason}</dd>
                </div>
              </dl>
            ) : null}
          </div>
        ) : (
          <p className="empty">No rules available</p>
        )}
      </div>
      <div className="panel wide-panel">
        <h2>Rule Audit Trail</h2>
        {auditQuery.data?.events.length ? (
          <ol className="audit-timeline">
            {auditQuery.data.events.map((event) => (
              <li key={event.audit_id}>
                <div>
                  <strong>{event.event_type}</strong>
                  <span>{event.event_status}</span>
                </div>
                <small>{event.created_at || event.run_id}</small>
                <p>{event.summary}</p>
                <ul className="result-list">
                  {event.evidence_refs.map((reference) => (
                    <li key={reference}>{reference}</li>
                  ))}
                </ul>
              </li>
            ))}
          </ol>
        ) : (
          <p className="empty">No rule audit events loaded</p>
        )}
      </div>
      <div className="panel wide-panel">
        <h2>Rule Promotion Gates</h2>
        {promotionQuery.error ? (
          <pre className="error">{String(promotionQuery.error.message)}</pre>
        ) : null}
        {promotionQuery.data ? (
          <>
            <div className="summary-grid">
              <div>
                <span>Routing Decision</span>
                <strong>{promotionQuery.data.decision}</strong>
              </div>
              <div>
                <span>Review Mode</span>
                <strong>{formatReviewModeLabel(promotionQuery.data.review_mode)}</strong>
              </div>
              <div>
                <span>Gates Passed</span>
                <strong>
                  {promotionQuery.data.passed_count}/{promotionQuery.data.total_count}
                </strong>
              </div>
              <div>
                <span>Triggered</span>
                <strong>{promotionQuery.data.trigger_count}</strong>
              </div>
              <div>
                <span>Reviewed</span>
                <strong>{promotionQuery.data.reviewed_count}</strong>
              </div>
              <div>
                <span>False Positive</span>
                <strong>{promotionQuery.data.false_positive_rate.toFixed(2)}</strong>
              </div>
              <div>
                <span>Saving</span>
                <strong>{promotionQuery.data.saving_amount}</strong>
              </div>
              <div>
                <span>Open Feedback</span>
                <strong>{promotionQuery.data.open_rule_feedback_count}</strong>
              </div>
              <div>
                <span>Unresolved Feedback</span>
                <strong>{promotionQuery.data.unresolved_rule_feedback_count}</strong>
              </div>
              <div>
                <span>Approved Labels</span>
                <strong>{promotionQuery.data.approved_label_count}</strong>
              </div>
              <div>
                <span>Labels Need Review</span>
                <strong>{promotionQuery.data.needs_review_label_count}</strong>
              </div>
            </div>
            <div className="table-list">
              {promotionGateRows.map((gate) => (
                <div className="metric-row compact-metric-row" key={gate.label}>
                  <span>{gate.label}</span>
                  <strong>{gate.status}</strong>
                  <small className={gate.evidenceClassName}>{gate.evidenceSource}</small>
                </div>
              ))}
            </div>
            <div className="result-stack">
              <label>
                Reviewer
                <input value={reviewer} onChange={(event) => setReviewer(event.target.value)} />
              </label>
              <label>
                Governance Note
                <textarea
                  value={reviewNotes}
                  onChange={(event) => setReviewNotes(event.target.value)}
                />
              </label>
              <div className="button-row">
                <button
                  onClick={() => reviewMutation.mutate("approved")}
                  disabled={reviewMutation.isPending}
                >
                  Approve Promotion
                </button>
                <button
                  onClick={() => reviewMutation.mutate("rejected")}
                  disabled={reviewMutation.isPending}
                >
                  Reject Promotion
                </button>
              </div>
              {reviewMutation.error ? (
                <pre className="error">{String(reviewMutation.error.message)}</pre>
              ) : null}
            </div>
          </>
        ) : (
          <p className="empty">No promotion gate data loaded</p>
        )}
      </div>
      <div className="panel wide-panel">
        <h2>QA Feedback</h2>
        {qaFeedbackQuery.error ? (
          <pre className="error">{String(qaFeedbackQuery.error.message)}</pre>
        ) : null}
        {outcomeLabelsQuery.error ? (
          <pre className="error">{String(outcomeLabelsQuery.error.message)}</pre>
        ) : null}
        <div className="summary-grid">
          <div>
            <span>Rule Labels</span>
            <strong>{ruleLabelSummary.ruleLabelCount}</strong>
          </div>
          <div>
            <span>Training Ready</span>
            <strong>{ruleLabelSummary.approvedForTrainingCount}</strong>
          </div>
          <div>
            <span>Needs Review</span>
            <strong>{ruleLabelSummary.needsReviewCount}</strong>
          </div>
          <div>
            <span>Confirmed FWA</span>
            <strong>{ruleLabelSummary.confirmedFwaCount}</strong>
          </div>
        </div>
        <div className="summary-grid">
          <div>
            <span>Open Items</span>
            <strong>{ruleFeedbackSummary.openCount}</strong>
          </div>
          <div>
            <span>Highest Priority</span>
            <strong>{ruleFeedbackSummary.highestPriority}</strong>
          </div>
          <div>
            <span>Evidence Backed</span>
            <strong>{ruleFeedbackSummary.evidenceBackedCount}</strong>
          </div>
        </div>
        <div className="table-list">
          {ruleFeedbackItems.map((item) => (
            <div className="metric-row compact-metric-row" key={item.feedback_id}>
              <span>{item.summary}</span>
              <strong>{item.issue_type}</strong>
              <small>
                {item.priority} · {item.status}
              </small>
              <small>{item.evidence_refs.length} evidence refs</small>
              {buildQaFeedbackStatusAuditLabel(item) ? (
                <small>{buildQaFeedbackStatusAuditLabel(item)}</small>
              ) : null}
              {buildQaFeedbackStatusEvidenceLabel(item) ? (
                <small>{buildQaFeedbackStatusEvidenceLabel(item)}</small>
              ) : null}
            </div>
          ))}
        </div>
        {ruleFeedbackItems.length === 0 ? <p className="empty">No rule feedback items</p> : null}
      </div>
      <div className="panel wide-panel">
        <h2>Rule Backtest</h2>
        <textarea
          value={backtestPayload}
          onChange={(event) => setBacktestPayload(event.target.value)}
        />
        <button onClick={() => backtestMutation.mutate()} disabled={backtestMutation.isPending}>
          Run Backtest
        </button>
        {backtestMutation.error ? (
          <pre className="error">{String(backtestMutation.error.message)}</pre>
        ) : null}
        {backtestSummary ? (
          <div className="result-stack">
            <div className="summary-grid">
              <div>
                <span>Matched</span>
                <strong>
                  {backtestSummary.matchedCount}/{backtestSummary.sampleCount}
                </strong>
              </div>
              <div>
                <span>Reviewed</span>
                <strong>{backtestSummary.reviewedCount}</strong>
              </div>
              <div>
                <span>Precision</span>
                <strong>{backtestSummary.precisionLabel}</strong>
              </div>
              <div>
                <span>Recall</span>
                <strong>{backtestSummary.recallLabel}</strong>
              </div>
              <div>
                <span>Lift</span>
                <strong>{backtestSummary.liftLabel}</strong>
              </div>
              <div>
                <span>False Positive</span>
                <strong>{backtestSummary.falsePositiveRateLabel}</strong>
              </div>
              <div>
                <span>Recommendation</span>
                <strong>{backtestSummary.recommendation}</strong>
              </div>
              <div>
                <span>Saving</span>
                <strong>{backtestSummary.estimatedSaving}</strong>
              </div>
              <div>
                <span>Evidence</span>
                <strong>{backtestSummary.evidenceCount}</strong>
              </div>
            </div>
            <p className="empty">Blockers: {backtestSummary.blockerLabel}</p>
            {backtestSummary.matchedClaimIds.length > 0 ? (
              <div className="table-list">
                {backtestSummary.matchedClaimIds.map((claimId) => (
                  <div className="metric-row compact-metric-row" key={claimId}>
                    <span>{claimId}</span>
                    <strong>matched</strong>
                  </div>
                ))}
              </div>
            ) : (
              <p className="empty">No matched claims</p>
            )}
            {backtestSummary.evidenceRefs.length > 0 ? (
              <ol className="audit-timeline">
                {backtestSummary.evidenceRefs.map((reference) => (
                  <li key={reference}>
                    <span>{reference}</span>
                  </li>
                ))}
              </ol>
            ) : null}
          </div>
        ) : null}
      </div>
      <div className="panel wide-panel">
        <h2>Rule Discovery</h2>
        <textarea
          value={discoveryPayload}
          onChange={(event) => setDiscoveryPayload(event.target.value)}
        />
        <button onClick={() => discoveryMutation.mutate()} disabled={discoveryMutation.isPending}>
          Discover Candidates
        </button>
        {discoveryMutation.error ? (
          <pre className="error">{String(discoveryMutation.error.message)}</pre>
        ) : null}
        {discoveryMutation.data ? (
          <div className="result-stack">
            <div className="summary-grid">
              <div>
                <span>Samples</span>
                <strong>{discoverySummary.sampleCount}</strong>
              </div>
              <div>
                <span>Positive Labels</span>
                <strong>{discoverySummary.positiveCount}</strong>
              </div>
              <div>
                <span>Candidates</span>
                <strong>{discoverySummary.candidateCount}</strong>
              </div>
              <div>
                <span>Top Rule</span>
                <strong>{discoverySummary.topRuleId}</strong>
              </div>
              <div>
                <span>Top Precision</span>
                <strong>{discoverySummary.topPrecisionLabel}</strong>
              </div>
              <div>
                <span>Top Lift</span>
                <strong>{discoverySummary.topLiftLabel}</strong>
              </div>
              <div>
                <span>Top Saving</span>
                <strong>{discoverySummary.topSaving}</strong>
              </div>
            </div>
            <div className="table-list">
              {discoveryMutation.data.candidates.map((candidate) => (
                <div className="metric-row" key={candidate.rule.rule_id}>
                  <span>
                    {candidate.rule.name ?? candidate.rule.rule_id}
                    <small>
                      {candidate.explanation} · matched{" "}
                      {candidate.matched_claim_ids.slice(0, 3).join(", ") || "none"}
                    </small>
                  </span>
                  <strong>{(candidate.precision * 100).toFixed(1)}%</strong>
                  <small>
                    support {candidate.support} · lift {candidate.lift.toFixed(2)}x · FPR{" "}
                    {(candidate.false_positive_rate * 100).toFixed(1)}% · saving{" "}
                    {candidate.estimated_saving}
                  </small>
                </div>
              ))}
            </div>
            <button
              onClick={() => saveCandidateMutation.mutate()}
              disabled={saveCandidateMutation.isPending}
            >
              Save Top Candidate
            </button>
            {saveCandidateMutation.error ? (
              <pre className="error">{String(saveCandidateMutation.error.message)}</pre>
            ) : null}
            {savedCandidateSummary ? (
              <dl className="result-grid">
                <div>
                  <dt>Saved Rule</dt>
                  <dd>{savedCandidateSummary.ruleId}</dd>
                </div>
                <div>
                  <dt>Name</dt>
                  <dd>{savedCandidateSummary.name}</dd>
                </div>
                <div>
                  <dt>Status</dt>
                  <dd>{savedCandidateSummary.status}</dd>
                </div>
                <div>
                  <dt>Owner</dt>
                  <dd>{savedCandidateSummary.owner}</dd>
                </div>
                <div>
                  <dt>Version</dt>
                  <dd>{savedCandidateSummary.versionLabel}</dd>
                </div>
                <div>
                  <dt>Review Mode</dt>
                  <dd>{formatReviewModeLabel(savedCandidateSummary.reviewMode)}</dd>
                </div>
                <div>
                  <dt>Scheme</dt>
                  <dd>{formatFwaSchemeLabel(savedCandidateSummary.schemeFamily, schemeLabelMap)}</dd>
                </div>
                <div>
                  <dt>Score</dt>
                  <dd>{savedCandidateSummary.score}</dd>
                </div>
                <div>
                  <dt>Alert</dt>
                  <dd>{savedCandidateSummary.alertCode}</dd>
                </div>
                <div>
                  <dt>Action</dt>
                  <dd>{savedCandidateSummary.recommendedAction}</dd>
                </div>
                <div>
                  <dt>Versions</dt>
                  <dd>{savedCandidateSummary.versionCount}</dd>
                </div>
                <div>
                  <dt>Embedded Audits</dt>
                  <dd>{savedCandidateSummary.auditEventCount}</dd>
                </div>
              </dl>
            ) : null}
          </div>
        ) : null}
      </div>
    </section>
  );
}
