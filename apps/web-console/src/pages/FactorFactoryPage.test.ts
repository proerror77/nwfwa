import { describe, expect, it } from "vitest";
import { buildFactorCards } from "./FactorFactoryPage";

describe("buildFactorCards", () => {
  it("derives factor cards from profiled dataset fields", () => {
    const cards = buildFactorCards({
      dataset_id: "dataset_1",
      dataset_key: "demo_claims_fwa",
      entity_keys: ["claim_id", "provider_id"],
      fields: [
        {
          field_name: "claim_amount_to_limit_ratio",
          logical_type: "decimal",
          nullable: false,
          semantic_role: "feature",
          description: "Claim amount divided by policy limit.",
          profile_json: {
            missing_rate: 0.03,
            top_values: [{ value: "0.8", count: 12 }],
          },
        },
        {
          field_name: "confirmed_fwa",
          logical_type: "boolean",
          nullable: false,
          semantic_role: "label",
          description: "Confirmed FWA label.",
          profile_json: { missing_rate: 0 },
        },
      ],
    });

    expect(cards[0]).toMatchObject({
      factor_name: "claim_amount_to_limit_ratio",
      semantic_role: "feature",
      display_label: "Claim Amount To Limit Ratio",
      missing_rate_label: "3.0%",
      online_status: "ready",
      is_label: false,
      is_entity_key: false,
      top_values: ["0.8 (12)"],
    });
    expect(cards[1]).toMatchObject({
      factor_name: "confirmed_fwa",
      semantic_role: "label",
      online_status: "review",
      is_label: true,
    });
  });
});
