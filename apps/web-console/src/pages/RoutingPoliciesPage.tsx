import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  activateRoutingPolicy,
  approveRoutingPolicy,
  getRoutingPolicyPromotionGates,
  listAuditEvents,
  listRoutingPolicies,
  rollbackRoutingPolicy,
  saveRoutingPolicyCandidate,
  submitRoutingPolicy,
} from "../api";
import {
  buildPromotionGateEvidenceRows,
  type PromotionGate,
} from "./promotionGateEvidence";
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

type RoutingPolicyPromotionGatesResponse = {
  policy_id: string;
  version: number;
  review_mode: string;
  status: string;
  decision: string;
  passed_count: number;
  total_count: number;
  gates: PromotionGate[];
  blockers: string[];
};

type AuditEvent = {
  audit_id: string;
  run_id: string;
  event_type: string;
  event_status: string;
  summary: string;
  payload?: Record<string, unknown>;
  evidence_refs: string[];
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

export function buildRoutingPolicyAuditFilters(policy: RoutingPolicyRecord, limit = 25) {
  return {
    limit,
    routing_policy_id: policy.policy_id,
    routing_policy_version: policy.version,
    review_mode: policy.review_mode,
  };
}

export function buildRoutingPolicyCandidateSaveSummary(policy?: RoutingPolicyRecord | null) {
  if (!policy) {
    return null;
  }
  return {
    policyId: policy.policy_id,
    versionLabel: `v${policy.version}`,
    reviewMode: policy.review_mode,
    status: policy.status,
    owner: policy.owner,
    riskThresholdLabel: `Low <= ${policy.risk_thresholds.low_max}, Medium >= ${policy.risk_thresholds.medium_min}, High >= ${policy.risk_thresholds.high_min}, Critical >= ${policy.risk_thresholds.critical_min}`,
    confidenceThresholdLabel: `Low confidence < ${policy.confidence_thresholds.low_confidence_below}, High confidence >= ${policy.confidence_thresholds.high_confidence_min}`,
    providerThresholdLabel: `Provider review >= ${policy.provider_review_threshold}`,
    createdAt: policy.created_at ?? "not recorded",
  };
}

export function buildRoutingPolicyThresholdGovernance(policy?: RoutingPolicyRecord | null) {
  if (!policy) {
    return {
      thresholdIntegrity: "not_available",
      riskRouteBand: "not_available",
      confidenceRouteBand: "not_available",
      providerRouteBand: "not_available",
      routeBoundaryStatus: "not_available",
    };
  }
  const thresholds = policy.risk_thresholds;
  const confidence = policy.confidence_thresholds;
  const riskThresholdsAreOrdered =
    thresholds.low_max < thresholds.medium_min &&
    thresholds.medium_min < thresholds.high_min &&
    thresholds.high_min < thresholds.critical_min;
  const confidenceThresholdsAreOrdered =
    confidence.low_confidence_below < confidence.high_confidence_min;
  const providerThresholdIsRoutable =
    policy.provider_review_threshold >= thresholds.medium_min &&
    policy.provider_review_threshold <= 100;
  return {
    thresholdIntegrity:
      riskThresholdsAreOrdered && confidenceThresholdsAreOrdered && providerThresholdIsRoutable
        ? "thresholds_ordered"
        : "threshold_review_required",
    riskRouteBand: `low<=${thresholds.low_max} medium>=${thresholds.medium_min} high>=${thresholds.high_min} critical>=${thresholds.critical_min}`,
    confidenceRouteBand: `low_confidence<${confidence.low_confidence_below} high_confidence>=${confidence.high_confidence_min}`,
    providerRouteBand: `provider_review>=${policy.provider_review_threshold}`,
    routeBoundaryStatus:
      policy.provider_review_threshold >= thresholds.high_min
        ? "provider_route_high_risk_aligned"
        : "provider_route_medium_risk",
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
  const promotionQuery = useQuery({
    queryKey: ["routing-policy-promotion-gates", selectedPolicy, apiKey],
    queryFn: () =>
      getRoutingPolicyPromotionGates(
        selectedPolicy!,
        apiKey,
      ) as Promise<RoutingPolicyPromotionGatesResponse>,
    enabled: Boolean(selectedPolicy),
  });
  const auditQuery = useQuery({
    queryKey: ["routing-policy-audit-events", selectedPolicy, apiKey],
    queryFn: () =>
      listAuditEvents(
        apiKey,
        buildRoutingPolicyAuditFilters(selectedPolicy!),
      ) as Promise<{ events: AuditEvent[] }>,
    enabled: Boolean(selectedPolicy),
  });
  const summary = buildRoutingPolicySummary(policies);
  const promotionGateRows = promotionQuery.data
    ? buildPromotionGateEvidenceRows(promotionQuery.data.gates)
    : [];
  const thresholdGovernance = buildRoutingPolicyThresholdGovernance(selectedPolicy);
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
      queryClient.invalidateQueries({ queryKey: ["routing-policy-promotion-gates"] });
      queryClient.invalidateQueries({ queryKey: ["routing-policy-audit-events"] });
    },
  });
  const saveCandidateMutation = useMutation({
    mutationFn: () => saveRoutingPolicyCandidate(JSON.parse(candidatePayload), apiKey),
    onSuccess: (record) => {
      const policy = record as RoutingPolicyRecord;
      setSelectedPolicyKey(routingPolicyKey(policy));
      queryClient.invalidateQueries({ queryKey: ["routing-policies"] });
      queryClient.invalidateQueries({ queryKey: ["routing-policy-promotion-gates"] });
      queryClient.invalidateQueries({ queryKey: ["routing-policy-audit-events"] });
    },
  });
  const savedCandidateSummary = buildRoutingPolicyCandidateSaveSummary(
    saveCandidateMutation.data as RoutingPolicyRecord | undefined,
  );

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
              <div>
                <span>Threshold Integrity</span>
                <strong>{thresholdGovernance.thresholdIntegrity}</strong>
              </div>
              <div>
                <span>Route Boundary</span>
                <strong>{thresholdGovernance.routeBoundaryStatus}</strong>
              </div>
            </div>
            <dl className="result-grid">
              <div>
                <dt>Risk Route Band</dt>
                <dd>{thresholdGovernance.riskRouteBand}</dd>
              </div>
              <div>
                <dt>Confidence Route Band</dt>
                <dd>{thresholdGovernance.confidenceRouteBand}</dd>
              </div>
              <div>
                <dt>Provider Route Band</dt>
                <dd>{thresholdGovernance.providerRouteBand}</dd>
              </div>
            </dl>
            <div className="button-row">
              <button onClick={() => lifecycleMutation.mutate("submit")}>Submit</button>
              <button onClick={() => lifecycleMutation.mutate("approve")}>Approve</button>
              <button onClick={() => lifecycleMutation.mutate("activate")}>Activate</button>
              <button onClick={() => lifecycleMutation.mutate("rollback")}>Rollback</button>
            </div>
            {lifecycleMutation.error ? (
              <pre className="error">{String(lifecycleMutation.error.message)}</pre>
            ) : null}
            {promotionQuery.error ? (
              <pre className="error">{String(promotionQuery.error.message)}</pre>
            ) : null}
            {auditQuery.error ? (
              <pre className="error">{String(auditQuery.error.message)}</pre>
            ) : null}
          </div>
        ) : (
          <p className="empty">No routing policies available</p>
        )}
      </div>
      <div className="panel wide-panel">
        <h2>Lifecycle Audit Trail</h2>
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
          <p className="empty">No lifecycle audit events loaded</p>
        )}
      </div>
      <div className="panel wide-panel">
        <h2>Promotion Gates</h2>
        {promotionQuery.data ? (
          <>
            <div className="summary-grid">
              <div>
                <span>Decision</span>
                <strong>{promotionQuery.data.decision}</strong>
              </div>
              <div>
                <span>Status</span>
                <strong>{promotionQuery.data.status}</strong>
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
                <span>Blockers</span>
                <strong>{promotionQuery.data.blockers.length}</strong>
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
          </>
        ) : (
          <p className="empty">No promotion gate data loaded</p>
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
        {savedCandidateSummary ? (
          <dl className="result-grid">
            <div>
              <dt>Saved Policy</dt>
              <dd>{savedCandidateSummary.policyId}</dd>
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
              <dt>Status</dt>
              <dd>{savedCandidateSummary.status}</dd>
            </div>
            <div>
              <dt>Owner</dt>
              <dd>{savedCandidateSummary.owner}</dd>
            </div>
            <div>
              <dt>Risk Thresholds</dt>
              <dd>{savedCandidateSummary.riskThresholdLabel}</dd>
            </div>
            <div>
              <dt>Confidence Thresholds</dt>
              <dd>{savedCandidateSummary.confidenceThresholdLabel}</dd>
            </div>
            <div>
              <dt>Provider Route</dt>
              <dd>{savedCandidateSummary.providerThresholdLabel}</dd>
            </div>
            <div>
              <dt>Created</dt>
              <dd>{savedCandidateSummary.createdAt}</dd>
            </div>
          </dl>
        ) : null}
      </div>
    </section>
  );
}
