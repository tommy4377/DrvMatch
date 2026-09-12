export type NavigationSection = "drivers" | "history" | "settings";
export type ThemePreference = "system" | "light" | "dark";
export type DetailTab = "overview" | "candidates" | "technical";
export type DriverFilter = "all" | "problem" | "missing" | "generic";
export type DeviceCondition = "current" | "missing" | "problem";
export type SignatureStatus = "whql" | "inbox" | "authenticode" | "signedUnclassified" | "unsigned" | "unknown";

export interface Device {
  instanceId: string;
  friendlyName: string;
  description: string;
  manufacturer: string | null;
  className: string | null;
  classGuid: string | null;
  hardwareIds: string[];
  compatibleIds: string[];
  present: boolean;
  problemCode: number | null;
  problemStatus: number | null;
  condition: DeviceCondition;
  installedDriver: InstalledDriver | null;
}

export interface InstalledDriver {
  description: string | null;
  provider: string | null;
  version: string | null;
  driverDate: number | null;
  infPath: string | null;
  publishedInfName: string | null;
  infSection: string | null;
  matchingId: string | null;
  driverKey: string | null;
  driverRank: number | null;
  signer: string | null;
  catalogFile: string | null;
  signature: SignatureStatus;
  infSignatureVerified: boolean;
  genericMicrosoft: boolean;
}

export interface ScanSummary {
  id: number;
  scannedAt: number;
  deviceCount: number;
  problemCount: number;
  missingCount: number;
  genericCount: number;
}

export interface InventorySnapshot {
  summary: ScanSummary;
  devices: Device[];
}

export type DriverSourceKind = "windowsUpdate" | "microsoftCatalog";
export type SourceHealthState = "available" | "failed" | "skipped";
export type CompatibilityState = "compatible" | "needsReview" | "incompatible";
export type MatchKind = "exactHardwareId" | "compatibleId" | "windowsApplicable";
export type RecommendationState = "recommended" | "optional" | "current" | "missing" | "notRecommended";

export interface SourceHealth {
  source: DriverSourceKind;
  state: SourceHealthState;
  checkedAt: number;
  cached: boolean;
  candidateCount: number;
  message: string | null;
}

export interface CandidateCompatibility {
  state: CompatibilityState;
  matchedId: string | null;
  matchKind: MatchKind | null;
  reasons: string[];
}

export interface DriverCandidate {
  id: string;
  source: DriverSourceKind;
  sourceSpecificId: string;
  displayName: string;
  provider: string | null;
  manufacturer: string | null;
  version: string | null;
  driverDate: number | null;
  publicationDate: string | null;
  className: string | null;
  supportedOs: string[];
  supportedArchitectures: string[];
  hardwareIds: string[];
  compatibleIds: string[];
  downloadUrl: string | null;
  detailsUrl: string | null;
  releaseNotesUrl: string | null;
  releaseChannel: string | null;
  oemModels: string[];
  knownIssues: string[];
  knownRegressions: string[];
  fixedIssues: string[];
  securityRelevant: boolean;
  signature: SignatureStatus;
  packageType: string | null;
  sizeBytes: number | null;
  retrievedAt: number;
  compatibility: CandidateCompatibility;
}

export interface CandidateDiscovery {
  deviceInstanceId: string;
  checkedAt: number;
  candidates: DriverCandidate[];
  rejectedCandidates: DriverCandidate[];
  sources: SourceHealth[];
  recommendation: DriverRecommendation;
}

export interface RankFactor {
  key: string;
  label: string;
  score: number;
  detail: string;
}

export interface RankedCandidate {
  candidate: DriverCandidate;
  score: number;
  state: RecommendationState;
  factors: RankFactor[];
  summary: string;
}

export interface DriverRecommendation {
  state: RecommendationState;
  selectedCandidateId: string | null;
  currentScore: number | null;
  currentFactors: RankFactor[];
  summary: string;
  newestNotBest: string | null;
  rankedCandidates: RankedCandidate[];
}

export interface DownloadResolution {
  source: DriverSourceKind;
  sourceSpecificId: string;
  downloadUrl: string;
  resolvedAt: number;
  cached: boolean;
}

export interface AppearanceSettings {
  theme: ThemePreference;
  acrylic: boolean;
}
