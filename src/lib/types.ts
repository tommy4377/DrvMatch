export type NavigationSection = "drivers" | "history" | "settings";
export type ThemePreference = "system" | "light" | "dark";
export type DetailTab = "overview" | "technical";

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
}

export interface AppearanceSettings {
  theme: ThemePreference;
  acrylic: boolean;
}
