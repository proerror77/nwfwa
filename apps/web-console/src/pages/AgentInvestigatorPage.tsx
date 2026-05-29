import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { investigateCase, listFwaSchemes } from "../api";
import {
  buildFwaSchemeLabelMap,
  buildFwaSchemeOptions,
  formatFwaSchemeLabel,
  type FwaSchemeDefinition,
} from "./fwaSchemeOptions";

type InvestigationResponse = {
  agent_run_id: string;
  decision_boundary: string;
  risk_summary: string;
  findings: Array<{ finding: string; evidence_refs: string[] }>;
  investigation_checklist: string[];
  similar_cases: SimilarCase[];
  qa_opinion_draft: string;
  evidence_sufficiency: EvidenceSufficiency;
  evidence_refs: string[];
};

export type SimilarCase = {
  case_id: string;
  similarity_score: number;
  matched_signals: string[];
};

type EvidenceSufficiency = {
  scheme_family: string;
  status: string;
  minimum_evidence: string[];
  present_evidence: string[];
  missing_evidence: string[];
};

export function buildEvidenceSufficiencyRows(sufficiency?: EvidenceSufficiency) {
  const present = new Set(sufficiency?.present_evidence ?? []);
  return (sufficiency?.minimum_evidence ?? []).map((item) => ({
    item,
    status: present.has(item) ? "present" : "missing",
  }));
}

export function buildAgentSimilarCaseRows(cases: SimilarCase[] = []) {
  return cases.map((item) => ({
    caseId: item.case_id,
    similarityLabel: `${(item.similarity_score * 100).toFixed(0)}%`,
    matchedSignalLabel: item.matched_signals.length ? item.matched_signals.join(", ") : "none",
  }));
}

export function AgentInvestigatorPage() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const [claimId, setClaimId] = useState("CLM-0287");
  const [riskScore, setRiskScore] = useState(87);
  const [rag, setRag] = useState("RED");
  const [schemeFamily, setSchemeFamily] = useState("diagnosis_procedure_mismatch");
  const [topReasons, setTopReasons] = useState(
    "金额高于同病种同地区 P99\n诊断-项目匹配度偏低",
  );
  const [diagnosisCode, setDiagnosisCode] = useState("J10");
  const [providerRegion, setProviderRegion] = useState("Shanghai");
  const [tags, setTags] = useState("early_claim, high_amount");
  const [result, setResult] = useState<InvestigationResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const schemesQuery = useQuery({
    queryKey: ["fwa-schemes", apiKey],
    queryFn: () => listFwaSchemes(apiKey) as Promise<{ schemes: FwaSchemeDefinition[] }>,
  });
  const schemeOptions = buildFwaSchemeOptions(schemesQuery.data?.schemes, schemeFamily);
  const schemeLabelMap = buildFwaSchemeLabelMap(schemesQuery.data?.schemes);
  const evidenceRows = buildEvidenceSufficiencyRows(result?.evidence_sufficiency);
  const similarCaseRows = buildAgentSimilarCaseRows(result?.similar_cases);

  async function runInvestigation() {
    setError(null);
    try {
      const response = (await investigateCase(
        {
          claim_id: claimId,
          risk_score: riskScore,
          rag,
          scheme_family: schemeFamily,
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
          <label>
            Scheme
            <select
              value={schemeFamily}
              onChange={(event) => setSchemeFamily(event.target.value)}
            >
              {schemeOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
        </div>
        {schemesQuery.error ? <pre className="error">{String(schemesQuery.error.message)}</pre> : null}
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
            <dl className="result-grid">
              <div>
                <dt>Scheme</dt>
                <dd>
                  {formatFwaSchemeLabel(
                    result.evidence_sufficiency.scheme_family,
                    schemeLabelMap,
                  )}
                </dd>
              </div>
              <div>
                <dt>Evidence Status</dt>
                <dd>{result.evidence_sufficiency.status}</dd>
              </div>
            </dl>
            <ul className="result-list">
              {evidenceRows.map((row) => (
                <li key={row.item}>
                  <strong>{row.item}</strong>
                  <span>{row.status}</span>
                </li>
              ))}
            </ul>
            {similarCaseRows.length > 0 ? (
              <ul className="result-list">
                {similarCaseRows.map((row) => (
                  <li key={row.caseId}>
                    <strong>
                      {row.caseId} · {row.similarityLabel}
                    </strong>
                    <span>{row.matchedSignalLabel}</span>
                  </li>
                ))}
              </ul>
            ) : null}
            {result.evidence_refs.length > 0 ? (
              <ul className="result-list compact-list">
                {result.evidence_refs.map((reference) => (
                  <li key={reference}>{reference}</li>
                ))}
              </ul>
            ) : null}
            <p>{result.qa_opinion_draft}</p>
          </div>
        ) : (
          <p className="empty">No investigation run yet</p>
        )}
      </div>
    </section>
  );
}
