import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { scoreClaim } from "../api";

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
    mutationFn: () => scoreClaim(JSON.parse(payload), apiKey),
  });

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
        {mutation.data ? <pre>{JSON.stringify(mutation.data, null, 2)}</pre> : null}
      </div>
    </section>
  );
}
