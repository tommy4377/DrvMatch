<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import type { AppInfo, AppSettings, CacheStats, CandidateDiscovery, DetailTab, Device, DownloadResolution, DriverCandidate, DriverFilter, DriverSourceKind, HardwareCategory, InstallRecord, InstallReview, InstallStatus, InventorySnapshot, MachineIdentity, MachineReview, NavigationSection, ScanSummary, SettingsCategory, SourceHealth, ThemePreference } from "$lib/types";
  import { compatibilityLabel, deviceCategory, deviceReviewPriority, formatBytes, formatCandidateDate, hardwareCategoryLabel, recommendationLabel, signatureLabel, sourceLabel, sourceStateLabel } from "$lib/presentation.js";
  import "$lib/styles/workbench.css";

  const allSources: DriverSourceKind[] = ["windowsUpdate", "microsoftCatalog", "amd", "nvidia", "intel"];
  const oemSources: DriverSourceKind[] = ["dell", "lenovo", "hp"];
  const defaultSettings: AppSettings = { theme: "system", acrylic: true, useWindowsAccent: false, reduceMotion: false, enabledSources: [...allSources], enabledOemSources: [...oemSources], createRestorePoint: true, backupCurrentPackage: true, showExactIds: false, showInternalScores: false, logVerbosity: "normal" };
  const detailTabs: DetailTab[] = ["overview", "candidates", "technical"];
  type DriverViewMode = "review" | "hardware";
  const categoryOrder: HardwareCategory[] = ["display", "network", "audio", "bluetooth", "storage", "input", "system", "usb", "camera", "other"];

  let section = $state<NavigationSection>("drivers");
  let detailTab = $state<DetailTab>("overview");
  let devices = $state<Device[]>([]);
  let selectedId = $state<string | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let settings = $state<AppSettings>({ ...defaultSettings, enabledSources: [...allSources], enabledOemSources: [...oemSources] });
  let savedSettings = $state<AppSettings>({ ...defaultSettings, enabledSources: [...allSources], enabledOemSources: [...oemSources] });
  let settingsCategory = $state<SettingsCategory>("appearance");
  let settingsLoading = $state(true);
  let settingsSaving = $state(false);
  let settingsError = $state<string | null>(null);
  let sourceHealth = $state<SourceHealth[]>([]);
  let cacheStats = $state<CacheStats>({ entryCount: 0, fileSizeBytes: 0 });
  let activityLog = $state("");
  let appInfo = $state<AppInfo>({ version: "1.1.0", repository: "https://github.com/tommy4377/DrvMatch" });
  let managementBusy = $state<string | null>(null);
  let lastScanned = $state<Date | null>(null);
  let scans = $state<ScanSummary[]>([]);
  let historyError = $state<string | null>(null);
  let activeSummary = $state<ScanSummary | null>(null);
  let machine = $state<MachineIdentity | null>(null);
  let viewingStoredScan = $state(false);
  let search = $state("");
  let filter = $state<DriverFilter>("all");
  let driverView = $state<DriverViewMode>("review");
  let hardwareCategory = $state<HardwareCategory | "all">("all");
  let candidateDiscovery = $state<CandidateDiscovery | null>(null);
  let machineReview = $state<MachineReview | null>(null);
  let machineReviewLoading = $state(false);
  let machineReviewError = $state<string | null>(null);
  let candidateLoading = $state(false);
  let candidateError = $state<string | null>(null);
  let resolvingCandidateId = $state<string | null>(null);
  let resolvedDownloadUrls = $state<Record<string, string>>({});
  let installReview = $state<InstallReview | null>(null);
  let installStatus = $state<InstallStatus>({ operationId: null, phase: "idle", progress: 0, currentItem: null, completedItems: 0, totalItems: 0, message: "Ready", cancellable: false, rebootRequired: false });
  let installHistory = $state<InstallRecord[]>([]);
  let installError = $state<string | null>(null);
  let preparingInstall = $state(false);
  let committingInstall = $state(false);
  let rollingBackId = $state<string | null>(null);
  let installTrigger: HTMLElement | null = null;
  let selectedHistoryId = $state<string | null>(null);

  function categoryIconPath(category: HardwareCategory): string {
    return ({
      display: "M4 5h16v11H4zm5 15h6m-3-4v4",
      network: "M5 12a7 7 0 0 1 14 0M8 15a4 4 0 0 1 8 0m-4 4h.01",
      audio: "M5 10v4h3l4 4V6L8 10zm11-2a6 6 0 0 1 0 8m2-11a10 10 0 0 1 0 14",
      bluetooth: "M12 3v18l5-5-10-8 10-5-5 5",
      storage: "M5 6c0-2 14-2 14 0s-14 2-14 0zm0 0v6c0 2 14 2 14 0V6m-14 6v6c0 2 14 2 14 0v-6",
      input: "M9 4h6a4 4 0 0 1 4 4v8a4 4 0 0 1-4 4H9a4 4 0 0 1-4-4V8a4 4 0 0 1 4-4zm3 0v6",
      system: "M8 8h8v8H8zm-4 3h4m8 0h4M4 15h4m8 0h4M11 4v4m4-4v4m-4 8v4m4-4v4",
      usb: "M12 3v13m0-13-2 2m2-2 2 2m-2 7 4-4m0 0v3m0-3h3m-7 8-4-4m0 0v3m0-3H5m7 4a2 2 0 1 0 0 4 2 2 0 0 0 0-4",
      camera: "M4 8h4l2-2h4l2 2h4v10H4zm8 2a3 3 0 1 0 0 6 3 3 0 0 0 0-6",
      other: "M5 5h14v14H5zm4 4h6v6H9z",
    })[category];
  }

  function openHardware(nextFilter: DriverFilter = "all", category: HardwareCategory | "all" = "all"): void {
    driverView = "hardware";
    filter = nextFilter;
    hardwareCategory = category;
    search = "";
    selectedId = null;
  }

  function openReview(): void {
    driverView = "review";
    search = "";
    filter = "all";
    hardwareCategory = "all";
    selectedId = null;
  }

  async function checkSelectedSources(): Promise<void> {
    detailTab = "candidates";
    await checkCandidates();
  }

  async function checkMachineDrivers(): Promise<void> {
    if (viewingStoredScan || machineReviewLoading) return;
    machineReviewLoading = true;
    machineReviewError = null;
    try {
      machineReview = await invoke<MachineReview>("check_machine_drivers");
      if (machineReview.sources.length) sourceHealth = machineReview.sources;
    } catch (cause) {
      machineReviewError = cause instanceof Error ? cause.message : String(cause);
    } finally {
      machineReviewLoading = false;
    }
  }

  function openMachineFinding(finding: CandidateDiscovery): void {
    const device = devices.find((entry) => entry.instanceId === finding.deviceInstanceId);
    if (!device) return;
    candidateDiscovery = finding;
    selectedId = device.instanceId;
    detailTab = "overview";
  }

  const selectedDevice = $derived(devices.find((device) => device.instanceId === selectedId) ?? null);
  const activeRecommendation = $derived(candidateDiscovery?.deviceInstanceId === selectedId ? candidateDiscovery.recommendation : null);
  const recommendedCandidate = $derived(activeRecommendation?.rankedCandidates.find((entry) => entry.candidate.id === activeRecommendation.selectedCandidateId) ?? null);
  const leadingCandidate = $derived(activeRecommendation?.rankedCandidates.find((entry) => entry.factors.length > 0) ?? null);
  const classCount = $derived(new Set(devices.map((device) => device.className).filter(Boolean)).size);
  const machineLabel = $derived([machine?.manufacturer, machine?.model].filter(Boolean).join(" ") || machine?.systemSku || "This Windows PC");
  const problemDevices = $derived(devices.filter((device) => device.condition === "problem"));
  const missingDevices = $derived(devices.filter((device) => device.condition === "missing"));
  const genericDevices = $derived(devices.filter((device) => device.installedDriver?.genericMicrosoft === true));
  const reviewDevices = $derived(devices.filter((device) => device.condition !== "current" || device.installedDriver?.genericMicrosoft === true).sort((a, b) => deviceReviewPriority(a) - deviceReviewPriority(b) || a.friendlyName.localeCompare(b.friendlyName)));
  const reviewPreviewDevices = $derived(reviewDevices.filter((device) => device.condition !== "current").slice(0, 6));
  const healthyDeviceCount = $derived(Math.max(0, devices.length - problemDevices.length - missingDevices.length));
  const categoryOverview = $derived(categoryOrder.map((category) => ({ category, count: devices.filter((device) => deviceCategory(device) === category).length })).filter((entry) => entry.count > 0));
  const settingsDirty = $derived(JSON.stringify(settings) !== JSON.stringify(savedSettings));
  const selectedHistory = $derived(installHistory.find((record) => record.id === selectedHistoryId) ?? installHistory[0] ?? null);
  const filteredDevices = $derived(devices.filter((device) => {
    const query = search.trim().toLocaleLowerCase();
    const matchesSearch = !query || [device.friendlyName, device.description, device.manufacturer, device.className, device.instanceId, ...device.hardwareIds, ...device.compatibleIds, device.installedDriver?.provider, device.installedDriver?.version, device.installedDriver?.publishedInfName, device.installedDriver?.matchingId]
      .some((value) => value?.toLocaleLowerCase().includes(query));
    const matchesFilter = filter === "all"
      || (filter === "attention" && device.condition !== "current")
      || (filter === "problem" && device.condition === "problem")
      || (filter === "missing" && device.condition === "missing")
      || (filter === "generic" && device.installedDriver?.genericMicrosoft === true);
    const matchesCategory = hardwareCategory === "all" || deviceCategory(device) === hardwareCategory;
    return matchesSearch && matchesFilter && matchesCategory;
  }));
  const groupedHardware = $derived(categoryOrder.map((category) => ({ category, devices: filteredDevices.filter((device) => deviceCategory(device) === category) })).filter((group) => group.devices.length > 0));
  const deviceTabStopId = $derived(
    filteredDevices.some((device) => device.instanceId === selectedId)
      ? selectedId
      : filteredDevices[0]?.instanceId ?? null,
  );

  function iconPath(name: NavigationSection | "refresh" | "close" | "minimize" | "device") {
    return {
      drivers: "M4 4h16v5H4zm0 7h16v9H4zm3 3v3m3-3v3",
      history: "M12 7v5l3 2m6-2a9 9 0 1 1-3-6.7M18 2v4h4",
      settings: "M12 15.5a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7m7.4-3.5.1-1.5 2-1.5-2-3.4-2.5 1-1.3-.8-.3-3H9.3l-.3 3-1.3.8-2.5-1-2 3.4 2 1.5.1 1.5-2 1.5 2 3.4 2.5-1 1.3.8.3 3h3.9l.3-3 1.3-.8 2.5 1 2-3.4z",
      refresh: "M20 11a8 8 0 1 0-2.3 5.7M20 4v7h-7",
      close: "m7 7 10 10M17 7 7 17",
      minimize: "M6 12h12",
      device: "M5 4h14v12H5zm4 16h6m-3-4v4",
    }[name];
  }

  function applyTheme(preference: ThemePreference): void {
    const dark = preference === "dark" || (preference === "system" && matchMedia("(prefers-color-scheme: dark)").matches);
    document.documentElement.dataset.theme = dark ? "dark" : "light";
  }

  function updateTheme(value: ThemePreference): void {
    settings.theme = value;
    applyTheme(value);
  }

  function updateSource(source: DriverSourceKind, enabled: boolean, oem = false): void {
    const current = oem ? settings.enabledOemSources : settings.enabledSources;
    const other = oem ? settings.enabledSources : settings.enabledOemSources;
    if (!enabled && current.length + other.length === 1) {
      settingsError = "At least one trusted driver source must remain enabled.";
      return;
    }
    const next = enabled
      ? [...new Set([...current, source])]
      : current.filter((entry) => entry !== source);
    if (oem) settings.enabledOemSources = next;
    else settings.enabledSources = next;
    settingsError = null;
    candidateDiscovery = null;
  }

  function navigate(item: NavigationSection): void {
    if (section === "settings" && item !== "settings" && settingsDirty) {
      settingsError = "Save or discard settings changes before leaving Settings.";
      return;
    }
    section = item;
  }

  async function updateAcrylic(enabled: boolean): Promise<void> {
    const previous = settings.acrylic;
    settings.acrylic = enabled;
    document.documentElement.dataset.material = enabled ? "acrylic" : "solid";
    try {
      if (isTauri()) await invoke("set_acrylic", { enabled });
    } catch (cause) {
      settings.acrylic = previous;
      document.documentElement.dataset.material = previous ? "acrylic" : "solid";
      settingsError = cause instanceof Error ? cause.message : String(cause);
    }
  }

  async function scanDevices(): Promise<void> {
    loading = true;
    error = null;
    try {
      if (!isTauri()) throw new Error("Device inventory is available in the DrvMatch desktop app.");
      const snapshot = await invoke<InventorySnapshot>("scan_inventory");
      devices = snapshot.devices;
      activeSummary = snapshot.summary;
      machine = snapshot.machine;
      lastScanned = new Date(snapshot.summary.scannedAt * 1000);
      viewingStoredScan = false;
      candidateDiscovery = null;
      machineReview = null;
      machineReviewError = null;
      candidateError = null;
      resolvedDownloadUrls = {};
      await refreshHistory();
      if (selectedId && !devices.some((device) => device.instanceId === selectedId)) selectedId = null;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
      devices = [];
      machine = null;
      selectedId = null;
    } finally {
      loading = false;
    }
  }

  function selectDevice(device: Device): void {
    if (selectedId !== device.instanceId) {
      candidateDiscovery = null;
      candidateError = null;
      resolvedDownloadUrls = {};
    }
    selectedId = device.instanceId;
    detailTab = "overview";
  }

  function handleDeviceKeydown(event: KeyboardEvent, device: Device): void {
    const current = filteredDevices.findIndex((entry) => entry.instanceId === device.instanceId);
    if (current < 0) return;
    let next = current;
    if (event.key === "ArrowDown") next = Math.min(current + 1, filteredDevices.length - 1);
    else if (event.key === "ArrowUp") next = Math.max(current - 1, 0);
    else if (event.key === "Home") next = 0;
    else if (event.key === "End") next = filteredDevices.length - 1;
    else return;
    event.preventDefault();
    const target = filteredDevices[next];
    if (!target) return;
    selectDevice(target);
    requestAnimationFrame(() => {
      document.querySelector<HTMLElement>(`[data-device-index="${next}"]`)?.focus();
    });
  }

  function selectDetailTab(tab: DetailTab): void {
    detailTab = tab;
  }

  async function checkCandidates(): Promise<void> {
    if (!selectedDevice || viewingStoredScan) return;
    const deviceInstanceId = selectedDevice.instanceId;
    candidateLoading = true;
    candidateError = null;
    try {
      const discovery = await invoke<CandidateDiscovery>("discover_candidates", {
        deviceInstanceId,
      });
      if (selectedId === deviceInstanceId) {
        candidateDiscovery = discovery;
        sourceHealth = discovery.sources;
      }
    } catch (cause) {
      candidateError = cause instanceof Error ? cause.message : String(cause);
    } finally {
      candidateLoading = false;
    }
  }

  async function resolveCandidateDownload(candidate: DriverCandidate): Promise<void> {
    resolvingCandidateId = candidate.id;
    candidateError = null;
    try {
      const resolution = await invoke<DownloadResolution>("resolve_catalog_download", {
        updateId: candidate.sourceSpecificId,
      });
      resolvedDownloadUrls[candidate.id] = resolution.downloadUrl;
    } catch (cause) {
      candidateError = cause instanceof Error ? cause.message : String(cause);
    } finally {
      resolvingCandidateId = null;
    }
  }

  async function prepareRecommendedInstall(): Promise<void> {
    if (!selectedDevice || !recommendedCandidate || viewingStoredScan) return;
    installTrigger = document.activeElement as HTMLElement | null;
    preparingInstall = true;
    installError = null;
    try {
      let resolvedDownloadUrl = resolvedDownloadUrls[recommendedCandidate.candidate.id] ?? null;
      if (!recommendedCandidate.candidate.downloadUrl && !resolvedDownloadUrl && recommendedCandidate.candidate.source === "microsoftCatalog") {
        const resolution = await invoke<DownloadResolution>("resolve_catalog_download", { updateId: recommendedCandidate.candidate.sourceSpecificId });
        resolvedDownloadUrl = resolution.downloadUrl;
        resolvedDownloadUrls[recommendedCandidate.candidate.id] = resolution.downloadUrl;
      }
      installReview = await invoke<InstallReview>("prepare_install", {
        selections: [{ deviceInstanceId: selectedDevice.instanceId, candidate: recommendedCandidate.candidate, resolvedDownloadUrl }],
      });
      requestAnimationFrame(() => document.getElementById("confirm-install")?.focus());
    } catch (cause) {
      installError = cause instanceof Error ? cause.message : String(cause);
    } finally {
      preparingInstall = false;
    }
  }

  function accentForeground(accent: string): string {
    const match = accent.trim().match(/^#([0-9a-f]{6})$/i);
    if (!match) return "#ffffff";
    const value = Number.parseInt(match[1], 16);
    const red = (value >> 16) & 0xff;
    const green = (value >> 8) & 0xff;
    const blue = value & 0xff;
    const luminance = (0.2126 * red + 0.7152 * green + 0.0722 * blue) / 255;
    return luminance > 0.62 ? "#171719" : "#ffffff";
  }

  async function applyAccent(enabled: boolean): Promise<void> {
    document.documentElement.style.removeProperty("--accent");
    document.documentElement.style.removeProperty("--accent-hover");
    document.documentElement.style.removeProperty("--focus");
    document.documentElement.style.removeProperty("--text-on-accent");
    if (!enabled || !isTauri()) return;
    try {
      const accent = await invoke<string | null>("get_windows_accent");
      if (accent) {
        document.documentElement.style.setProperty("--accent", accent);
        document.documentElement.style.setProperty("--accent-hover", `color-mix(in srgb, ${accent} 82%, black)`);
        document.documentElement.style.setProperty("--focus", accent);
        document.documentElement.style.setProperty("--text-on-accent", accentForeground(accent));
      }
    } catch (cause) {
      settingsError = cause instanceof Error ? cause.message : String(cause);
    }
  }

  function applyReducedMotion(enabled: boolean): void {
    document.documentElement.dataset.reduceMotion = enabled ? "true" : "false";
  }

  function applyTechnicalVisibility(value: AppSettings): void {
    document.documentElement.dataset.showExactIds = value.showExactIds ? "true" : "false";
    document.documentElement.dataset.showScores = value.showInternalScores ? "true" : "false";
  }

  function cloneSettings(value: AppSettings): AppSettings {
    return { ...value, enabledSources: [...value.enabledSources], enabledOemSources: [...value.enabledOemSources] };
  }

  async function loadManagement(): Promise<void> {
    settingsLoading = true;
    settingsError = null;
    try {
      const [loaded, health, stats, info] = await Promise.all([
        invoke<AppSettings>("get_settings"),
        invoke<SourceHealth[]>("list_source_health"),
        invoke<CacheStats>("get_cache_stats"),
        invoke<AppInfo>("get_app_info"),
      ]);
      settings = cloneSettings(loaded);
      savedSettings = cloneSettings(loaded);
      sourceHealth = health;
      cacheStats = stats;
      appInfo = info;
      applyTheme(settings.theme);
      applyReducedMotion(settings.reduceMotion);
      applyTechnicalVisibility(settings);
      await applyAccent(settings.useWindowsAccent);
      await updateAcrylic(settings.acrylic);
    } catch (cause) {
      settingsError = cause instanceof Error ? cause.message : String(cause);
    } finally { settingsLoading = false; }
  }

  async function saveApplicationSettings(): Promise<void> {
    settingsSaving = true;
    settingsError = null;
    try {
      const saved = await invoke<AppSettings>("save_settings", { values: settings });
      settings = cloneSettings(saved);
      savedSettings = cloneSettings(saved);
      candidateDiscovery = null;
      applyReducedMotion(saved.reduceMotion);
      applyTechnicalVisibility(saved);
      await applyAccent(saved.useWindowsAccent);
      await updateAcrylic(saved.acrylic);
    } catch (cause) {
      settingsError = cause instanceof Error ? cause.message : String(cause);
    } finally { settingsSaving = false; }
  }

  async function discardSettings(): Promise<void> {
    settings = cloneSettings(savedSettings);
    applyTheme(settings.theme);
    applyReducedMotion(settings.reduceMotion);
    applyTechnicalVisibility(settings);
    await applyAccent(settings.useWindowsAccent);
    await updateAcrylic(settings.acrylic);
    settingsError = null;
  }

  async function clearCache(): Promise<void> {
    managementBusy = "cache";
    settingsError = null;
    try { cacheStats = await invoke<CacheStats>("clear_metadata_cache"); }
    catch (cause) { settingsError = cause instanceof Error ? cause.message : String(cause); }
    finally { managementBusy = null; }
  }

  async function loadActivityLog(): Promise<void> {
    managementBusy = "log";
    settingsError = null;
    try { activityLog = await invoke<string>("read_activity_log"); }
    catch (cause) { settingsError = cause instanceof Error ? cause.message : String(cause); }
    finally { managementBusy = null; }
  }

  async function copyActivityLog(): Promise<void> {
    if (!activityLog) await loadActivityLog();
    try { await navigator.clipboard.writeText(activityLog); }
    catch (cause) { settingsError = cause instanceof Error ? cause.message : "Could not copy the activity log."; }
  }

  async function clearActivityLog(): Promise<void> {
    managementBusy = "clear-log";
    try { await invoke("clear_activity_log"); activityLog = ""; }
    catch (cause) { settingsError = cause instanceof Error ? cause.message : String(cause); }
    finally { managementBusy = null; }
  }

  async function copyRepository(): Promise<void> {
    try { await navigator.clipboard.writeText(appInfo.repository); }
    catch (cause) { settingsError = cause instanceof Error ? cause.message : "Could not copy the repository address."; }
  }

  async function commitReviewedInstall(): Promise<void> {
    if (!installReview) return;
    committingInstall = true;
    installError = null;
    try {
      await invoke<string>("commit_install", { token: installReview.token });
      closeInstallReview();
    } catch (cause) {
      installError = cause instanceof Error ? cause.message : String(cause);
    } finally {
      committingInstall = false;
    }
  }

  async function cancelInstall(): Promise<void> {
    try { await invoke("cancel_install"); }
    catch (cause) { installError = cause instanceof Error ? cause.message : String(cause); }
  }

  async function refreshInstallHistory(): Promise<void> {
    if (!isTauri()) return;
    try {
      installHistory = await invoke<InstallRecord[]>("list_install_history");
      if (!installHistory.some((record) => record.id === selectedHistoryId)) selectedHistoryId = installHistory[0]?.id ?? null;
    }
    catch (cause) { historyError = cause instanceof Error ? cause.message : String(cause); }
  }

  async function rollback(record: InstallRecord): Promise<void> {
    rollingBackId = record.id;
    installError = null;
    try {
      await invoke<InstallRecord>("rollback_install", { recordId: record.id });
      await refreshInstallHistory();
      await scanDevices();
    } catch (cause) {
      installError = cause instanceof Error ? cause.message : String(cause);
    } finally { rollingBackId = null; }
  }

  function handleReviewKeydown(event: KeyboardEvent): void {
    if (!installReview) return;
    if (event.key === "Escape" && !committingInstall) { event.preventDefault(); closeInstallReview(); return; }
    if (event.key !== "Tab") return;
    const controls = Array.from(document.querySelectorAll<HTMLButtonElement>(".install-dialog button:not(:disabled)"));
    if (!controls.length) return;
    const current = controls.indexOf(document.activeElement as HTMLButtonElement);
    const next = event.shiftKey ? (current <= 0 ? controls.length - 1 : current - 1) : (current >= controls.length - 1 ? 0 : current + 1);
    event.preventDefault(); controls[next].focus();
  }

  function closeInstallReview(): void {
    installReview = null;
    requestAnimationFrame(() => installTrigger?.focus());
  }

  async function copyDownloadUrl(candidate: DriverCandidate): Promise<void> {
    const url = candidate.downloadUrl ?? resolvedDownloadUrls[candidate.id];
    if (!url) return;
    try {
      await navigator.clipboard.writeText(url);
    } catch (cause) {
      candidateError = cause instanceof Error ? cause.message : "Could not copy the package URL.";
    }
  }

  async function refreshHistory(): Promise<void> {
    if (!isTauri()) return;
    try {
      scans = await invoke<ScanSummary[]>("list_scans");
      historyError = null;
    } catch (cause) {
      historyError = cause instanceof Error ? cause.message : String(cause);
    }
  }

  async function openStoredScan(id: number): Promise<void> {
    loading = true;
    error = null;
    try {
      const snapshot = await invoke<InventorySnapshot>("load_scan", { id });
      devices = snapshot.devices;
      activeSummary = snapshot.summary;
      machine = snapshot.machine;
      lastScanned = new Date(snapshot.summary.scannedAt * 1000);
      viewingStoredScan = true;
      selectedId = null;
      driverView = "review";
      section = "drivers";
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      loading = false;
    }
  }

  function formatDriverDate(seconds: number | null): string {
    return seconds ? new Date(seconds * 1000).toLocaleDateString() : "Not reported";
  }

  function conditionLabel(device: Device): string {
    if (candidateDiscovery?.deviceInstanceId === device.instanceId) return recommendationLabel(candidateDiscovery.recommendation.state);
    if (device.condition === "problem") return `Problem${device.problemCode ? ` · code ${device.problemCode}` : ""}`;
    if (device.condition === "missing") return "Missing";
    return "Current";
  }

  function conditionDescription(device: Device): string {
    if (candidateDiscovery?.deviceInstanceId === device.instanceId) return candidateDiscovery.recommendation.summary;
    if (device.condition === "current") return device.installedDriver ? "Windows reports an installed driver and no device problem." : "Windows reports no device problem. This device does not expose a standalone installed driver package.";
    if (device.condition === "missing") return "No installed driver package was associated with this hardware device.";
    return "Windows reports a problem for this device.";
  }

  function factorScore(score: number): string {
    return score > 0 ? `+${score}` : `${score}`;
  }

  function handleTabKeydown(event: KeyboardEvent, tab: DetailTab): void {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    event.preventDefault();
    const offset = event.key === "ArrowRight" ? 1 : -1;
    const index = (detailTabs.indexOf(tab) + offset + detailTabs.length) % detailTabs.length;
    detailTab = detailTabs[index];
    document.querySelector<HTMLButtonElement>(`[data-detail-tab="${detailTab}"]`)?.focus();
  }

  function handleWindowKeydown(event: KeyboardEvent): void {
    if (installReview) return;
    if (event.key === "F5" && section === "drivers") {
      event.preventDefault();
      void scanDevices();
    } else if (event.ctrlKey && event.key.toLocaleLowerCase() === "f" && section === "drivers") {
      event.preventDefault();
      document.getElementById("device-search")?.focus();
    } else if (event.key === "Escape" && selectedId) {
      selectedId = null;
    }
  }

  function installationIsActive(): boolean {
    return ["downloading", "verifying", "preparingSafety", "installing"].includes(installStatus.phase);
  }

  onMount(() => {
    let stopInstallListener: (() => void) | undefined;
    let stopCloseListener: (() => void) | undefined;
    applyTheme(settings.theme);
    document.documentElement.dataset.material = settings.acrylic ? "acrylic" : "solid";
    applyReducedMotion(settings.reduceMotion);
    const systemTheme = matchMedia("(prefers-color-scheme: dark)");
    const themeListener = () => settings.theme === "system" && applyTheme("system");
    systemTheme.addEventListener("change", themeListener);
    window.addEventListener("keydown", handleWindowKeydown);
    void refreshInstallHistory();
    if (isTauri()) {
      void loadManagement();
      void getCurrentWindow().onCloseRequested((event) => {
        if (installationIsActive()) {
          event.preventDefault();
          installError = installStatus.cancellable ? "Cancel the download before closing DrvMatch." : "Keep DrvMatch open while Windows completes the driver change.";
        } else if (settingsDirty) {
          event.preventDefault();
          settingsError = "Save or discard settings changes before closing DrvMatch.";
          section = "settings";
        }
      }).then((unlisten) => stopCloseListener = unlisten);
      void invoke<InstallStatus>("get_install_status").then((status) => installStatus = status);
      void listen<InstallStatus>("install-status", (event) => {
        installStatus = event.payload;
        if (["completed", "failed", "cancelled"].includes(event.payload.phase)) void refreshInstallHistory();
        if (event.payload.phase === "completed") void scanDevices();
      }).then((unlisten) => stopInstallListener = unlisten);
    } else {
      settingsLoading = false;
    }
    void scanDevices();
    return () => {
      systemTheme.removeEventListener("change", themeListener);
      window.removeEventListener("keydown", handleWindowKeydown);
      stopInstallListener?.();
      stopCloseListener?.();
    };
  });
</script>

<div class="shell">
  <header class="titlebar" data-tauri-drag-region>
    <div class="brand" data-tauri-drag-region><span class="brand-mark"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 6h4v12H7m10-12h-4v12h4M10 12h4" /></svg></span><strong>DrvMatch</strong></div>
    <div class="window-controls">
      <button aria-label="Minimize window" onclick={() => getCurrentWindow().minimize()}><svg viewBox="0 0 24 24" aria-hidden="true"><path d={iconPath("minimize")} /></svg></button>
      <button aria-label="Maximize unavailable because the window has a fixed size" disabled><svg viewBox="0 0 24 24" aria-hidden="true"><rect x="7" y="7" width="10" height="10" /></svg></button>
      <button class="close" aria-label="Close window" onclick={() => getCurrentWindow().close()}><svg viewBox="0 0 24 24" aria-hidden="true"><path d={iconPath("close")} /></svg></button>
    </div>
  </header>

  <nav class="primary-navigation" aria-label="Primary navigation">
    <div class="primary-tabs">
      {#each (["drivers", "history", "settings"] as NavigationSection[]) as item}
        <button aria-current={section === item ? "page" : undefined} onclick={() => navigate(item)}>
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d={iconPath(item)} /></svg>
          <span>{item[0].toUpperCase() + item.slice(1)}</span>
        </button>
      {/each}
    </div>
    <div class="navigation-context" aria-live="polite">
      <span class="context-dot" class:attention={(activeSummary?.problemCount ?? 0) > 0 || (activeSummary?.missingCount ?? 0) > 0}></span>
      <span>{lastScanned ? `Local inventory · ${lastScanned.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}` : "Local inventory"}</span>
    </div>
  </nav>

  <div class="workspace">
    <main class="content" inert={section === "settings" || (section === "history" && installHistory.length) ? true : undefined}>
{#if section === "drivers"}
        <section class="drivers-page">
          <header class="drivers-header">
            <div>
              <span class="page-kicker">Driver workbench</span>
              <h1>Drivers</h1>
              <p>{machineLabel}{devices.length ? ` · ${devices.length} devices` : ""}{lastScanned ? ` · scanned ${lastScanned.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}` : ""}</p>
            </div>
            <div class="drivers-header-actions">
              <div class="scan-context">
                <strong>{viewingStoredScan ? "Stored inventory" : lastScanned ? "Inventory ready" : "Not scanned yet"}</strong>
                <small>{lastScanned ? `Last scan ${lastScanned.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}` : "Scan this PC to begin"}</small>
              </div>
              <button class="primary-inline-action header-check-action" disabled={machineReviewLoading || viewingStoredScan || loading || devices.length === 0} onclick={checkMachineDrivers}>{machineReviewLoading ? "Checking…" : machineReview ? "Check again" : "Check drivers"}</button>
            </div>
          </header>

          <div class="driver-mode-tabs" aria-label="Driver workspace view">
            <button aria-pressed={driverView === "review"} onclick={openReview}>
              <span>Review</span>
              <small>{problemDevices.length + missingDevices.length ? `${problemDevices.length + missingDevices.length} need attention` : "Start here"}</small>
            </button>
            <button aria-pressed={driverView === "hardware"} onclick={() => { driverView = "hardware"; selectedId = null; }}>
              <span>All hardware</span>
              <small>{devices.length ? `${devices.length} devices` : "Inventory"}</small>
            </button>
          </div>

          {#if error}
            <div class="page-message" role="alert"><div class="message-mark error">!</div><span><strong>Device inventory unavailable</strong><small>{error}</small></span><button onclick={scanDevices}>Try again</button></div>
          {:else if loading}
            <div class="page-message" role="status"><span class="spinner"></span><span><strong>Inspecting this PC</strong><small>Reading present Plug and Play devices and installed packages from Windows.</small></span></div>
          {:else if devices.length === 0}
            <div class="page-message"><div class="message-mark">i</div><span><strong>No present devices were returned</strong><small>Run the scan again. DrvMatch never fabricates hardware inventory.</small></span></div>
          {:else if driverView === "review"}
            <div class="review-stage" class:details-open={selectedDevice !== null}>
              <div class="review-scroll">
                {#if viewingStoredScan}
                  <div class="stored-banner" role="status"><span>Stored scan</span><p>You are looking at a previous inventory. Scan the current PC before evaluating or installing drivers.</p><button onclick={scanDevices}>Scan current PC</button></div>
                {/if}

                <section class="machine-board">
                  <div class="machine-heading">
                    <div class="machine-glyph"><svg viewBox="0 0 24 24" aria-hidden="true"><path d={categoryIconPath("system")} /></svg></div>
                    <span><small>This PC</small><strong>{machineLabel}</strong><em>{classCount} hardware classes · {devices.length} present devices</em></span>
                  </div>
                  <div class="machine-verdict" class:attention={problemDevices.length + missingDevices.length > 0}>
                    <div class="verdict-mark">{problemDevices.length + missingDevices.length > 0 ? "!" : "✓"}</div>
                    <span>
                      <strong>{problemDevices.length + missingDevices.length > 0 ? `${problemDevices.length + missingDevices.length} devices need attention` : "No hardware problems found"}</strong>
                      <small>{problemDevices.length + missingDevices.length > 0 ? "Windows reported a missing driver or device problem." : `${healthyDeviceCount} devices report no Plug and Play problem.`}</small>
                    </span>
                  </div>
                  <div class="machine-facts" aria-label="Local inventory summary">
                    <span><strong>{missingDevices.length}</strong><small>Missing</small></span>
                    <span><strong>{problemDevices.length}</strong><small>Problems</small></span>
                    <span><strong>{devices.length}</strong><small>Present</small></span>
                  </div>
                </section>

                <section class="review-panel source-review-panel">
                  <header>
                    <div><span class="section-kicker">Source-backed review</span><h2>{machineReview ? "Needs attention" : "Driver recommendations"}</h2></div>
                    {#if machineReview}<span class="quiet-count">{machineReview.evaluatedDevices} checked</span>{/if}
                  </header>
                  {#if machineReviewError}
                    <div class="inline-error"><strong>Driver check could not finish</strong><span>{machineReviewError}</span></div>
                  {:else if machineReviewLoading}
                    <div class="machine-check-progress" role="status"><span class="spinner"></span><span><strong>Comparing review targets</strong><small>Checking trusted Microsoft, vendor, and applicable OEM sources. Healthy specific drivers are not treated as updates by default.</small></span></div>
                  {:else if machineReview}
                    {@const actionableFindings = machineReview.findings.filter((finding) => finding.recommendation.state === "recommended" || finding.recommendation.state === "missing" || finding.recommendation.state === "optional")}
                    <div class="machine-review-summary"><strong>{machineReview.recommendedCount ? `${machineReview.recommendedCount} recommended change${machineReview.recommendedCount === 1 ? "" : "s"}` : "No recommended changes"}</strong><small>{machineReview.evaluatedDevices} review target{machineReview.evaluatedDevices === 1 ? "" : "s"} checked · {machineReview.unresolvedMissingCount} unresolved missing</small></div>
                    {#if actionableFindings.length}
                      <div class="machine-finding-columns" aria-hidden="true"><span>Device</span><span>Installed</span><span>Recommended</span><span>Source</span><span></span></div>
                      <div class="machine-finding-list">
                        {#each actionableFindings as finding}
                          {@const findingDevice = devices.find((entry) => entry.instanceId === finding.deviceInstanceId)}
                          {@const findingCandidate = finding.recommendation.rankedCandidates.find((entry) => entry.candidate.id === finding.recommendation.selectedCandidateId)?.candidate}
                          <button class:selected={selectedId === finding.deviceInstanceId} onclick={() => openMachineFinding(finding)}>
                            <span class="machine-finding-device"><strong>{findingDevice?.friendlyName ?? finding.deviceInstanceId}</strong><small>{finding.recommendation.summary}</small></span>
                            <span class="machine-finding-version mono">{findingDevice?.installedDriver?.version ?? "None"}</span>
                            <span class="machine-finding-version recommended mono">{findingCandidate?.version ?? recommendationLabel(finding.recommendation.state)}</span>
                            <span class="machine-finding-source">{findingCandidate ? sourceLabel(findingCandidate.source) : "Needs review"}</span>
                            <span class="machine-finding-action">Review ›</span>
                          </button>
                        {/each}
                      </div>
                      <div class="machine-review-rest"><span><strong>Everything else</strong><small>{machineReview.currentCount} checked review target{machineReview.currentCount === 1 ? "" : "s"} keep the current driver.</small></span><button class="text-action" onclick={() => openHardware()}>Open all hardware ›</button></div>
                    {:else}
                      <div class="calm-state compact"><span class="calm-check">✓</span><span><strong>The checked drivers already make sense for this machine.</strong><small>DrvMatch found no source-backed reason to change the review targets.</small></span></div>
                    {/if}
                  {:else}
                    <p class="source-review-copy">Local Windows findings are shown below. Run a driver check when you want DrvMatch to compare only the devices that deserve review against trusted sources.</p>
                  {/if}
                </section>

                {#if !machineReview}
                <section class="review-panel local-findings-panel">
                  <header><div><span class="section-kicker">Local findings</span><h2>What deserves a look</h2></div><button class="text-action" onclick={() => openHardware()}>Browse inventory</button></header>
                  {#if reviewPreviewDevices.length}
                    <div class="review-device-list">
                      {#each reviewPreviewDevices as device}
                        {@const category = deviceCategory(device)}
                        <button class="review-device" onclick={() => selectDevice(device)}>
                          <span class="device-icon"><svg viewBox="0 0 24 24" aria-hidden="true"><path d={categoryIconPath(category)} /></svg></span>
                          <span class="review-device-copy"><strong>{device.friendlyName}</strong><small>{hardwareCategoryLabel(category)} · {device.manufacturer ?? "Manufacturer not reported"}</small></span>
                          <span class="finding-state" class:problem={device.condition === "problem"} class:missing={device.condition === "missing"}>{conditionLabel(device)}</span>
                          <span class="row-arrow" aria-hidden="true">›</span>
                        </button>
                      {/each}
                    </div>
                    {#if problemDevices.length + missingDevices.length > reviewPreviewDevices.length}<button class="panel-footer-action" onclick={() => openHardware("attention")}>Show all devices needing attention</button>{/if}
                  {:else}
                    <div class="calm-state"><span class="calm-check">✓</span><span><strong>No missing or problem devices</strong><small>Windows reports the local hardware inventory as present and working.</small></span></div>
                  {/if}

                  {#if genericDevices.length}
                    <button class="generic-summary" onclick={() => openHardware("generic")}>
                      <span class="generic-mark">G</span>
                      <span><strong>{genericDevices.length} generic Microsoft driver{genericDevices.length === 1 ? "" : "s"}</strong><small>Often normal for class devices. Review them only when a more specific package may add value.</small></span>
                      <span class="row-arrow" aria-hidden="true">›</span>
                    </button>
                  {/if}
                </section>
                {/if}

                <section class="review-panel hardware-overview">
                  <header><div><span class="section-kicker">Inventory map</span><h2>Browse by hardware</h2></div><span class="quiet-count">{devices.length} total</span></header>
                  <div class="category-grid">
                    {#each categoryOverview as entry}
                      <button onclick={() => openHardware("all", entry.category)}>
                        <span class="category-icon"><svg viewBox="0 0 24 24" aria-hidden="true"><path d={categoryIconPath(entry.category)} /></svg></span>
                        <span><strong>{hardwareCategoryLabel(entry.category)}</strong><small>{entry.count} device{entry.count === 1 ? "" : "s"}</small></span>
                        <span class="row-arrow" aria-hidden="true">›</span>
                      </button>
                    {/each}
                  </div>
                </section>

                <p class="review-footnote">DrvMatch keeps the complete Windows inventory one step away, but the Review view stays focused on decisions instead of raw device enumeration.</p>
              </div>

              {#if selectedDevice}
                <aside class="details-pane" aria-label="Device details">
                  <header><div><h2>{selectedDevice.friendlyName}</h2><p>{selectedDevice.className ?? "Other device"}</p></div><button class="icon-button" aria-label="Close device details" onclick={() => selectedId = null}><svg viewBox="0 0 24 24" aria-hidden="true"><path d={iconPath("close")} /></svg></button></header>
                  <div class="tabs" role="tablist" aria-label="Device information">{#each detailTabs as tab}<button id={`detail-tab-${tab}`} role="tab" data-detail-tab={tab} aria-controls="device-detail-panel" aria-selected={detailTab === tab} tabindex={detailTab === tab ? 0 : -1} onclick={() => selectDetailTab(tab)} onkeydown={(event) => handleTabKeydown(event, tab)}>{tab[0].toUpperCase() + tab.slice(1)}</button>{/each}</div>
                  <div id="device-detail-panel" class="details-content" role="tabpanel" aria-labelledby={`detail-tab-${detailTab}`}>
                    {#if detailTab === "overview"}
                      <div class="status-line"><span class="status-dot" class:problem={selectedDevice.condition === "problem"} class:missing={selectedDevice.condition === "missing"}></span><span><strong>{conditionLabel(selectedDevice)}</strong><small>{conditionDescription(selectedDevice)}</small></span></div>
                      <dl><div><dt>Description</dt><dd>{selectedDevice.description}</dd></div><div><dt>Manufacturer</dt><dd>{selectedDevice.manufacturer ?? "Not reported"}</dd></div><div><dt>Device class</dt><dd>{selectedDevice.className ?? "Not reported"}</dd></div></dl>
                      <section class="driver-summary"><h3>Installed driver</h3>{#if selectedDevice.installedDriver}<dl><div><dt>Provider</dt><dd>{selectedDevice.installedDriver.provider ?? "Not reported"}</dd></div><div><dt>Version</dt><dd class="mono">{selectedDevice.installedDriver.version ?? "Not reported"}</dd></div><div><dt>INF driver date</dt><dd>{formatDriverDate(selectedDevice.installedDriver.driverDate)}</dd></div><div><dt>Signature</dt><dd>{signatureLabel(selectedDevice.installedDriver.signature)}{selectedDevice.installedDriver.signer ? ` · ${selectedDevice.installedDriver.signer}` : ""}</dd></div></dl>{:else}<p>Windows did not associate an installed package with this hardware device.</p>{/if}</section>
                      {#if activeRecommendation}
                        <section class="recommendation-panel" class:recommended={activeRecommendation.state === "recommended"} class:current={activeRecommendation.state === "current"}>
                          <span>{recommendationLabel(activeRecommendation.state)}</span>
                          <strong>{recommendedCandidate?.candidate.version ?? (activeRecommendation.state === "current" ? selectedDevice.installedDriver?.version ?? "Installed driver" : "Driver required")}</strong>
                          <p>{activeRecommendation.summary}</p>
                          {#if (activeRecommendation.state === "recommended" || activeRecommendation.state === "missing") && recommendedCandidate}
                            <button class="install-action" disabled={preparingInstall || ["downloading", "verifying", "preparingSafety", "installing"].includes(installStatus.phase)} onclick={prepareRecommendedInstall}>{preparingInstall ? "Preparing review" : "Review installation"}</button>
                          {/if}
                        </section>
                        {#if activeRecommendation.state === "missing" && recommendedCandidate}
                          <section class="driver-summary"><h3>Identified hardware</h3><dl><div><dt>Best package match</dt><dd>{recommendedCandidate.candidate.displayName}</dd></div><div><dt>Provider</dt><dd>{recommendedCandidate.candidate.provider ?? recommendedCandidate.candidate.manufacturer ?? "Not reported"}</dd></div><div><dt>Matched ID</dt><dd class="mono">{recommendedCandidate.candidate.compatibility.matchedId ?? selectedDevice.hardwareIds[0] ?? "Not reported"}</dd></div><div><dt>Source</dt><dd>{sourceLabel(recommendedCandidate.candidate.source)}</dd></div></dl></section>
                        {/if}
                        {#if installError}<div class="inline-error" role="alert"><strong>Installation issue</strong><span>{installError}</span></div>{/if}
                        <section class="why-driver"><h3>Why this driver?</h3><ul>{#each (recommendedCandidate?.factors ?? activeRecommendation.currentFactors) as factor}<li class:negative={factor.score < 0}><span>{factor.score < 0 ? "!" : "✓"}</span><span><strong>{factor.label}</strong><small>{factor.detail}</small></span></li>{/each}</ul></section>
                        {#if activeRecommendation.newestNotBest}<div class="quiet-note"><strong>Newest is not always best</strong><p>{activeRecommendation.newestNotBest}</p></div>{/if}
                      {:else if selectedDevice.installedDriver?.genericMicrosoft}<div class="quiet-note actionable"><strong>Generic Microsoft driver</strong><p>This may be completely normal. DrvMatch can compare trusted Microsoft, vendor, and applicable OEM packages before suggesting any change.</p><button class="candidate-action" disabled={candidateLoading || viewingStoredScan} onclick={checkSelectedSources}>{candidateLoading ? "Checking sources" : "Compare trusted drivers"}</button></div>{:else}<div class="quiet-note actionable"><strong>Not evaluated yet</strong><p>Local inventory says the device is working. Check trusted sources only if you want DrvMatch to compare alternatives.</p><button class="candidate-action" disabled={candidateLoading || viewingStoredScan} onclick={checkSelectedSources}>{candidateLoading ? "Checking sources" : "Check this device"}</button></div>{/if}
                    {:else if detailTab === "candidates"}
                      <div class="candidate-intro"><strong>DriverRank evaluation</strong><p>Checks enabled Microsoft, component-vendor, and applicable OEM sources, ranks proven compatible packages, and compares the best result with the installed driver. No package is downloaded or installed.</p><button class="candidate-action" disabled={candidateLoading || viewingStoredScan} onclick={checkCandidates}>{candidateLoading ? "Evaluating sources" : candidateDiscovery ? "Evaluate again" : "Check and rank candidates"}</button>{#if viewingStoredScan}<small>Scan the current machine before checking sources for a stored inventory.</small>{/if}</div>
                      {#if candidateError}<div class="inline-error" role="alert"><strong>Source check issue</strong><span>{candidateError}</span></div>{/if}
                      {#if candidateLoading}
                        <div class="candidate-loading" role="status"><span class="spinner"></span><span><strong>Checking trusted sources</strong><small>Windows Update, Catalog, component vendors, and an applicable Dell/Lenovo/HP catalog may take a moment.</small></span></div>
                      {:else if candidateDiscovery}
                        <section class="source-health" aria-label="Driver source health"><h3>Sources</h3>{#each candidateDiscovery.sources as source}<div><span class="source-dot" class:failed={source.state === "failed"} class:skipped={source.state === "skipped"}></span><span><strong>{sourceLabel(source.source)}</strong><small>{source.message ?? `${source.candidateCount} candidates returned${source.cached ? " · cached metadata" : ""}`}</small></span><span>{sourceStateLabel(source.state)}</span></div>{/each}</section>
                        <section class="candidate-list"><h3>Ranked candidates <span>{candidateDiscovery.recommendation.rankedCandidates.length}</span></h3>{#each candidateDiscovery.recommendation.rankedCandidates as ranked (ranked.candidate.id)}{@const candidate = ranked.candidate}<article class="candidate-row" class:not-recommended={ranked.state === "notRecommended"}><div class="candidate-heading"><span><strong>{candidate.displayName}</strong><small>{sourceLabel(candidate.source)} · {compatibilityLabel(candidate.compatibility.state)}{candidate.alternateSources.length ? ` · Also available from ${candidate.alternateSources.map(sourceLabel).join(", ")}` : ""}</small></span><span class="candidate-rank" class:recommended={ranked.state === "recommended"} class:review={ranked.state === "notRecommended"}><strong>{recommendationLabel(ranked.state)}</strong><small>{candidate.versionIsPackageVersion ? "Package " : ""}{candidate.version ?? "Version not reported"}</small></span></div><p class="candidate-summary">{ranked.summary}</p><dl><div><dt>Provider</dt><dd>{candidate.provider ?? candidate.manufacturer ?? "Not reported"}</dd></div><div><dt>Date</dt><dd>{formatCandidateDate(candidate)}</dd></div><div><dt>Channel</dt><dd>{candidate.releaseChannel ?? "Not reported"}</dd></div><div><dt>Package</dt><dd>{candidate.packageGroup ?? candidate.packageType ?? "Driver package"} · {formatBytes(candidate.sizeBytes)}</dd></div></dl>{#if ranked.factors.length}<ul class="evidence">{#each ranked.factors.slice(0, 4) as factor}<li>{factor.detail}</li>{/each}</ul>{:else}<ul class="evidence">{#each candidate.compatibility.reasons as reason}<li>{reason}</li>{/each}</ul>{/if}{#if candidate.supportedOs.length}<p class="candidate-products">Products: {candidate.supportedOs.join(", ")}</p>{/if}<div class="candidate-actions">{#if candidate.downloadUrl || resolvedDownloadUrls[candidate.id]}<button onclick={() => copyDownloadUrl(candidate)}>Copy package URL</button>{:else if candidate.source === "microsoftCatalog"}<button disabled={resolvingCandidateId === candidate.id} onclick={() => resolveCandidateDownload(candidate)}>{resolvingCandidateId === candidate.id ? "Resolving" : "Resolve package metadata"}</button>{/if}{#if candidate.releaseNotesUrl}<button onclick={() => navigator.clipboard.writeText(candidate.releaseNotesUrl ?? "")}>Copy release notes URL</button>{/if}</div></article>{:else}<div class="candidate-empty"><strong>No candidates returned</strong><p>The installed driver can remain Current when sources have no suitable alternative.</p></div>{/each}</section>
                        {#if candidateDiscovery.recommendation.newestNotBest}<div class="quiet-note"><strong>Newest is not the best match</strong><p>{candidateDiscovery.recommendation.newestNotBest}</p></div>{/if}
                      {/if}
                    {:else}
                      <dl class="technical"><div><dt>Device instance ID</dt><dd>{selectedDevice.instanceId}</dd></div><div><dt>Class GUID</dt><dd>{selectedDevice.classGuid ?? "Not reported"}</dd></div><div><dt>Problem code</dt><dd>{selectedDevice.problemCode ?? "None"}</dd></div><div><dt>Problem status</dt><dd>{selectedDevice.problemStatus === null ? "None" : `0x${(selectedDevice.problemStatus >>> 0).toString(16).padStart(8, "0")}`}</dd></div></dl>
                      {#if selectedDevice.hardwareIdentity}<section class="id-section"><h3>Detected hardware identity</h3><dl class="technical"><div><dt>Bus</dt><dd>{selectedDevice.hardwareIdentity.bus}</dd></div><div><dt>Vendor ID</dt><dd class="mono">{selectedDevice.hardwareIdentity.vendorId ?? "Not reported"}</dd></div><div><dt>Device ID</dt><dd class="mono">{selectedDevice.hardwareIdentity.deviceId ?? "Not reported"}</dd></div><div><dt>Subsystem / product</dt><dd class="mono">{selectedDevice.hardwareIdentity.subsystemId ?? "Not reported"}</dd></div><div><dt>Detected identity</dt><dd>{selectedDevice.hardwareIdentity.description}</dd></div></dl></section>{/if}
                      {#if machine}<section class="id-section"><h3>Machine applicability</h3><dl class="technical"><div><dt>System</dt><dd>{machineLabel}</dd></div><div><dt>System SKU</dt><dd class="mono">{machine.systemSku ?? "Not reported"}</dd></div><div><dt>Baseboard</dt><dd>{[machine.baseboardManufacturer, machine.baseboardProduct].filter(Boolean).join(" ") || "Not reported"}</dd></div><div><dt>BIOS</dt><dd>{machine.biosVersion ?? "Not reported"}</dd></div><div><dt>Windows release</dt><dd>{machine.windowsDisplayVersion ?? "Not reported"}</dd></div><div><dt>Windows build</dt><dd class="mono">{machine.windowsBuild ?? "Not reported"}</dd></div></dl></section>{/if}
                      {#if selectedDevice.installedDriver}<section class="id-section"><h3>Installed package</h3><dl class="technical"><div><dt>Published INF</dt><dd>{selectedDevice.installedDriver.publishedInfName ?? "Not reported"}</dd></div><div><dt>INF path</dt><dd>{selectedDevice.installedDriver.infPath ?? "Not reported"}</dd></div><div><dt>INF section</dt><dd>{selectedDevice.installedDriver.infSection ?? "Not reported"}</dd></div><div><dt>Matching ID</dt><dd>{selectedDevice.installedDriver.matchingId ?? "Not reported"}</dd></div><div><dt>Driver key</dt><dd>{selectedDevice.installedDriver.driverKey ?? "Not reported"}</dd></div><div><dt>Windows driver rank</dt><dd>{selectedDevice.installedDriver.driverRank === null ? "Not reported" : `0x${selectedDevice.installedDriver.driverRank.toString(16).padStart(8, "0")}`}</dd></div><div><dt>Signature class</dt><dd>{signatureLabel(selectedDevice.installedDriver.signature)}</dd></div><div><dt>INF signature verified</dt><dd>{selectedDevice.installedDriver.infSignatureVerified ? "Yes" : "Not verified"}</dd></div><div><dt>Signer</dt><dd>{selectedDevice.installedDriver.signer ?? "Not reported"}</dd></div><div><dt>Catalog / store identity</dt><dd>{selectedDevice.installedDriver.catalogFile ?? "Not reported"}</dd></div></dl></section>{/if}
                      {#if activeRecommendation}<section class="id-section ranking-technical"><h3>DriverRank factors</h3><dl class="technical"><div><dt>Decision</dt><dd>{recommendationLabel(activeRecommendation.state)}</dd></div><div><dt>Installed score</dt><dd>{activeRecommendation.currentScore ?? "No installed driver"}</dd></div>{#if leadingCandidate}<div><dt>Leading candidate score</dt><dd>{leadingCandidate.score}</dd></div>{/if}</dl>{#each (recommendedCandidate?.factors ?? (activeRecommendation.state === "current" ? activeRecommendation.currentFactors : leadingCandidate?.factors ?? [])) as factor}<div class="factor-row"><span><strong>{factor.label}</strong><small>{factor.detail}</small></span><span class:negative={factor.score < 0}>{factorScore(factor.score)}</span></div>{/each}<small class="score-disclaimer">Internal scores compare decomposed evidence; they are not a confidence percentage.</small></section>{/if}
                      <section class="id-section technical-identifiers"><h3>Hardware IDs</h3>{#if selectedDevice.hardwareIds.length}<ul>{#each selectedDevice.hardwareIds as id}<li>{id}</li>{/each}</ul>{:else}<p>Windows did not expose hardware IDs for this device.</p>{/if}</section>
                      <section class="id-section technical-identifiers"><h3>Compatible IDs</h3>{#if selectedDevice.compatibleIds.length}<ul>{#each selectedDevice.compatibleIds as id}<li>{id}</li>{/each}</ul>{:else}<p>Windows did not expose compatible IDs for this device.</p>{/if}</section>
                    {/if}
                  </div>
                </aside>
              {/if}
            </div>
          {:else}
            <div class="hardware-toolbar">
              <button class="back-review" onclick={openReview}><span aria-hidden="true">‹</span> Review</button>
              <label class="search-box" aria-label="Search hardware inventory"><svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="10.5" cy="10.5" r="6.5"></circle><path d="m15.5 15.5 4 4"></path></svg><input id="device-search" bind:value={search} placeholder="Search hardware, drivers, or IDs" /></label>
              <label class="compact-select"><span>Category</span><select bind:value={hardwareCategory} aria-label="Filter by hardware category"><option value="all">All categories</option>{#each categoryOverview as entry}<option value={entry.category}>{hardwareCategoryLabel(entry.category)} ({entry.count})</option>{/each}</select></label>
              <label class="compact-select"><span>Status</span><select bind:value={filter} aria-label="Filter hardware status"><option value="all">All devices</option><option value="attention">Needs attention</option><option value="problem">Problems</option><option value="missing">Missing driver</option><option value="generic">Generic Microsoft</option></select></label>
            </div>

            <div class="hardware-stage" class:details-open={selectedDevice !== null}>
              <div class="hardware-list" aria-label="Detected hardware">
                {#each groupedHardware as group}
                  <div class="hardware-group-heading"><span class="category-icon"><svg viewBox="0 0 24 24" aria-hidden="true"><path d={categoryIconPath(group.category)} /></svg></span><strong>{hardwareCategoryLabel(group.category)}</strong><small>{group.devices.length}</small></div>
                  {#each group.devices as device (device.instanceId)}
                    {@const flatIndex = filteredDevices.findIndex((entry) => entry.instanceId === device.instanceId)}
                    {@const category = deviceCategory(device)}
                    <button class="device-row" class:selected={selectedId === device.instanceId} aria-current={selectedId === device.instanceId ? "true" : undefined} tabindex={deviceTabStopId === device.instanceId ? 0 : -1} data-device-index={flatIndex} onclick={() => selectDevice(device)} onkeydown={(event) => handleDeviceKeydown(event, device)}>
                      <span class="device-identity"><span class="device-icon"><svg viewBox="0 0 24 24" aria-hidden="true"><path d={categoryIconPath(category)} /></svg></span><span><strong>{device.friendlyName}</strong><small>{device.manufacturer ?? hardwareCategoryLabel(category)} · {device.className ?? "Other"}</small></span></span>
                      <span class="condition" class:problem={device.condition === "problem"} class:missing={device.condition === "missing"}><span></span>{conditionLabel(device)}</span>
                      <span class="driver-cell"><strong>{device.installedDriver?.version ?? "No standalone package"}</strong><small>{device.installedDriver?.provider ?? device.installedDriver?.publishedInfName ?? "Windows-managed device"}</small></span>
                    </button>
                  {/each}
                {:else}
                  <div class="filtered-empty"><strong>No matching hardware</strong><span>Change the search, category, or status filter.</span></div>
                {/each}
              </div>

              {#if selectedDevice}
                <aside class="details-pane" aria-label="Device details">
                  <header><div><h2>{selectedDevice.friendlyName}</h2><p>{selectedDevice.className ?? "Other device"}</p></div><button class="icon-button" aria-label="Close device details" onclick={() => selectedId = null}><svg viewBox="0 0 24 24" aria-hidden="true"><path d={iconPath("close")} /></svg></button></header>
                  <div class="tabs" role="tablist" aria-label="Device information">{#each detailTabs as tab}<button id={`detail-tab-${tab}`} role="tab" data-detail-tab={tab} aria-controls="device-detail-panel" aria-selected={detailTab === tab} tabindex={detailTab === tab ? 0 : -1} onclick={() => selectDetailTab(tab)} onkeydown={(event) => handleTabKeydown(event, tab)}>{tab[0].toUpperCase() + tab.slice(1)}</button>{/each}</div>
                  <div id="device-detail-panel" class="details-content" role="tabpanel" aria-labelledby={`detail-tab-${detailTab}`}>
                    {#if detailTab === "overview"}
                      <div class="status-line"><span class="status-dot" class:problem={selectedDevice.condition === "problem"} class:missing={selectedDevice.condition === "missing"}></span><span><strong>{conditionLabel(selectedDevice)}</strong><small>{conditionDescription(selectedDevice)}</small></span></div>
                      <dl><div><dt>Description</dt><dd>{selectedDevice.description}</dd></div><div><dt>Manufacturer</dt><dd>{selectedDevice.manufacturer ?? "Not reported"}</dd></div><div><dt>Device class</dt><dd>{selectedDevice.className ?? "Not reported"}</dd></div></dl>
                      <section class="driver-summary"><h3>Installed driver</h3>{#if selectedDevice.installedDriver}<dl><div><dt>Provider</dt><dd>{selectedDevice.installedDriver.provider ?? "Not reported"}</dd></div><div><dt>Version</dt><dd class="mono">{selectedDevice.installedDriver.version ?? "Not reported"}</dd></div><div><dt>INF driver date</dt><dd>{formatDriverDate(selectedDevice.installedDriver.driverDate)}</dd></div><div><dt>Signature</dt><dd>{signatureLabel(selectedDevice.installedDriver.signature)}{selectedDevice.installedDriver.signer ? ` · ${selectedDevice.installedDriver.signer}` : ""}</dd></div></dl>{:else}<p>Windows did not associate an installed package with this hardware device.</p>{/if}</section>
                      {#if activeRecommendation}
                        <section class="recommendation-panel" class:recommended={activeRecommendation.state === "recommended"} class:current={activeRecommendation.state === "current"}>
                          <span>{recommendationLabel(activeRecommendation.state)}</span>
                          <strong>{recommendedCandidate?.candidate.version ?? (activeRecommendation.state === "current" ? selectedDevice.installedDriver?.version ?? "Installed driver" : "Driver required")}</strong>
                          <p>{activeRecommendation.summary}</p>
                          {#if (activeRecommendation.state === "recommended" || activeRecommendation.state === "missing") && recommendedCandidate}
                            <button class="install-action" disabled={preparingInstall || ["downloading", "verifying", "preparingSafety", "installing"].includes(installStatus.phase)} onclick={prepareRecommendedInstall}>{preparingInstall ? "Preparing review" : "Review installation"}</button>
                          {/if}
                        </section>
                        {#if activeRecommendation.state === "missing" && recommendedCandidate}
                          <section class="driver-summary"><h3>Identified hardware</h3><dl><div><dt>Best package match</dt><dd>{recommendedCandidate.candidate.displayName}</dd></div><div><dt>Provider</dt><dd>{recommendedCandidate.candidate.provider ?? recommendedCandidate.candidate.manufacturer ?? "Not reported"}</dd></div><div><dt>Matched ID</dt><dd class="mono">{recommendedCandidate.candidate.compatibility.matchedId ?? selectedDevice.hardwareIds[0] ?? "Not reported"}</dd></div><div><dt>Source</dt><dd>{sourceLabel(recommendedCandidate.candidate.source)}</dd></div></dl></section>
                        {/if}
                        {#if installError}<div class="inline-error" role="alert"><strong>Installation issue</strong><span>{installError}</span></div>{/if}
                        <section class="why-driver"><h3>Why this driver?</h3><ul>{#each (recommendedCandidate?.factors ?? activeRecommendation.currentFactors) as factor}<li class:negative={factor.score < 0}><span>{factor.score < 0 ? "!" : "✓"}</span><span><strong>{factor.label}</strong><small>{factor.detail}</small></span></li>{/each}</ul></section>
                        {#if activeRecommendation.newestNotBest}<div class="quiet-note"><strong>Newest is not always best</strong><p>{activeRecommendation.newestNotBest}</p></div>{/if}
                      {:else if selectedDevice.installedDriver?.genericMicrosoft}<div class="quiet-note actionable"><strong>Generic Microsoft driver</strong><p>This may be completely normal. DrvMatch can compare trusted Microsoft, vendor, and applicable OEM packages before suggesting any change.</p><button class="candidate-action" disabled={candidateLoading || viewingStoredScan} onclick={checkSelectedSources}>{candidateLoading ? "Checking sources" : "Compare trusted drivers"}</button></div>{:else}<div class="quiet-note actionable"><strong>Not evaluated yet</strong><p>Local inventory says the device is working. Check trusted sources only if you want DrvMatch to compare alternatives.</p><button class="candidate-action" disabled={candidateLoading || viewingStoredScan} onclick={checkSelectedSources}>{candidateLoading ? "Checking sources" : "Check this device"}</button></div>{/if}
                    {:else if detailTab === "candidates"}
                      <div class="candidate-intro"><strong>DriverRank evaluation</strong><p>Checks enabled Microsoft, component-vendor, and applicable OEM sources, ranks proven compatible packages, and compares the best result with the installed driver. No package is downloaded or installed.</p><button class="candidate-action" disabled={candidateLoading || viewingStoredScan} onclick={checkCandidates}>{candidateLoading ? "Evaluating sources" : candidateDiscovery ? "Evaluate again" : "Check and rank candidates"}</button>{#if viewingStoredScan}<small>Scan the current machine before checking sources for a stored inventory.</small>{/if}</div>
                      {#if candidateError}<div class="inline-error" role="alert"><strong>Source check issue</strong><span>{candidateError}</span></div>{/if}
                      {#if candidateLoading}
                        <div class="candidate-loading" role="status"><span class="spinner"></span><span><strong>Checking trusted sources</strong><small>Windows Update, Catalog, component vendors, and an applicable Dell/Lenovo/HP catalog may take a moment.</small></span></div>
                      {:else if candidateDiscovery}
                        <section class="source-health" aria-label="Driver source health"><h3>Sources</h3>{#each candidateDiscovery.sources as source}<div><span class="source-dot" class:failed={source.state === "failed"} class:skipped={source.state === "skipped"}></span><span><strong>{sourceLabel(source.source)}</strong><small>{source.message ?? `${source.candidateCount} candidates returned${source.cached ? " · cached metadata" : ""}`}</small></span><span>{sourceStateLabel(source.state)}</span></div>{/each}</section>
                        <section class="candidate-list"><h3>Ranked candidates <span>{candidateDiscovery.recommendation.rankedCandidates.length}</span></h3>{#each candidateDiscovery.recommendation.rankedCandidates as ranked (ranked.candidate.id)}{@const candidate = ranked.candidate}<article class="candidate-row" class:not-recommended={ranked.state === "notRecommended"}><div class="candidate-heading"><span><strong>{candidate.displayName}</strong><small>{sourceLabel(candidate.source)} · {compatibilityLabel(candidate.compatibility.state)}{candidate.alternateSources.length ? ` · Also available from ${candidate.alternateSources.map(sourceLabel).join(", ")}` : ""}</small></span><span class="candidate-rank" class:recommended={ranked.state === "recommended"} class:review={ranked.state === "notRecommended"}><strong>{recommendationLabel(ranked.state)}</strong><small>{candidate.versionIsPackageVersion ? "Package " : ""}{candidate.version ?? "Version not reported"}</small></span></div><p class="candidate-summary">{ranked.summary}</p><dl><div><dt>Provider</dt><dd>{candidate.provider ?? candidate.manufacturer ?? "Not reported"}</dd></div><div><dt>Date</dt><dd>{formatCandidateDate(candidate)}</dd></div><div><dt>Channel</dt><dd>{candidate.releaseChannel ?? "Not reported"}</dd></div><div><dt>Package</dt><dd>{candidate.packageGroup ?? candidate.packageType ?? "Driver package"} · {formatBytes(candidate.sizeBytes)}</dd></div></dl>{#if ranked.factors.length}<ul class="evidence">{#each ranked.factors.slice(0, 4) as factor}<li>{factor.detail}</li>{/each}</ul>{:else}<ul class="evidence">{#each candidate.compatibility.reasons as reason}<li>{reason}</li>{/each}</ul>{/if}{#if candidate.supportedOs.length}<p class="candidate-products">Products: {candidate.supportedOs.join(", ")}</p>{/if}<div class="candidate-actions">{#if candidate.downloadUrl || resolvedDownloadUrls[candidate.id]}<button onclick={() => copyDownloadUrl(candidate)}>Copy package URL</button>{:else if candidate.source === "microsoftCatalog"}<button disabled={resolvingCandidateId === candidate.id} onclick={() => resolveCandidateDownload(candidate)}>{resolvingCandidateId === candidate.id ? "Resolving" : "Resolve package metadata"}</button>{/if}{#if candidate.releaseNotesUrl}<button onclick={() => navigator.clipboard.writeText(candidate.releaseNotesUrl ?? "")}>Copy release notes URL</button>{/if}</div></article>{:else}<div class="candidate-empty"><strong>No candidates returned</strong><p>The installed driver can remain Current when sources have no suitable alternative.</p></div>{/each}</section>
                        {#if candidateDiscovery.recommendation.newestNotBest}<div class="quiet-note"><strong>Newest is not the best match</strong><p>{candidateDiscovery.recommendation.newestNotBest}</p></div>{/if}
                      {/if}
                    {:else}
                      <dl class="technical"><div><dt>Device instance ID</dt><dd>{selectedDevice.instanceId}</dd></div><div><dt>Class GUID</dt><dd>{selectedDevice.classGuid ?? "Not reported"}</dd></div><div><dt>Problem code</dt><dd>{selectedDevice.problemCode ?? "None"}</dd></div><div><dt>Problem status</dt><dd>{selectedDevice.problemStatus === null ? "None" : `0x${(selectedDevice.problemStatus >>> 0).toString(16).padStart(8, "0")}`}</dd></div></dl>
                      {#if selectedDevice.hardwareIdentity}<section class="id-section"><h3>Detected hardware identity</h3><dl class="technical"><div><dt>Bus</dt><dd>{selectedDevice.hardwareIdentity.bus}</dd></div><div><dt>Vendor ID</dt><dd class="mono">{selectedDevice.hardwareIdentity.vendorId ?? "Not reported"}</dd></div><div><dt>Device ID</dt><dd class="mono">{selectedDevice.hardwareIdentity.deviceId ?? "Not reported"}</dd></div><div><dt>Subsystem / product</dt><dd class="mono">{selectedDevice.hardwareIdentity.subsystemId ?? "Not reported"}</dd></div><div><dt>Detected identity</dt><dd>{selectedDevice.hardwareIdentity.description}</dd></div></dl></section>{/if}
                      {#if machine}<section class="id-section"><h3>Machine applicability</h3><dl class="technical"><div><dt>System</dt><dd>{machineLabel}</dd></div><div><dt>System SKU</dt><dd class="mono">{machine.systemSku ?? "Not reported"}</dd></div><div><dt>Baseboard</dt><dd>{[machine.baseboardManufacturer, machine.baseboardProduct].filter(Boolean).join(" ") || "Not reported"}</dd></div><div><dt>BIOS</dt><dd>{machine.biosVersion ?? "Not reported"}</dd></div><div><dt>Windows release</dt><dd>{machine.windowsDisplayVersion ?? "Not reported"}</dd></div><div><dt>Windows build</dt><dd class="mono">{machine.windowsBuild ?? "Not reported"}</dd></div></dl></section>{/if}
                      {#if selectedDevice.installedDriver}<section class="id-section"><h3>Installed package</h3><dl class="technical"><div><dt>Published INF</dt><dd>{selectedDevice.installedDriver.publishedInfName ?? "Not reported"}</dd></div><div><dt>INF path</dt><dd>{selectedDevice.installedDriver.infPath ?? "Not reported"}</dd></div><div><dt>INF section</dt><dd>{selectedDevice.installedDriver.infSection ?? "Not reported"}</dd></div><div><dt>Matching ID</dt><dd>{selectedDevice.installedDriver.matchingId ?? "Not reported"}</dd></div><div><dt>Driver key</dt><dd>{selectedDevice.installedDriver.driverKey ?? "Not reported"}</dd></div><div><dt>Windows driver rank</dt><dd>{selectedDevice.installedDriver.driverRank === null ? "Not reported" : `0x${selectedDevice.installedDriver.driverRank.toString(16).padStart(8, "0")}`}</dd></div><div><dt>Signature class</dt><dd>{signatureLabel(selectedDevice.installedDriver.signature)}</dd></div><div><dt>INF signature verified</dt><dd>{selectedDevice.installedDriver.infSignatureVerified ? "Yes" : "Not verified"}</dd></div><div><dt>Signer</dt><dd>{selectedDevice.installedDriver.signer ?? "Not reported"}</dd></div><div><dt>Catalog / store identity</dt><dd>{selectedDevice.installedDriver.catalogFile ?? "Not reported"}</dd></div></dl></section>{/if}
                      {#if activeRecommendation}<section class="id-section ranking-technical"><h3>DriverRank factors</h3><dl class="technical"><div><dt>Decision</dt><dd>{recommendationLabel(activeRecommendation.state)}</dd></div><div><dt>Installed score</dt><dd>{activeRecommendation.currentScore ?? "No installed driver"}</dd></div>{#if leadingCandidate}<div><dt>Leading candidate score</dt><dd>{leadingCandidate.score}</dd></div>{/if}</dl>{#each (recommendedCandidate?.factors ?? (activeRecommendation.state === "current" ? activeRecommendation.currentFactors : leadingCandidate?.factors ?? [])) as factor}<div class="factor-row"><span><strong>{factor.label}</strong><small>{factor.detail}</small></span><span class:negative={factor.score < 0}>{factorScore(factor.score)}</span></div>{/each}<small class="score-disclaimer">Internal scores compare decomposed evidence; they are not a confidence percentage.</small></section>{/if}
                      <section class="id-section technical-identifiers"><h3>Hardware IDs</h3>{#if selectedDevice.hardwareIds.length}<ul>{#each selectedDevice.hardwareIds as id}<li>{id}</li>{/each}</ul>{:else}<p>Windows did not expose hardware IDs for this device.</p>{/if}</section>
                      <section class="id-section technical-identifiers"><h3>Compatible IDs</h3>{#if selectedDevice.compatibleIds.length}<ul>{#each selectedDevice.compatibleIds as id}<li>{id}</li>{/each}</ul>{:else}<p>Windows did not expose compatible IDs for this device.</p>{/if}</section>
                    {/if}
                  </div>
                </aside>
              {/if}
            </div>
          {/if}
        </section>
      {:else if section === "history"}
        <section class="page"><header class="page-header"><div><h1>History</h1><p>Stored device and installed-driver inventories</p></div></header>{#if historyError}<div class="message" role="alert"><strong>Scan history unavailable</strong><span>{historyError}</span><button onclick={refreshHistory}>Try again</button></div>{:else if scans.length}<div class="history-list"><div class="history-header"><span>Scan</span><span>Devices</span><span>Findings</span><span></span></div>{#each scans as scan}<button class="history-row" onclick={() => openStoredScan(scan.id)}><span><strong>{new Date(scan.scannedAt * 1000).toLocaleString()}</strong><small>Local inventory · scan {scan.id}</small></span><span>{scan.deviceCount}</span><span>{scan.problemCount} problems · {scan.missingCount} missing{scan.genericCount ? ` · ${scan.genericCount} generic` : ""}</span><span>Open</span></button>{/each}</div>{:else}<div class="empty-section"><svg viewBox="0 0 24 24" aria-hidden="true"><path d={iconPath("history")} /></svg><h2>No scans recorded</h2><p>A completed device scan will appear here and can be reloaded without inspecting the machine again.</p></div>{/if}</section>
      {/if}
    </main>
    {#if section === "history" && installHistory.length}
      <section class="operation-history" aria-label="Driver installation history">
        <header><div><h1>History</h1><p>Driver changes and stored device inventories</p></div></header>
        <div class="history-workspace">
          <div class="history-scroll" role="listbox" aria-label="Driver changes">
            <h2>Driver changes</h2>
            <div class="operation-history-head"><span>Device</span><span>Version</span><span>Result</span></div>
            {#each installHistory as record}
              <button class="operation-history-row" class:selected={selectedHistory?.id === record.id} role="option" aria-selected={selectedHistory?.id === record.id} onclick={() => selectedHistoryId = record.id}>
                <span><strong>{record.deviceName}</strong><small>{new Date(record.completedAt * 1000).toLocaleString()} · {sourceLabel(record.source)}</small></span>
                <span class="mono">{record.previousVersion ?? "None"} → {record.installedVersion ?? "Not reported"}</span>
                <span><strong>{record.state === "succeeded" ? "Installed" : record.state === "staged" ? "Staged / retained" : record.state === "rolledBack" ? "Rolled back" : record.state === "rollbackFailed" ? "Rollback failed" : record.state === "cancelled" ? "Cancelled" : "Failed"}</strong><small>{record.rebootRequired ? "Restart required" : record.message}</small></span>
              </button>
            {/each}
            {#if scans.length}<h2>Device scans</h2>{#each scans as scan}<button class="stored-scan-row" onclick={() => openStoredScan(scan.id)}><span><strong>{new Date(scan.scannedAt * 1000).toLocaleString()}</strong><small>{scan.deviceCount} devices · {scan.problemCount} problems · {scan.missingCount} missing</small></span><span>Open</span></button>{/each}{/if}
          </div>
          {#if selectedHistory}<aside class="history-details" aria-label="Installation details"><header><div><h2>{selectedHistory.deviceName}</h2><p>{new Date(selectedHistory.completedAt * 1000).toLocaleString()}</p></div>{#if selectedHistory.rollbackAvailable}<button disabled={rollingBackId === selectedHistory.id} onclick={() => rollback(selectedHistory)}>{rollingBackId === selectedHistory.id ? "Rolling back" : "Rollback"}</button>{/if}</header><div><section><h3>Change</h3><dl><div><dt>Previous version</dt><dd class="mono">{selectedHistory.previousVersion ?? "No installed driver"}</dd></div><div><dt>Installed version</dt><dd class="mono">{selectedHistory.installedVersion ?? "Not reported"}</dd></div><div><dt>Package</dt><dd>{selectedHistory.candidateName}</dd></div><div><dt>Source</dt><dd>{sourceLabel(selectedHistory.source)}</dd></div><div><dt>Result</dt><dd>{selectedHistory.message}</dd></div></dl></section><section><h3>Verification &amp; safety</h3><dl><div><dt>Signature</dt><dd>{selectedHistory.signatureVerified ? "Windows trust verified" : "Not verified"}</dd></div><div><dt>SHA-256</dt><dd class="mono wrap">{selectedHistory.packageSha256 ?? "Not available"}</dd></div><div><dt>Restore point</dt><dd>{selectedHistory.restorePointCreated ? "Created" : selectedHistory.restorePointAttempted ? "Attempted; unavailable" : "Not requested"}</dd></div><div><dt>Previous package backup</dt><dd class="wrap">{selectedHistory.backupPath ?? "Not available"}</dd></div><div><dt>Restart</dt><dd>{selectedHistory.rebootRequired ? "Required" : "Not required"}</dd></div></dl></section><section><h3>Rollback</h3><p>{selectedHistory.state === "rolledBack" ? "Windows restored the previous driver." : selectedHistory.state === "rollbackFailed" ? selectedHistory.message : selectedHistory.rollbackAvailable ? "This completed INF change is eligible for a Windows native rollback attempt. Any exported package shown above is a separate recovery copy, not a guarantee that native rollback will succeed." : selectedHistory.backupPath ? "Native rollback is not available for this change. An exported copy of the previous package is preserved as recovery evidence." : "Rollback is not available for this change."}</p></section></div></aside>{/if}
        </div>
      </section>
    {/if}
    {#if section === "settings"}
      <section class="settings-workspace" aria-label="Application settings">
        <header><div><h1>Settings</h1><p>Appearance changes preview immediately. Save to keep changes.</p></div></header>
        {#if settingsLoading}<div class="message" role="status"><span class="spinner"></span><strong>Loading settings</strong></div>{:else}<div class="settings-layout"><nav class="settings-tabs" aria-label="Settings categories">{#each (["appearance", "sources", "safety", "advanced", "about"] as SettingsCategory[]) as category}<button aria-current={settingsCategory === category ? "page" : undefined} onclick={() => settingsCategory = category}>{category === "safety" ? "Safety & Rollback" : category[0].toUpperCase() + category.slice(1)}</button>{/each}</nav><div class="settings-panel">{#if settingsError}<div class="inline-error" role="alert"><strong>Settings issue</strong><span>{settingsError}</span></div>{/if}
          {#if settingsCategory === "appearance"}<h2>Appearance</h2><p class="category-note">Choose the shell material and visual behavior without changing the information layout.</p><label class="setting-row"><span><strong>Theme</strong><small>Follow Windows or use a fixed light or dark appearance.</small></span><select value={settings.theme} onchange={(event) => updateTheme(event.currentTarget.value as ThemePreference)} aria-label="Application theme"><option value="system">System</option><option value="light">Light</option><option value="dark">Dark</option></select></label><label class="setting-row"><span><strong>Acrylic backdrop</strong><small>Use the Windows acrylic material behind the application shell.</small></span><input type="checkbox" checked={settings.acrylic} onchange={(event) => updateAcrylic(event.currentTarget.checked)} aria-label="Use acrylic backdrop" /></label><label class="setting-row"><span><strong>Use Windows accent color</strong><small>Read the current Windows colorization color for primary actions.</small></span><input type="checkbox" checked={settings.useWindowsAccent} onchange={(event) => { settings.useWindowsAccent = event.currentTarget.checked; void applyAccent(settings.useWindowsAccent); }} aria-label="Use Windows accent color" /></label><label class="setting-row"><span><strong>Reduce motion</strong><small>Disable non-essential interface transitions.</small></span><input type="checkbox" checked={settings.reduceMotion} onchange={(event) => { settings.reduceMotion = event.currentTarget.checked; applyReducedMotion(settings.reduceMotion); }} aria-label="Reduce interface motion" /></label>
          {:else if settingsCategory === "sources"}<h2>Driver sources</h2><p class="category-note">Sources contribute evidence; none overrides hardware suitability by name alone.</p>{#each allSources as source}{@const health = sourceHealth.find((entry) => entry.source === source)}<label class="source-setting-row"><span><strong>{sourceLabel(source)}</strong><small>{health ? `${sourceStateLabel(health.state)} · checked ${new Date(health.checkedAt * 1000).toLocaleString()} · ${health.candidateCount} candidates${health.cached ? " · cached" : ""}` : "Not checked yet"}</small>{#if health?.message}<small>{health.message}</small>{/if}</span><input type="checkbox" checked={settings.enabledSources.includes(source)} onchange={(event) => updateSource(source, event.currentTarget.checked)} aria-label={`Use ${sourceLabel(source)} source`} /></label>{/each}<h3 class="settings-subheading">System OEM sources</h3><p class="category-note">Dell, Lenovo, and HP query the detected machine's official OEM catalog. A package is considered compatible only when the catalog proves machine applicability and provides a matching PnP device ID; BIOS, firmware, and app-only entries stay outside the driver flow.</p>{#each oemSources as source}{@const health = sourceHealth.find((entry) => entry.source === source)}<label class="source-setting-row"><span><strong>{sourceLabel(source)}</strong><small>{health ? `${sourceStateLabel(health.state)} · checked ${new Date(health.checkedAt * 1000).toLocaleString()} · ${health.candidateCount} candidates` : "Not checked yet"}</small>{#if health?.message}<small>{health.message}</small>{/if}</span><input type="checkbox" checked={settings.enabledOemSources.includes(source)} onchange={(event) => updateSource(source, event.currentTarget.checked, true)} aria-label={`Use ${sourceLabel(source)} OEM source`} /></label>{/each}
          {:else if settingsCategory === "safety"}<h2>Safety &amp; rollback</h2><p class="category-note">These defaults apply to every reviewed installation.</p><label class="setting-row"><span><strong>Create a restore point</strong><small>Ask Windows for a system checkpoint before driver changes.</small></span><input type="checkbox" bind:checked={settings.createRestorePoint} aria-label="Create restore point before installation" /></label><label class="setting-row"><span><strong>Back up current package</strong><small>Export the current OEM package when Windows permits it.</small></span><input type="checkbox" bind:checked={settings.backupCurrentPackage} aria-label="Back up current driver package" /></label>
          {:else if settingsCategory === "advanced"}<h2>Advanced</h2><p class="category-note">Technical visibility, cached source metadata, and local diagnostic records.</p><label class="setting-row"><span><strong>Show exact hardware IDs</strong><small>Expose device identifiers in Technical details.</small></span><input type="checkbox" checked={settings.showExactIds} onchange={(event) => { settings.showExactIds = event.currentTarget.checked; applyTechnicalVisibility(settings); }} aria-label="Show exact hardware IDs" /></label><label class="setting-row"><span><strong>Show internal ranking scores</strong><small>Expose decomposed numeric scores only in Technical details.</small></span><input type="checkbox" checked={settings.showInternalScores} onchange={(event) => { settings.showInternalScores = event.currentTarget.checked; applyTechnicalVisibility(settings); }} aria-label="Show internal ranking scores" /></label><label class="setting-row"><span><strong>Logging</strong><small>Detailed logging adds source and operation diagnostics.</small></span><select bind:value={settings.logVerbosity} aria-label="Logging verbosity"><option value="normal">Normal</option><option value="detailed">Detailed</option></select></label><div class="management-row"><span><strong>Source metadata cache</strong><small>{cacheStats.entryCount} entries · {formatBytes(cacheStats.fileSizeBytes)}</small></span><button disabled={managementBusy === "cache"} onclick={clearCache}>{managementBusy === "cache" ? "Clearing" : "Clear cache"}</button></div><div class="management-row"><span><strong>Activity log</strong><small>Access the most recent 256 KB of local diagnostic events.</small></span><span class="row-actions"><button disabled={managementBusy === "log"} onclick={loadActivityLog}>{managementBusy === "log" ? "Loading" : "View log"}</button><button onclick={copyActivityLog}>Copy</button><button disabled={managementBusy === "clear-log"} onclick={clearActivityLog}>Clear</button></span></div>{#if activityLog}<pre class="activity-log">{activityLog}</pre>{/if}
          {:else}<h2>About</h2><p class="category-note">DrvMatch recommends the best-supported driver for the exact machine, not simply the newest package.</p><dl class="about-details"><div><dt>Version</dt><dd class="mono">{appInfo.version}</dd></div><div><dt>Repository</dt><dd>{appInfo.repository}</dd></div><div><dt>Platform</dt><dd>Windows 11 x64</dd></div><div><dt>Safety model</dt><dd>Compatibility, provenance, signing, and explicit approval before installation.</dd></div></dl><button class="standard-button" onclick={copyRepository}>Copy repository address</button>{/if}
        </div></div>{/if}
        {#if settingsDirty}<footer class="settings-save-bar" role="status"><span><strong>Unsaved changes</strong><small>Save or discard before leaving these preferences.</small></span><span><button disabled={settingsSaving} onclick={discardSettings}>Discard</button><button class="primary-button" disabled={settingsSaving} onclick={saveApplicationSettings}>{settingsSaving ? "Saving" : "Save changes"}</button></span></footer>{/if}
      </section>
    {/if}
  </div>

  {#if installReview}
    <div class="dialog-backdrop" role="presentation" onkeydown={handleReviewKeydown}>
      <div class="install-dialog" role="dialog" aria-modal="true" aria-labelledby="install-review-title">
        <header><h2 id="install-review-title">Ready to install</h2><p>Review the exact package and safety actions before Windows is changed.</p></header>
        <div class="review-items">{#each installReview.items as item}<div><strong>{item.deviceName}</strong><span class="mono">{item.currentVersion ?? "No driver"} → {item.version ?? "Version not reported"}</span><small>{sourceLabel(item.source)}{item.channel ? ` · ${item.channel}` : ""} · {item.packageType ?? "Driver package"}</small></div>{/each}</div>
        <section class="review-safety"><h3>Safety</h3><p><span>✓</span> The package will be hashed with SHA-256 and its Windows signature verified.</p><p><span>✓</span> For INF packages, DrvMatch must prove one exact target INF before elevation; ambiguous multi-INF packages are blocked rather than installed as a set.</p><p><span>✓</span> Windows will install the selected signed INF without a force-install flag.</p><p><span>{savedSettings.createRestorePoint ? "✓" : "—"}</span> {savedSettings.createRestorePoint ? "A restore point will be attempted." : "Restore-point creation is disabled."}</p><p><span>{savedSettings.backupCurrentPackage ? "✓" : "—"}</span> {savedSettings.backupCurrentPackage ? "The current package will be exported when available." : "Current-package backup is disabled."}</p></section>
        {#if installError}<div class="inline-error" role="alert"><strong>Could not start installation</strong><span>{installError}</span></div>{/if}
        <footer><button disabled={committingInstall} onclick={closeInstallReview}>Cancel</button><button id="confirm-install" class="primary-button" disabled={committingInstall} onclick={commitReviewedInstall}>{committingInstall ? "Starting" : `Install ${installReview.items.length} driver${installReview.items.length === 1 ? "" : "s"}`}</button></footer>
      </div>
    </div>
  {/if}

  <footer class="status-bar" class:operation={installStatus.phase !== "idle"}>
    {#if installStatus.phase !== "idle"}
      <span class="ready-dot" class:error={installStatus.phase === "failed"}></span>
      <span class="footer-status"><strong>{installStatus.message}</strong><small>{installStatus.currentItem ?? "Driver operation"}</small></span>
      <div class="operation-progress" role="progressbar" aria-label="Driver installation progress" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(installStatus.progress * 100)}><span style:width={`${Math.round(installStatus.progress * 100)}%`}></span></div>
      <span class="operation-count">{installStatus.completedItems} of {installStatus.totalItems}</span>
      {#if installStatus.cancellable}<button onclick={cancelInstall}>Cancel</button>{/if}
      {#if installStatus.phase === "completed" && installHistory[0]?.rollbackAvailable}<button onclick={() => rollback(installHistory[0])}>Rollback</button>{/if}
    {:else}
      {#if section === "drivers"}
        <button class="footer-scan" disabled={loading} onclick={scanDevices}><svg viewBox="0 0 24 24" aria-hidden="true"><path d={iconPath("refresh")} /></svg>{loading ? "Scanning" : "Scan again"}</button>
      {:else}
        <span class="footer-product"><span class="brand-mini"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 6h4v12H7m10-12h-4v12h4M10 12h4" /></svg></span>DrvMatch</span>
      {/if}
      <div class="footer-track" aria-hidden="true"><span></span></div>
      <span class="ready-dot" class:error={error !== null || candidateError !== null}></span>
      <span class="footer-status"><strong>{loading ? "Inspecting devices" : candidateLoading ? "Checking sources" : error ? "Inventory unavailable" : "Ready"}</strong><small>{devices.length ? `${devices.length} present devices` : "No inventory loaded"}</small></span>
      <span class="status-time">{lastScanned ? `Scanned ${lastScanned.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}` : "Local machine"}</span>
    {/if}
  </footer>
</div>