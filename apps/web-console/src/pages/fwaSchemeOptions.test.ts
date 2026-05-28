import { describe, expect, it } from "vitest";
import {
  buildFwaSchemeLabelMap,
  buildFwaSchemeOptions,
  formatFwaSchemeLabel,
} from "./fwaSchemeOptions";

describe("buildFwaSchemeOptions", () => {
  it("sorts governed FWA schemes and preserves a current fallback value", () => {
    expect(
      buildFwaSchemeOptions(
        [
          {
            scheme_family: "provider_peer_outlier",
            display_name: "Provider peer outlier",
            risk_domain: "Provider",
          },
          {
            scheme_family: "diagnosis_procedure_mismatch",
            display_name: "Diagnosis-procedure mismatch",
            risk_domain: "Clinical",
          },
        ],
        "lab_overuse",
      ),
    ).toEqual([
      {
        value: "lab_overuse",
        label: "lab_overuse",
        riskDomain: "Current",
      },
      {
        value: "diagnosis_procedure_mismatch",
        label: "Diagnosis-procedure mismatch (diagnosis_procedure_mismatch)",
        riskDomain: "Clinical",
      },
      {
        value: "provider_peer_outlier",
        label: "Provider peer outlier (provider_peer_outlier)",
        riskDomain: "Provider",
      },
    ]);
  });

  it("builds display labels for canonical scheme families", () => {
    const labelMap = buildFwaSchemeLabelMap([
      {
        scheme_family: "early_high_value_claim",
        display_name: "Early high-value claim",
        risk_domain: "Policy",
      },
    ]);

    expect(formatFwaSchemeLabel("early_high_value_claim", labelMap)).toBe(
      "Early high-value claim (early_high_value_claim)",
    );
    expect(formatFwaSchemeLabel("legacy_scheme", labelMap)).toBe("legacy_scheme");
  });
});
