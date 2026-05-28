import { describe, expect, it } from "vitest";
import { formatReviewModeLabel } from "./reviewMode";

describe("formatReviewModeLabel", () => {
  it("formats governed pre-payment and post-payment review modes", () => {
    expect(formatReviewModeLabel("pre_payment")).toBe("Pre-payment");
    expect(formatReviewModeLabel("post_payment")).toBe("Post-payment");
    expect(formatReviewModeLabel("both")).toBe("Pre + post");
    expect(formatReviewModeLabel(undefined)).toBe("Unspecified");
  });
});
