import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  activateRoutingPolicy,
  approveRoutingPolicy,
  listRoutingPolicies,
  rollbackRoutingPolicy,
  saveRoutingPolicyCandidate,
  submitRoutingPolicy,
} from "../api";
import { formatReviewModeLabel } from "./reviewMode";

type RiskThresholds = {
  low_max: number;
  medium_min: number;
  high_min: number;
  critical_min: number;
};

type ConfidenceThresholds = {
  low_confidence_below: number;
  high_confidence_min: number;
};

export type RoutingPolicyRecord = {
  policy_id: string;
  version: number;
  review_mode: string;
  status: string;
  owner: string;
  risk_thresholds: RiskThresholds;
  confidence_thresholds: ConfidenceThresholds;
  provider_review_threshold: number;
  activated_at?: string | null;
  created_at?: string | null;
};

const defaultCandidate = JSON.stringify(
  {
    owner: "policy-ops",
    policy: {
      policy_id: "fwa_risk_fusion_routing",
      version: 2,
      review_mode: "pre_payment",
      risk_thresholds: {
        low_max: 24,
        medium_min: 25,
        high_min: 65,
        critical_min: 88,
      },
      confidence_thresholds: {
        low_confidence_below: 55,
        high_confidence_min: 85,
      },
      provider_review_threshold: 72,
    },
  },
  null,
  2,
);

function routingPolicyKey(policy: RoutingPolicyRecord) {
  return `${policy.policy_id}:${policy.review_mode}:${policy.version}`;
}

export function buildRoutingPolicySummary(policies: RoutingPolicyRecord[] = []) {
  return {
    policyCount: policies.length,
    activeCount: policies.filter((policy) => policy.status === "active").length,
    draftCount: policies.filter((policy) => policy.status === "draft").length,
    submittedCount: policies.filter((policy) => policy.status === "submitted").length,
    approvedCount: policies.filter((policy) => policy.status === "approved").length,
    reviewModeCount: new Set(policies.map((policy) => policy.review_mode)).size,
  };
}

export function RoutingPoliciesPage() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const [selectedPolicyKey, setSelectedPolicyKey] = useState("");
  const [candidatePayload, setCandidatePayload] = useState(defaultCandidate);
  const queryClient = useQueryClient();
  const policiesQuery = useQuery({
    queryKey: ["routing-policies", apiKey],
    queryFn: () =>
      listRoutingPolicies(apiKey) as Promise<{ policies: RoutingPolicyRecord[] }>,
  });
  const policies = policiesQuery.data?.policies ?? [];
  const selectedPolicy = useMemo(
    () =>
      policies.find((policy) => routingPolicyKey(policy) === selectedPolicyKey) ??
      policies[0],
    [policies, selectedPolicyKey],
  );
  const summary = buildRoutingPolicySummary(policies);
  const lifecycleMutation = useMutation({
    mutationFn: (action: "submit" | "approve" | "activate" | "rollback") => {
      if (!selectedPolicy) throw new Error("No routing policy selected");
      if (action === "submit") return submitRoutingPolicy(selectedPolicy, apiKey);
      if (action === "approve") return approveRoutingPolicy(selectedPolicy, apiKey);
      if (action === "rollback") return rollbackRoutingPolicy(selectedPolicy, apiKey);
      return activateRoutingPolicy(selectedPolicy, apiKey);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["routing-policies"] });
    },
  });
  const saveCandidateMutation = useMutation({
    mutationFn: () => saveRoutingPolicyCandidate(JSON.parse(candidatePayload), apiKey),
    onSuccess: (record) => {
      const policy = record as RoutingPolicyRecord;
      setSelectedPolicyKey(routingPolicyKey(policy));
      queryClient.invalidateQueries({ queryKey: ["routing-policies"] });
    },
  });

  return (
    <section className="ops-grid">
      <div className="panel">
        <h2>Routing Policies</h2>
        <label>
          API Key
          <input value={apiKey} onChange={(event) => setApiKey(event.target.value)} />
        </label>
        {policiesQuery.error ? (
          <pre className="error">{String(policiesQuery.error.message)}</pre>
        ) : null}
        <div className="summary-grid">
          <div>
            <span>Policies</span>
            <strong>{summary.policyCount}</strong>
          </div>
          <div>
            <span>Active</span>
            <strong>{summary.activeCount}</strong>
          </div>
          <div>
            <span>Draft</span>
            <strong>{summary.draftCount}</strong>
          </div>
          <div>
            <span>Submitted</span>
            <strong>{summary.submittedCount}</strong>
          </div>
          <div>
            <span>Approved</span>
            <strong>{summary.approvedCount}</strong>
          </div>
          <div>
            <span>Review Modes</span>
            <strong>{summary.reviewModeCount}</strong>
          </div>
        </div>
        <div className="table-list">
          {policies.map((policy) => (
            <button
              className={
                routingPolicyKey(policy) === routingPolicyKey(selectedPolicy ?? policy)
                  ? "row-button active"
                  : "row-button"
              }
              key={routingPolicyKey(policy)}
              onClick={() => setSelectedPolicyKey(routingPolicyKey(policy))}
            >
              <span>{policy.policy_id}</span>
              <strong>{policy.status}</strong>
              <small>
                v{policy.version} · {formatReviewModeLabel(policy.review_mode)} ·{" "}
                {policy.owner}
              </small>
            </button>
          ))}
        </div>
      </div>
      <div className="panel">
        <h2>Policy Detail</h2>
        {selectedPolicy ? (
          <div className="result-stack">
            <dl className="result-grid">
              <div>
                <dt>Policy</dt>
                <dd>{selectedPolicy.policy_id}</dd>
              </div>
              <div>
                <dt>Status</dt>
                <dd>{selectedPolicy.status}</dd>
              </div>
              <div>
                <dt>Version</dt>
                <dd>{selectedPolicy.version}</dd>
              </div>
              <div>
                <dt>Review Mode</dt>
                <dd>{formatReviewModeLabel(selectedPolicy.review_mode)}</dd>
              </div>
              <div>
                <dt>Owner</dt>
                <dd>{selectedPolicy.owner}</dd>
              </div>
              <div>
                <dt>Provider Review</dt>
                <dd>{selectedPolicy.provider_review_threshold}</dd>
              </div>
            </dl>
            <div className="summary-grid">
              <div>
                <span>Low Max</span>
                <strong>{selectedPolicy.risk_thresholds.low_max}</strong>
              </div>
              <div>
                <span>Medium Min</span>
                <strong>{selectedPolicy.risk_thresholds.medium_min}</strong>
              </div>
              <div>
                <span>High Min</span>
                <strong>{selectedPolicy.risk_thresholds.high_min}</strong>
              </div>
              <div>
                <span>Critical Min</span>
                <strong>{selectedPolicy.risk_thresholds.critical_min}</strong>
              </div>
              <div>
                <span>Low Confidence</span>
                <strong>{selectedPolicy.confidence_thresholds.low_confidence_below}</strong>
              </div>
              <div>
                <span>High Confidence</span>
                <strong>{selectedPolicy.confidence_thresholds.high_confidence_min}</strong>
              </div>
            </div>
            <div className="button-row">
              <button onClick={() => lifecycleMutation.mutate("submit")}>Submit</button>
              <button onClick={() => lifecycleMutation.mutate("approve")}>Approve</button>
              <button onClick={() => lifecycleMutation.mutate("activate")}>Activate</button>
              <button onClick={() => lifecycleMutation.mutate("rollback")}>Rollback</button>
            </div>
            {lifecycleMutation.error ? (
              <pre className="error">{String(lifecycleMutation.error.message)}</pre>
            ) : null}
          </div>
        ) : (
          <p className="empty">No routing policies available</p>
        )}
      </div>
      <div className="panel wide-panel">
        <h2>Candidate Policy</h2>
        <textarea
          value={candidatePayload}
          onChange={(event) => setCandidatePayload(event.target.value)}
        />
        <button
          onClick={() => saveCandidateMutation.mutate()}
          disabled={saveCandidateMutation.isPending}
        >
          Save Candidate
        </button>
        {saveCandidateMutation.error ? (
          <pre className="error">{String(saveCandidateMutation.error.message)}</pre>
        ) : null}
        {saveCandidateMutation.data ? (
          <pre>{JSON.stringify(saveCandidateMutation.data, null, 2)}</pre>
        ) : null}
      </div>
    </section>
  );
}
