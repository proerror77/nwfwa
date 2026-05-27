import { useState } from "react";
import { investigateCase } from "../api";

type InvestigationResponse = {
  agent_run_id: string;
  decision_boundary: string;
  risk_summary: string;
  findings: Array<{ finding: string; evidence_refs: string[] }>;
  investigation_checklist: string[];
  similar_cases: Array<{ case_id: string; similarity_score: number; matched_signals: string[] }>;
  qa_opinion_draft: string;
  evidence_refs: string[];
};

export function AgentInvestigatorPage() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const [claimId, setClaimId] = useState("CLM-0287");
  const [riskScore, setRiskScore] = useState(87);
  const [rag, setRag] = useState("RED");
  const [topReasons, setTopReasons] = useState(
    "金额高于同病种同地区 P99\n诊断-项目匹配度偏低",
  );
  const [diagnosisCode, setDiagnosisCode] = useState("J10");
  const [providerRegion, setProviderRegion] = useState("Shanghai");
  const [tags, setTags] = useState("early_claim, high_amount");
  const [result, setResult] = useState<InvestigationResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function runInvestigation() {
    setError(null);
    try {
      const response = (await investigateCase(
        {
          claim_id: claimId,
          risk_score: riskScore,
          rag,
          top_reasons: topReasons
            .split("\n")
            .map((reason) => reason.trim())
            .filter(Boolean),
          similar_case_query: {
            diagnosis_code: diagnosisCode,
            provider_region: providerRegion,
            tags: tags
              .split(",")
              .map((tag) => tag.trim())
              .filter(Boolean),
          },
        },
        apiKey,
      )) as InvestigationResponse;
      setResult(response);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  }

  return (
    <section className="ops-grid">
      <div className="panel">
        <h2>Agent Investigator</h2>
        <label>
          API Key
          <input value={apiKey} onChange={(event) => setApiKey(event.target.value)} />
        </label>
        <label>
          Claim ID
          <input value={claimId} onChange={(event) => setClaimId(event.target.value)} />
        </label>
        <div className="form-grid">
          <label>
            Risk Score
            <input
              type="number"
              value={riskScore}
              onChange={(event) => setRiskScore(Number(event.target.value))}
            />
          </label>
          <label>
            RAG
            <input value={rag} onChange={(event) => setRag(event.target.value)} />
          </label>
        </div>
        <label>
          Top Reasons
          <textarea value={topReasons} onChange={(event) => setTopReasons(event.target.value)} />
        </label>
      </div>
      <div className="panel">
        <h2>Similar Case Query</h2>
        <label>
          Diagnosis
          <input value={diagnosisCode} onChange={(event) => setDiagnosisCode(event.target.value)} />
        </label>
        <label>
          Region
          <input value={providerRegion} onChange={(event) => setProviderRegion(event.target.value)} />
        </label>
        <label>
          Tags
          <input value={tags} onChange={(event) => setTags(event.target.value)} />
        </label>
        <button onClick={runInvestigation}>Run Investigation</button>
        {error ? <pre className="error">{error}</pre> : null}
      </div>
      <div className="panel wide-panel">
        <h2>Investigation Package</h2>
        {result ? (
          <div className="result-stack">
            <dl className="result-grid">
              <div>
                <dt>Agent Run</dt>
                <dd>{result.agent_run_id}</dd>
              </div>
              <div>
                <dt>Boundary</dt>
                <dd>{result.decision_boundary}</dd>
              </div>
            </dl>
            <p>{result.risk_summary}</p>
            <ul className="result-list">
              {result.findings.map((finding) => (
                <li key={finding.finding}>
                  <strong>{finding.finding}</strong>
                  <span>{finding.evidence_refs.join(", ")}</span>
                </li>
              ))}
            </ul>
            <ul className="result-list">
              {result.investigation_checklist.map((item) => (
                <li key={item}>{item}</li>
              ))}
            </ul>
            <p>{result.qa_opinion_draft}</p>
          </div>
        ) : (
          <p className="empty">No investigation run yet</p>
        )}
      </div>
    </section>
  );
}
