import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { scoreClaim } from "../api";

type ScoringResponse = {
  run_id: string;
  audit_id: string;
  claim_id: string;
  risk_score: number;
  rag: string;
  risk_level: string;
  recommended_action: string;
  confidence_score: number;
  confidence: string;
  routing_reason: string;
  scores: {
    peer_deviation_score: number;
    rule_score: number;
    anomaly_score: number;
    ml_score: number;
    medical_reasonableness_score: number;
    provider_network_score: number;
    similar_case_score: number;
    final_score: number;
  };
  alerts: Array<{
    alert_code: string;
    severity: string;
    reason: string;
    rule_id: string;
    rule_version: number;
  }>;
  top_reasons: string[];
  evidence_refs: unknown[];
};

const defaultPayload = JSON.stringify(
  {
    source_system: "tpa-demo",
    claim: {
      external_claim_id: "CLM-0287",
      claim_amount: "8000",
      currency: "CNY",
    },
  },
  null,
  2,
);

export function RuntimeScoring() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const [payload, setPayload] = useState(defaultPayload);
  const mutation = useMutation({
    mutationFn: () => scoreClaim(JSON.parse(payload), apiKey) as Promise<ScoringResponse>,
  });
  const result = mutation.data;

  return (
    <section className="runtime">
      <div className="panel">
        <h2>Runtime Scoring</h2>
        <label>
          API Key
          <input value={apiKey} onChange={(event) => setApiKey(event.target.value)} />
        </label>
        <label>
          Claim Request JSON
          <textarea value={payload} onChange={(event) => setPayload(event.target.value)} />
        </label>
        <button onClick={() => mutation.mutate()} disabled={mutation.isPending}>
          Score Claim
        </button>
      </div>
      <div className="panel">
        <h2>Result</h2>
        {mutation.error ? <pre className="error">{String(mutation.error.message)}</pre> : null}
        {result ? (
          <div className="result-stack">
            <div className="score-hero">
              <div>
                <span>Risk Score</span>
                <strong>{result.risk_score}</strong>
              </div>
              <div>
                <span>RAG</span>
                <strong>{result.rag}</strong>
              </div>
              <div>
                <span>Risk Level</span>
                <strong>{result.risk_level}</strong>
              </div>
              <div>
                <span>Action</span>
                <strong>{result.recommended_action}</strong>
              </div>
            </div>
            <dl className="result-grid">
              <div>
                <dt>Claim</dt>
                <dd>{result.claim_id}</dd>
              </div>
              <div>
                <dt>Run</dt>
                <dd>{result.run_id}</dd>
              </div>
              <div>
                <dt>Audit</dt>
                <dd>{result.audit_id}</dd>
              </div>
              <div>
                <dt>Confidence</dt>
                <dd>
                  {result.confidence} ({result.confidence_score})
                </dd>
              </div>
              <div>
                <dt>Routing</dt>
                <dd>{result.routing_reason}</dd>
              </div>
              <div>
                <dt>Peer Deviation</dt>
                <dd>{result.scores.peer_deviation_score}</dd>
              </div>
              <div>
                <dt>Rule Score</dt>
                <dd>{result.scores.rule_score}</dd>
              </div>
              <div>
                <dt>Anomaly Score</dt>
                <dd>{result.scores.anomaly_score}</dd>
              </div>
              <div>
                <dt>ML Score</dt>
                <dd>{result.scores.ml_score}</dd>
              </div>
              <div>
                <dt>Medical Score</dt>
                <dd>{result.scores.medical_reasonableness_score}</dd>
              </div>
              <div>
                <dt>Provider Score</dt>
                <dd>{result.scores.provider_network_score}</dd>
              </div>
              <div>
                <dt>Similar Case</dt>
                <dd>{result.scores.similar_case_score}</dd>
              </div>
              <div>
                <dt>Final Score</dt>
                <dd>{result.scores.final_score}</dd>
              </div>
            </dl>
            <section>
              <h3>Alerts</h3>
              {result.alerts.length > 0 ? (
                <ul className="result-list">
                  {result.alerts.map((alert) => (
                    <li key={`${alert.rule_id}-${alert.rule_version}`}>
                      <strong>{alert.alert_code}</strong>
                      <span>{alert.reason}</span>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="empty">No alerts</p>
              )}
            </section>
            <section>
              <h3>Top Reasons</h3>
              <ul className="result-list">
                {result.top_reasons.map((reason) => (
                  <li key={reason}>{reason}</li>
                ))}
              </ul>
            </section>
            <section>
              <h3>Evidence Refs</h3>
              <pre>{JSON.stringify(result.evidence_refs, null, 2)}</pre>
            </section>
            <section>
              <h3>Raw JSON</h3>
              <pre>{JSON.stringify(result, null, 2)}</pre>
            </section>
          </div>
        ) : null}
      </div>
    </section>
  );
}
