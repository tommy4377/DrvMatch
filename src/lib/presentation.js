// @ts-check

/** @param {import('./types').SignatureStatus} signature */
export function signatureLabel(signature) {
  return ({
    whql: "WHQL",
    inbox: "Microsoft inbox",
    authenticode: "Authenticode",
    signedUnclassified: "Microsoft-signed",
    unsigned: "Unsigned",
    unknown: "Not determined",
  })[signature];
}

/** @param {import('./types').DriverSourceKind} source */
export function sourceLabel(source) {
  return ({
    windowsUpdate: "Windows Update",
    microsoftCatalog: "Microsoft Update Catalog",
    amd: "AMD",
    nvidia: "NVIDIA",
    intel: "Intel",
    dell: "Dell",
    lenovo: "Lenovo",
    hp: "HP",
  })[source];
}

/** @param {import('./types').CompatibilityState} state */
export function compatibilityLabel(state) {
  return state === "compatible"
    ? "Compatible"
    : state === "needsReview"
      ? "Needs package review"
      : "Rejected";
}

/** @param {import('./types').RecommendationState} state */
export function recommendationLabel(state) {
  return ({
    recommended: "Recommended",
    optional: "Optional",
    current: "Current",
    missing: "Missing",
    notRecommended: "Not recommended",
  })[state];
}

/** @param {import('./types').SourceHealthState} state */
export function sourceStateLabel(state) {
  return state === "available" ? "Available" : state === "skipped" ? "Skipped" : "Unavailable";
}

/** @param {import('./types').DriverCandidate} candidate */
export function formatCandidateDate(candidate) {
  if (candidate.publicationDate) return candidate.publicationDate;
  return candidate.driverDate
    ? new Date(candidate.driverDate * 1000).toLocaleDateString()
    : "Date not reported";
}

/** @param {number | null} bytes */
export function formatBytes(bytes) {
  if (bytes === null) return "Size not reported";
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(unit > 1 ? 1 : 0)} ${units[unit]}`;
}
