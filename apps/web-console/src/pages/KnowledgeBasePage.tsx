import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  listFwaSchemes,
  listKnowledgeCases,
  publishKnowledgeCase,
  searchSimilarCases,
} from "../api";
import {
  buildFwaSchemeLabelMap,
  buildFwaSchemeOptions,
  formatFwaSchemeLabel,
  type FwaSchemeDefinition,
} from "./fwaSchemeOptions";

type KnowledgeCase = {
  case_id: string;
  title: string;
  fwa_type: string;
  scheme_family: string;
  diagnosis_code: string;
  provider_region: string;
  provider_type: string;
  summary: string;
  outcome: string;
  tags: string[];
  evidence_refs: string[];
};

type SimilarCase = {
  case_id: string;
  title: string;
  scheme_family: string;
  similarity_score: number;
  matched_signals: string[];
  retrieval_method: string;
  provenance_refs: string[];
  summary: string;
  outcome: string;
  evidence_refs: string[];
};

export function KnowledgeBasePage() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const [selectedCaseId, setSelectedCaseId] = useState("KC-1001");
  const [diagnosisCode, setDiagnosisCode] = useState("J10");
  const [providerRegion, setProviderRegion] = useState("Shanghai");
  const [tags, setTags] = useState("early_claim, high_amount");
  const [publishCaseId, setPublishCaseId] = useState("KC-PILOT-001");
  const [publishTitle, setPublishTitle] = useState("Confirmed pilot FWA case");
  const [publishSourceClaimId, setPublishSourceClaimId] = useState("CLM-0287");
  const [publishSchemeFamily, setPublishSchemeFamily] = useState("early_high_value_claim");
  const [lastSearch, setLastSearch] = useState<SimilarCase[] | null>(null);
  const queryClient = useQueryClient();

  const casesQuery = useQuery({
    queryKey: ["knowledge-cases", apiKey],
    queryFn: () => listKnowledgeCases(apiKey) as Promise<{ cases: KnowledgeCase[] }>,
  });
  const schemesQuery = useQuery({
    queryKey: ["fwa-schemes", apiKey],
    queryFn: () => listFwaSchemes(apiKey) as Promise<{ schemes: FwaSchemeDefinition[] }>,
  });
  const schemeOptions = buildFwaSchemeOptions(schemesQuery.data?.schemes, publishSchemeFamily);
  const schemeLabelMap = buildFwaSchemeLabelMap(schemesQuery.data?.schemes);
  const selectedCase = useMemo(
    () =>
      casesQuery.data?.cases.find((item) => item.case_id === selectedCaseId) ??
      casesQuery.data?.cases[0],
    [casesQuery.data?.cases, selectedCaseId],
  );

  async function runSearch() {
    const response = (await searchSimilarCases(
      {
        diagnosis_code: diagnosisCode,
        provider_region: providerRegion,
        tags: tags
          .split(",")
          .map((tag) => tag.trim())
          .filter(Boolean),
      },
      apiKey,
    )) as { results: SimilarCase[] };
    setLastSearch(response.results);
  }
  const publishMutation = useMutation({
    mutationFn: () =>
      publishKnowledgeCase(
        {
          case_id: publishCaseId,
          title: publishTitle,
          fwa_type: "Waste",
          scheme_family: publishSchemeFamily,
          diagnosis_code: diagnosisCode,
          provider_region: providerRegion,
          provider_type: "provider",
          summary: "Confirmed case published from operations review.",
          outcome: "Published to FWA knowledge base for future similarity search.",
          tags: tags
            .split(",")
            .map((tag) => tag.trim())
            .filter(Boolean),
          evidence_refs: [
            `investigation_results:INV-${publishSourceClaimId}`,
            `qa_reviews:QA-${publishSourceClaimId}`,
          ],
          source_claim_id: publishSourceClaimId,
        },
        apiKey,
      ),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["knowledge-cases"] });
    },
  });

  return (
    <section className="ops-grid">
      <div className="panel">
        <h2>Knowledge Base</h2>
        <label>
          API Key
          <input value={apiKey} onChange={(event) => setApiKey(event.target.value)} />
        </label>
        {casesQuery.error ? <pre className="error">{String(casesQuery.error.message)}</pre> : null}
        {schemesQuery.error ? (
          <pre className="error">{String(schemesQuery.error.message)}</pre>
        ) : null}
        <div className="table-list">
          {casesQuery.data?.cases.map((item) => (
            <button
              className={item.case_id === selectedCase?.case_id ? "row-button active" : "row-button"}
              key={item.case_id}
              onClick={() => setSelectedCaseId(item.case_id)}
            >
              <span>{item.title}</span>
              <strong>{item.fwa_type}</strong>
              <small>
                {item.case_id} · {formatFwaSchemeLabel(item.scheme_family, schemeLabelMap)}
              </small>
            </button>
          ))}
        </div>
      </div>
      <div className="panel">
        <h2>Case Detail</h2>
        {selectedCase ? (
          <div className="result-stack">
            <dl className="result-grid">
              <div>
                <dt>Case</dt>
                <dd>{selectedCase.case_id}</dd>
              </div>
              <div>
                <dt>Diagnosis</dt>
                <dd>{selectedCase.diagnosis_code}</dd>
              </div>
              <div>
                <dt>Scheme</dt>
                <dd>{formatFwaSchemeLabel(selectedCase.scheme_family, schemeLabelMap)}</dd>
              </div>
              <div>
                <dt>Region</dt>
                <dd>{selectedCase.provider_region}</dd>
              </div>
              <div>
                <dt>Provider</dt>
                <dd>{selectedCase.provider_type}</dd>
              </div>
            </dl>
            <p>{selectedCase.summary}</p>
            <p>{selectedCase.outcome}</p>
            <ul className="result-list">
              {selectedCase.evidence_refs.map((reference) => (
                <li key={reference}>{reference}</li>
              ))}
            </ul>
          </div>
        ) : (
          <p className="empty">No case selected</p>
        )}
      </div>
      <div className="panel wide-panel">
        <h2>Similar Search</h2>
        <div className="form-grid">
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
        </div>
        <button onClick={runSearch}>Search</button>
        {lastSearch ? (
          <ul className="result-list">
            {lastSearch.map((item) => (
              <li key={item.case_id}>
                <strong>
                  {item.case_id} · {(item.similarity_score * 100).toFixed(0)}%
                </strong>
                <small>{item.retrieval_method}</small>
                <span>{formatFwaSchemeLabel(item.scheme_family, schemeLabelMap)}</span>
                <span>{item.matched_signals.join(", ")}</span>
                <span>{item.provenance_refs.join(", ")}</span>
              </li>
            ))}
          </ul>
        ) : (
          <p className="empty">No search run yet</p>
        )}
      </div>
      <div className="panel wide-panel">
        <h2>Publish Case</h2>
        <div className="form-grid">
          <label>
            Case ID
            <input
              value={publishCaseId}
              onChange={(event) => setPublishCaseId(event.target.value)}
            />
          </label>
          <label>
            Title
            <input value={publishTitle} onChange={(event) => setPublishTitle(event.target.value)} />
          </label>
          <label>
            Source Claim
            <input
              value={publishSourceClaimId}
              onChange={(event) => setPublishSourceClaimId(event.target.value)}
            />
          </label>
          <label>
            Scheme
            <select
              value={publishSchemeFamily}
              onChange={(event) => setPublishSchemeFamily(event.target.value)}
            >
              {schemeOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
        </div>
        <button onClick={() => publishMutation.mutate()} disabled={publishMutation.isPending}>
          Publish
        </button>
        {publishMutation.error ? (
          <pre className="error">{String(publishMutation.error.message)}</pre>
        ) : null}
        {publishMutation.data ? (
          <pre>{JSON.stringify(publishMutation.data, null, 2)}</pre>
        ) : (
          <p className="empty">No case published in this session</p>
        )}
      </div>
    </section>
  );
}
