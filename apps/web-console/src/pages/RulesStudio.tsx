import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  approveRule,
  backtestRule,
  discoverRules,
  getRule,
  listRules,
  publishRule,
  saveRuleCandidate,
  submitRule,
} from "../api";

type RuleSummary = {
  rule_id: string;
  name: string;
  status: string;
  owner: string;
  active_version: number | null;
  latest_version: number;
  score: number;
  alert_code: string;
  recommended_action: string;
};

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
  const [backtestPayload, setBacktestPayload] = useState(defaultBacktest);
  const [discoveryPayload, setDiscoveryPayload] = useState(defaultDiscovery);
  const queryClient = useQueryClient();
  const rulesQuery = useQuery({
    queryKey: ["rules", apiKey],
    queryFn: () => listRules(apiKey) as Promise<{ rules: RuleSummary[] }>,
  });
  const selectedRule = useMemo(
    () =>
      rulesQuery.data?.rules.find((rule) => rule.rule_id === selectedRuleId) ??
      rulesQuery.data?.rules[0],
    [rulesQuery.data?.rules, selectedRuleId],
  );
  const detailQuery = useQuery({
    queryKey: ["rule", selectedRule?.rule_id, apiKey],
    queryFn: () => getRule(selectedRule!.rule_id, apiKey),
    enabled: Boolean(selectedRule?.rule_id),
  });
  const lifecycleMutation = useMutation({
    mutationFn: (action: "submit" | "approve" | "publish") => {
      if (!selectedRule) throw new Error("No rule selected");
      if (action === "submit") return submitRule(selectedRule.rule_id, apiKey);
      if (action === "approve") return approveRule(selectedRule.rule_id, apiKey);
      return publishRule(selectedRule.rule_id, apiKey);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["rules"] });
      queryClient.invalidateQueries({ queryKey: ["rule"] });
    },
  });
  const backtestMutation = useMutation({
    mutationFn: () => backtestRule(JSON.parse(backtestPayload), apiKey),
  });
  const discoveryMutation = useMutation({
    mutationFn: () => discoverRules(JSON.parse(discoveryPayload), apiKey),
  });
  const saveCandidateMutation = useMutation({
    mutationFn: () => {
      const discovery = discoveryMutation.data as
        | { candidates?: Array<{ rule?: unknown }> }
        | undefined;
      const rule = discovery?.candidates?.[0]?.rule;
      if (!rule) throw new Error("No candidate rule available");
      return saveRuleCandidate({ owner: "rule-discovery", rule }, apiKey);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["rules"] });
      queryClient.invalidateQueries({ queryKey: ["rule"] });
    },
  });

  return (
    <section className="ops-grid">
      <div className="panel">
        <h2>Rules</h2>
        <label>
          API Key
          <input value={apiKey} onChange={(event) => setApiKey(event.target.value)} />
        </label>
        {rulesQuery.error ? <pre className="error">{String(rulesQuery.error.message)}</pre> : null}
        <div className="table-list">
          {rulesQuery.data?.rules.map((rule) => (
            <button
              className={rule.rule_id === selectedRule?.rule_id ? "row-button active" : "row-button"}
              key={rule.rule_id}
              onClick={() => setSelectedRuleId(rule.rule_id)}
            >
              <span>{rule.name}</span>
              <strong>{rule.status}</strong>
              <small>{rule.alert_code}</small>
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
            </div>
            {lifecycleMutation.error ? (
              <pre className="error">{String(lifecycleMutation.error.message)}</pre>
            ) : null}
            <pre>{JSON.stringify(detailQuery.data, null, 2)}</pre>
          </div>
        ) : (
          <p className="empty">No rules available</p>
        )}
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
        {backtestMutation.data ? <pre>{JSON.stringify(backtestMutation.data, null, 2)}</pre> : null}
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
            <button
              onClick={() => saveCandidateMutation.mutate()}
              disabled={saveCandidateMutation.isPending}
            >
              Save Top Candidate
            </button>
            {saveCandidateMutation.error ? (
              <pre className="error">{String(saveCandidateMutation.error.message)}</pre>
            ) : null}
            {saveCandidateMutation.data ? (
              <pre>{JSON.stringify(saveCandidateMutation.data, null, 2)}</pre>
            ) : null}
            <pre>{JSON.stringify(discoveryMutation.data, null, 2)}</pre>
          </div>
        ) : null}
      </div>
    </section>
  );
}
