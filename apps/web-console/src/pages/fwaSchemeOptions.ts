export type FwaSchemeDefinition = {
  scheme_family: string;
  display_name: string;
  risk_domain: string;
};

export type FwaSchemeOption = {
  value: string;
  label: string;
  riskDomain: string;
};

export function buildFwaSchemeOptions(
  schemes: FwaSchemeDefinition[] = [],
  fallbackValue = "",
): FwaSchemeOption[] {
  const options = [...schemes]
    .sort(
      (left, right) =>
        left.risk_domain.localeCompare(right.risk_domain) ||
        left.display_name.localeCompare(right.display_name) ||
        left.scheme_family.localeCompare(right.scheme_family),
    )
    .map((scheme) => ({
      value: scheme.scheme_family,
      label: `${scheme.display_name} (${scheme.scheme_family})`,
      riskDomain: scheme.risk_domain,
    }));

  if (fallbackValue && !options.some((option) => option.value === fallbackValue)) {
    options.unshift({
      value: fallbackValue,
      label: fallbackValue,
      riskDomain: "Current",
    });
  }

  return options;
}

export function buildFwaSchemeLabelMap(
  schemes: FwaSchemeDefinition[] = [],
): Record<string, string> {
  return Object.fromEntries(
    schemes.map((scheme) => [
      scheme.scheme_family,
      `${scheme.display_name} (${scheme.scheme_family})`,
    ]),
  );
}

export function formatFwaSchemeLabel(
  schemeFamily: string,
  labelMap: Record<string, string> = {},
) {
  return labelMap[schemeFamily] ?? schemeFamily;
}
