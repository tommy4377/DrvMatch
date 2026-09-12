export type NavigationSection = "drivers" | "history" | "settings";
export type ThemePreference = "system" | "light" | "dark";
export type DetailTab = "overview" | "technical";
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

export interface AppearanceSettings {
  theme: ThemePreference;
  acrylic: boolean;
}
