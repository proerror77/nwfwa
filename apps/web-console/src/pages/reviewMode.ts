export function formatReviewModeLabel(reviewMode?: string) {
  if (reviewMode === "pre_payment") return "Pre-payment";
  if (reviewMode === "post_payment") return "Post-payment";
  if (reviewMode === "both") return "Pre + post";
  return "Unspecified";
}
