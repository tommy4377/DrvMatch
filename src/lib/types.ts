export type NavigationSection = "drivers" | "history" | "settings";
export type ThemePreference = "system" | "light" | "dark";
export type SettingsCategory = "appearance" | "sources" | "safety" | "advanced" | "about";
export type LogVerbosity = "normal" | "detailed";
export type DetailTab = "overview" | "candidates" | "technical";
export type DriverFilter = "all" | "attention" | "problem" | "missing" | "generic";
export type HardwareCategory = "display" | "network" | "audio" | "bluetooth" | "storage" | "input" | "system" | "usb" | "camera" | "other";
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
  hardwareIdentity: HardwareIdentity | null;
}

export interface HardwareIdentity {
  bus: string;
  vendorId: string | null;
  deviceId: string | null;
  subsystemId: string | null;
  description: string;
}

export interface MachineIdentity {
  manufacturer: string | null;
  model: string | null;
  systemSku: string | null;
  systemFamily: string | null;
  baseboardManufacturer: string | null;
  baseboardProduct: string | null;
  biosVersion: string | null;
  windowsDisplayVersion: string | null;
  windowsBuild: string | null;
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
  machine: MachineIdentity;
  devices: Device[];
}

export type DriverSourceKind = "windowsUpdate" | "microsoftCatalog" | "amd" | "nvidia" | "intel" | "dell" | "lenovo" | "hp";
export type SourceHealthState = "available" | "failed" | "skipped";
export type CompatibilityState = "compatible" | "needsReview" | "incompatible";
export type MatchKind = "exactHardwareId" | "compatibleId" | "windowsApplicable" | "exactOemModel";
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
  alternateSources: DriverSourceKind[];
  displayName: string;
  provider: string | null;
  manufacturer: string | null;
  version: string | null;
  versionIsPackageVersion: boolean;
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
  expectedSha256: string | null;
  packageType: string | null;
  packageGroup: string | null;
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

export interface MachineReview {
  checkedAt: number;
  evaluatedDevices: number;
  recommendedCount: number;
  unresolvedMissingCount: number;
  currentCount: number;
  sources: SourceHealth[];
  findings: CandidateDiscovery[];
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
  enabledSources: DriverSourceKind[];
}

export interface AppSettings {
  theme: ThemePreference;
  acrylic: boolean;
  useWindowsAccent: boolean;
  reduceMotion: boolean;
  enabledSources: DriverSourceKind[];
  enabledOemSources: DriverSourceKind[];
  createRestorePoint: boolean;
  backupCurrentPackage: boolean;
  showExactIds: boolean;
  showInternalScores: boolean;
  logVerbosity: LogVerbosity;
}

export interface CacheStats {
  entryCount: number;
  fileSizeBytes: number;
}

export interface AppInfo {
  version: string;
  repository: string;
}

export type InstallPhase = "idle" | "downloading" | "verifying" | "preparingSafety" | "installing" | "completed" | "failed" | "cancelled";
export type InstallResultState = "succeeded" | "staged" | "failed" | "cancelled" | "rolledBack" | "rollbackFailed";

export interface InstallOptions {
  createRestorePoint: boolean;
  backupCurrentPackage: boolean;
}

export interface InstallSelection {
  deviceInstanceId: string;
  candidate: DriverCandidate;
  resolvedDownloadUrl: string | null;
}

export interface InstallReviewItem {
  deviceInstanceId: string;
  deviceName: string;
  candidateId: string;
  candidateName: string;
  source: DriverSourceKind;
  version: string | null;
  channel: string | null;
  currentVersion: string | null;
  downloadSizeBytes: number | null;
  packageType: string | null;
  restorePointRequested: boolean;
  backupRequested: boolean;
}

export interface InstallReview {
  token: string;
  items: InstallReviewItem[];
  expiresAt: number;
  warning: string | null;
}

export interface InstallStatus {
  operationId: string | null;
  phase: InstallPhase;
  progress: number;
  currentItem: string | null;
  completedItems: number;
  totalItems: number;
  message: string;
  cancellable: boolean;
  rebootRequired: boolean;
}

export interface InstallRecord {
  id: string;
  operationId: string;
  startedAt: number;
  completedAt: number;
  deviceInstanceId: string;
  deviceName: string;
  candidateId: string;
  candidateName: string;
  source: DriverSourceKind;
  previousVersion: string | null;
  installedVersion: string | null;
  previousInf: string | null;
  packageSha256: string | null;
  signatureVerified: boolean;
  restorePointAttempted: boolean;
  restorePointCreated: boolean;
  backupPath: string | null;
  state: InstallResultState;
  message: string;
  rebootRequired: boolean;
  rollbackAvailable: boolean;
}
