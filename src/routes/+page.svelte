<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import type { AppInfo, AppSettings, CacheStats, CandidateDiscovery, CompatibilityState, DetailTab, Device, DownloadResolution, DriverCandidate, DriverFilter, DriverSourceKind, InstallRecord, InstallReview, InstallStatus, InventorySnapshot, NavigationSection, RecommendationState, ScanSummary, SettingsCategory, SignatureStatus, SourceHealth, SourceHealthState, ThemePreference } from "$lib/types";

  const allSources: DriverSourceKind[] = ["windowsUpdate", "microsoftCatalog", "amd", "nvidia", "intel"];
  const defaultSettings: AppSettings = { theme: "system", acrylic: true, useWindowsAccent: true, reduceMotion: false, enabledSources: [...allSources], createRestorePoint: true, backupCurrentPackage: true, confirmOptionalDrivers: true, offerRollbackAfterFailure: true, showExactIds: false, showInternalScores: false, logVerbosity: "normal" };
  const detailTabs: DetailTab[] = ["overview", "candidates", "technical"];

  let section = $state<NavigationSection>("drivers");
  let detailTab = $state<DetailTab>("overview");
  let devices = $state<Device[]>([]);
  let selectedId = $state<string | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let settings = $state<AppSettings>({ ...defaultSettings, enabledSources: [...allSources] });
  let savedSettings = $state<AppSettings>({ ...defaultSettings, enabledSources: [...allSources] });
  let settingsCategory = $state<SettingsCategory>("appearance");
  let settingsLoading = $state(true);
  let settingsSaving = $state(false);
  let settingsError = $state<string | null>(null);
  let sourceHealth = $state<SourceHealth[]>([]);
  let cacheStats = $state<CacheStats>({ entryCount: 0, fileSizeBytes: 0 });
  let activityLog = $state("");
  let appInfo = $state<AppInfo>({ version: "0.7.0", repository: "https://github.com/tommy4377/DrvMatch" });
  let managementBusy = $state<string | null>(null);
  let lastScanned = $state<Date | null>(null);
  let scans = $state<ScanSummary[]>([]);
  let historyError = $state<string | null>(null);
  let activeSummary = $state<ScanSummary | null>(null);
  let viewingStoredScan = $state(false);
  let search = $state("");
  let filter = $state<DriverFilter>("all");
  let candidateDiscovery = $state<CandidateDiscovery | null>(null);
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

  const selectedDevice = $derived(devices.find((device) => device.instanceId === selectedId) ?? null);
  const activeRecommendation = $derived(candidateDiscovery?.deviceInstanceId === selectedId ? candidateDiscovery.recommendation : null);
  const recommendedCandidate = $derived(activeRecommendation?.rankedCandidates.find((entry) => entry.candidate.id === activeRecommendation.selectedCandidateId) ?? null);
  const leadingCandidate = $derived(activeRecommendation?.rankedCandidates.find((entry) => entry.factors.length > 0) ?? null);
  const classCount = $derived(new Set(devices.map((device) => device.className).filter(Boolean)).size);
  const settingsDirty = $derived(JSON.stringify(settings) !== JSON.stringify(savedSettings));
  const selectedHistory = $derived(installHistory.find((record) => record.id === selectedHistoryId) ?? installHistory[0] ?? null);
  const filteredDevices = $derived(devices.filter((device) => {
    const query = search.trim().toLocaleLowerCase();
    const matchesSearch = !query || [device.friendlyName, device.description, device.manufacturer, device.className, device.instanceId, ...device.hardwareIds, ...device.compatibleIds, device.installedDriver?.provider, device.installedDriver?.version, device.installedDriver?.publishedInfName, device.installedDriver?.matchingId]
      .some((value) => value?.toLocaleLowerCase().includes(query));
    const matchesFilter = filter === "all"
      || (filter === "problem" && device.condition === "problem")
      || (filter === "missing" && device.condition === "missing")
      || (filter === "generic" && device.installedDriver?.genericMicrosoft === true);
    return matchesSearch && matchesFilter;
  }));

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

  function updateSource(source: DriverSourceKind, enabled: boolean): void {
    if (!enabled && settings.enabledSources.length === 1) {
      settings.enabledSources = [...settings.enabledSources];
      settingsError = "At least one trusted driver source must remain enabled.";
      return;
    }
    settings.enabledSources = enabled
      ? [...new Set([...settings.enabledSources, source])]
      : settings.enabledSources.filter((entry) => entry !== source);
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
      lastScanned = new Date(snapshot.summary.scannedAt * 1000);
      viewingStoredScan = false;
      candidateDiscovery = null;
      candidateError = null;
      resolvedDownloadUrls = {};
      await refreshHistory();
      if (!devices.some((device) => device.instanceId === selectedId)) selectedId = devices[0]?.instanceId ?? null;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
      devices = [];
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

  async function applyAccent(enabled: boolean): Promise<void> {
    document.documentElement.style.removeProperty("--accent");
    document.documentElement.style.removeProperty("--accent-hover");
    document.documentElement.style.removeProperty("--focus");
    if (!enabled || !isTauri()) return;
    try {
      const accent = await invoke<string | null>("get_windows_accent");
      if (accent) {
        document.documentElement.style.setProperty("--accent", accent);
        document.documentElement.style.setProperty("--accent-hover", `color-mix(in srgb, ${accent} 82%, black)`);
        document.documentElement.style.setProperty("--focus", accent);
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
    return { ...value, enabledSources: [...value.enabledSources] };
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
      lastScanned = new Date(snapshot.summary.scannedAt * 1000);
      viewingStoredScan = true;
      selectedId = devices[0]?.instanceId ?? null;
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

  function signatureLabel(signature: SignatureStatus): string {
    return ({ whql: "WHQL", inbox: "Microsoft inbox", authenticode: "Authenticode", signedUnclassified: "Microsoft-signed", unsigned: "Unsigned", unknown: "Not determined" })[signature];
  }

  function conditionLabel(device: Device): string {
    if (candidateDiscovery?.deviceInstanceId === device.instanceId) return recommendationLabel(candidateDiscovery.recommendation.state);
    if (device.condition === "problem") return `Problem${device.problemCode ? ` · code ${device.problemCode}` : ""}`;
    if (device.condition === "missing") return "Missing";
    return "Current";
  }

  function conditionDescription(device: Device): string {
    if (candidateDiscovery?.deviceInstanceId === device.instanceId) return candidateDiscovery.recommendation.summary;
    if (device.condition === "current") return "Windows reports an installed driver and no device problem.";
    if (device.condition === "missing") return "No installed driver package was associated with this hardware device.";
    return "Windows reports a problem for this device.";
  }

  function sourceLabel(source: DriverSourceKind): string {
    return ({ windowsUpdate: "Windows Update", microsoftCatalog: "Microsoft Update Catalog", amd: "AMD", nvidia: "NVIDIA", intel: "Intel" })[source];
  }

  function compatibilityLabel(state: CompatibilityState): string {
    return state === "compatible" ? "Compatible" : state === "needsReview" ? "Needs package review" : "Rejected";
  }

  function recommendationLabel(state: RecommendationState): string {
    return ({ recommended: "Recommended", optional: "Optional", current: "Current", missing: "Missing", notRecommended: "Not recommended" })[state];
  }

  function factorScore(score: number): string {
    return score > 0 ? `+${score}` : `${score}`;
  }

  function sourceStateLabel(state: SourceHealthState): string {
    return state === "available" ? "Available" : state === "skipped" ? "Skipped" : "Unavailable";
  }

  function formatCandidateDate(candidate: DriverCandidate): string {
    if (candidate.publicationDate) return candidate.publicationDate;
    return candidate.driverDate ? new Date(candidate.driverDate * 1000).toLocaleDateString() : "Date not reported";
  }

  function formatBytes(bytes: number | null): string {
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
    void refreshHistory();
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
    <div class="brand" data-tauri-drag-region><span class="brand-mark">D</span><strong>DrvMatch</strong></div>
    <div class="window-controls">
      <button aria-label="Minimize window" onclick={() => getCurrentWindow().minimize()}><svg viewBox="0 0 24 24" aria-hidden="true"><path d={iconPath("minimize")} /></svg></button>
      <button aria-label="Maximize unavailable because the window has a fixed size" disabled><svg viewBox="0 0 24 24" aria-hidden="true"><rect x="7" y="7" width="10" height="10" /></svg></button>
      <button class="close" aria-label="Close window" onclick={() => getCurrentWindow().close()}><svg viewBox="0 0 24 24" aria-hidden="true"><path d={iconPath("close")} /></svg></button>
    </div>
  </header>

  <div class="workspace">
    <nav class="navigation" aria-label="Primary navigation">
      <p class="nav-label">DrvMatch</p>
      {#each (["drivers", "history", "settings"] as NavigationSection[]) as item}
        <button aria-current={section === item ? "page" : undefined} onclick={() => navigate(item)}>
          <span class="selection-indicator"></span><svg viewBox="0 0 24 24" aria-hidden="true"><path d={iconPath(item)} /></svg><span>{item[0].toUpperCase() + item.slice(1)}</span>
        </button>
      {/each}
      <div class="nav-note"><strong>Suitability first</strong><span>Newer does not always mean better.</span></div>
    </nav>

    <main class="content" inert={section === "settings" || (section === "history" && installHistory.length) ? true : undefined}>
      {#if section === "drivers"}
        <section class="page">
          <header class="page-header"><div><h1>Drivers</h1><p>Devices detected on this machine</p></div><button class="primary-button" disabled={loading} onclick={scanDevices}><svg viewBox="0 0 24 24" aria-hidden="true"><path d={iconPath("refresh")} /></svg>{loading ? "Inspecting devices" : "Scan again"}</button></header>
          <div class="summary-bar" aria-live="polite"><span><strong>{devices.length}</strong> present</span><span class="separator"></span><span class:attention={activeSummary?.problemCount}>{activeSummary?.problemCount ?? 0} problems</span><span class="separator"></span><span class:attention={activeSummary?.missingCount}>{activeSummary?.missingCount ?? 0} missing drivers</span><span class="summary-copy">{viewingStoredScan ? "Stored inventory — scan again before making decisions." : `${classCount} device classes · local inventory`}</span></div>
          <div class="command-bar">
            <label class="search-box" aria-label="Search device inventory"><svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="10.5" cy="10.5" r="6.5"></circle><path d="m15.5 15.5 4 4"></path></svg><input id="device-search" bind:value={search} placeholder="Search devices, drivers, or IDs" /></label>
            <label class="filter-control"><span>Show</span><select bind:value={filter} aria-label="Filter device inventory"><option value="all">All devices</option><option value="problem">Problems</option><option value="missing">Missing driver</option><option value="generic">Generic Microsoft</option></select></label>
          </div>

          {#if error}
            <div class="message" role="alert"><strong>Device inventory unavailable</strong><span>{error}</span><button onclick={scanDevices}>Try again</button></div>
          {:else if loading}
            <div class="message" role="status"><span class="spinner"></span><strong>Inspecting Windows devices</strong><span>Reading present Plug and Play devices through Windows SetupAPI.</span></div>
          {:else if devices.length === 0}
            <div class="message"><strong>No present devices were returned</strong><span>Run the scan again. DrvMatch has not inferred or fabricated any inventory.</span></div>
          {:else}
            <div class="split-view" class:details-open={selectedDevice !== null}>
              <div class="device-list" role="listbox" aria-label="Detected devices">
                <div class="list-header"><span>Device</span><span>Status</span><span>Installed driver</span></div>
                {#each filteredDevices as device (device.instanceId)}
                  <button class="device-row" class:selected={selectedId === device.instanceId} role="option" aria-selected={selectedId === device.instanceId} onclick={() => selectDevice(device)}>
                    <span class="device-identity"><span class="device-icon"><svg viewBox="0 0 24 24" aria-hidden="true"><path d={iconPath("device")} /></svg></span><span><strong>{device.friendlyName}</strong><small>{device.className ?? "Other"} · {device.manufacturer ?? "Not reported"}</small></span></span>
                    <span class="condition" class:problem={device.condition === "problem"} class:missing={device.condition === "missing"}><span></span>{conditionLabel(device)}</span>
                    <span class="driver-cell"><strong>{device.installedDriver?.version ?? "No installed package"}</strong><small>{device.installedDriver?.provider ?? device.installedDriver?.publishedInfName ?? "No provider reported"}</small></span>
                  </button>
                {:else}
                  <div class="filtered-empty">No devices match the current search and filter.</div>
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
                          {#if activeRecommendation.state === "recommended" && recommendedCandidate}
                            <button class="install-action" disabled={preparingInstall || ["downloading", "verifying", "preparingSafety", "installing"].includes(installStatus.phase)} onclick={prepareRecommendedInstall}>{preparingInstall ? "Preparing review" : "Review installation"}</button>
                          {/if}
                        </section>
                        {#if installError}<div class="inline-error" role="alert"><strong>Installation issue</strong><span>{installError}</span></div>{/if}
                        <section class="why-driver"><h3>Why this driver?</h3><ul>{#each (recommendedCandidate?.factors ?? activeRecommendation.currentFactors) as factor}<li class:negative={factor.score < 0}><span>{factor.score < 0 ? "!" : "✓"}</span><span><strong>{factor.label}</strong><small>{factor.detail}</small></span></li>{/each}</ul></section>
                        {#if activeRecommendation.newestNotBest}<div class="quiet-note"><strong>Newest is not always best</strong><p>{activeRecommendation.newestNotBest}</p></div>{/if}
                      {:else if selectedDevice.installedDriver?.genericMicrosoft}<div class="quiet-note"><strong>Generic Microsoft driver detected</strong><p>The installed package identifies itself as generic. Check Microsoft sources to compare suitability.</p></div>{:else}<div class="quiet-note"><strong>No recommendation yet</strong><p>Check Microsoft sources in the Candidates tab to evaluate suitable alternatives.</p></div>{/if}
                    {:else if detailTab === "candidates"}
                      <div class="candidate-intro"><strong>DriverRank evaluation</strong><p>Checks Microsoft sources, ranks compatible packages, and compares the best result with the installed driver. No package is downloaded or installed.</p><button class="candidate-action" disabled={candidateLoading || viewingStoredScan} onclick={checkCandidates}>{candidateLoading ? "Evaluating sources" : candidateDiscovery ? "Evaluate again" : "Check and rank candidates"}</button>{#if viewingStoredScan}<small>Scan the current machine before checking sources for a stored inventory.</small>{/if}</div>
                      {#if candidateError}<div class="inline-error" role="alert"><strong>Source check issue</strong><span>{candidateError}</span></div>{/if}
                      {#if candidateLoading}
                        <div class="candidate-loading" role="status"><span class="spinner"></span><span><strong>Checking Microsoft sources</strong><small>Windows Update applicability and the Catalog exact-ID query may take a moment.</small></span></div>
                      {:else if candidateDiscovery}
                        <section class="source-health" aria-label="Driver source health"><h3>Sources</h3>{#each candidateDiscovery.sources as source}<div><span class="source-dot" class:failed={source.state === "failed"} class:skipped={source.state === "skipped"}></span><span><strong>{sourceLabel(source.source)}</strong><small>{source.message ?? `${source.candidateCount} candidates returned${source.cached ? " · cached metadata" : ""}`}</small></span><span>{sourceStateLabel(source.state)}</span></div>{/each}</section>
                        <section class="candidate-list"><h3>Ranked candidates <span>{candidateDiscovery.recommendation.rankedCandidates.length}</span></h3>{#each candidateDiscovery.recommendation.rankedCandidates as ranked (ranked.candidate.id)}{@const candidate = ranked.candidate}<article class="candidate-row" class:not-recommended={ranked.state === "notRecommended"}><div class="candidate-heading"><span><strong>{candidate.displayName}</strong><small>{sourceLabel(candidate.source)} · {compatibilityLabel(candidate.compatibility.state)}{candidate.alternateSources.length ? ` · Also available from ${candidate.alternateSources.map(sourceLabel).join(", ")}` : ""}</small></span><span class="candidate-rank" class:recommended={ranked.state === "recommended"} class:review={ranked.state === "notRecommended"}><strong>{recommendationLabel(ranked.state)}</strong><small>{candidate.versionIsPackageVersion ? "Package " : ""}{candidate.version ?? "Version not reported"}</small></span></div><p class="candidate-summary">{ranked.summary}</p><dl><div><dt>Provider</dt><dd>{candidate.provider ?? candidate.manufacturer ?? "Not reported"}</dd></div><div><dt>Date</dt><dd>{formatCandidateDate(candidate)}</dd></div><div><dt>Channel</dt><dd>{candidate.releaseChannel ?? "Not reported"}</dd></div><div><dt>Package</dt><dd>{candidate.packageGroup ?? candidate.packageType ?? "Driver package"} · {formatBytes(candidate.sizeBytes)}</dd></div></dl>{#if ranked.factors.length}<ul class="evidence">{#each ranked.factors.slice(0, 4) as factor}<li>{factor.detail}</li>{/each}</ul>{:else}<ul class="evidence">{#each candidate.compatibility.reasons as reason}<li>{reason}</li>{/each}</ul>{/if}{#if candidate.supportedOs.length}<p class="candidate-products">Products: {candidate.supportedOs.join(", ")}</p>{/if}<div class="candidate-actions">{#if candidate.downloadUrl || resolvedDownloadUrls[candidate.id]}<button onclick={() => copyDownloadUrl(candidate)}>Copy package URL</button>{:else if candidate.source === "microsoftCatalog"}<button disabled={resolvingCandidateId === candidate.id} onclick={() => resolveCandidateDownload(candidate)}>{resolvingCandidateId === candidate.id ? "Resolving" : "Resolve package metadata"}</button>{/if}{#if candidate.releaseNotesUrl}<button onclick={() => navigator.clipboard.writeText(candidate.releaseNotesUrl ?? "")}>Copy release notes URL</button>{/if}</div></article>{:else}<div class="candidate-empty"><strong>No candidates returned</strong><p>The installed driver can remain Current when sources have no suitable alternative.</p></div>{/each}</section>
                        {#if candidateDiscovery.recommendation.newestNotBest}<div class="quiet-note"><strong>Newest is not the best match</strong><p>{candidateDiscovery.recommendation.newestNotBest}</p></div>{/if}
                      {/if}
                    {:else}
                      <dl class="technical"><div><dt>Device instance ID</dt><dd>{selectedDevice.instanceId}</dd></div><div><dt>Class GUID</dt><dd>{selectedDevice.classGuid ?? "Not reported"}</dd></div><div><dt>Problem code</dt><dd>{selectedDevice.problemCode ?? "None"}</dd></div><div><dt>Problem status</dt><dd>{selectedDevice.problemStatus === null ? "None" : `0x${(selectedDevice.problemStatus >>> 0).toString(16).padStart(8, "0")}`}</dd></div></dl>
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
      {:else}
        <section class="page"><header class="page-header"><div><h1>Settings</h1><p>Appearance and application behavior</p></div></header><div class="settings-content"><section><h2>Appearance</h2><label class="setting-row"><span><strong>Theme</strong><small>Follow Windows or choose a fixed appearance.</small></span><select value={settings.theme} onchange={(event) => updateTheme(event.currentTarget.value as ThemePreference)} aria-label="Application theme"><option value="system">System</option><option value="light">Light</option><option value="dark">Dark</option></select></label><label class="setting-row"><span><strong>Acrylic backdrop</strong><small>Use the Windows acrylic material behind the application shell.</small></span><input type="checkbox" checked={settings.acrylic} onchange={(event) => updateAcrylic(event.currentTarget.checked)} aria-label="Use acrylic backdrop" /></label></section><section><h2>Driver sources</h2>{#each allSources as source}<label class="setting-row"><span><strong>{sourceLabel(source)}</strong><small>{source === "windowsUpdate" ? "Machine-applicable offers from Windows." : source === "microsoftCatalog" ? "Exact-ID searches of the official Microsoft catalog." : `Public ${sourceLabel(source)} packages and release channels.`}</small></span><input type="checkbox" checked={settings.enabledSources.includes(source)} onchange={(event) => updateSource(source, event.currentTarget.checked)} aria-label={`Use ${sourceLabel(source)} source`} /></label>{/each}<p class="settings-note">Sources contribute evidence; no source overrides hardware suitability by name alone.</p></section><section><h2>About</h2><div class="setting-row"><span><strong>DrvMatch 0.7.0</strong><small>Find the right driver for this machine, not simply the newest driver.</small></span><span class="status-badge">Safe install &amp; rollback</span></div></section></div></section>
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
                <span><strong>{record.state === "succeeded" ? "Installed" : record.state === "rolledBack" ? "Rolled back" : record.state === "rollbackFailed" ? "Rollback failed" : record.state === "cancelled" ? "Cancelled" : "Failed"}</strong><small>{record.rebootRequired ? "Restart required" : record.message}</small></span>
              </button>
            {/each}
            {#if scans.length}<h2>Device scans</h2>{#each scans as scan}<button class="stored-scan-row" onclick={() => openStoredScan(scan.id)}><span><strong>{new Date(scan.scannedAt * 1000).toLocaleString()}</strong><small>{scan.deviceCount} devices · {scan.problemCount} problems · {scan.missingCount} missing</small></span><span>Open</span></button>{/each}{/if}
          </div>
          {#if selectedHistory}<aside class="history-details" aria-label="Installation details"><header><div><h2>{selectedHistory.deviceName}</h2><p>{new Date(selectedHistory.completedAt * 1000).toLocaleString()}</p></div>{#if selectedHistory.rollbackAvailable}<button disabled={rollingBackId === selectedHistory.id} onclick={() => rollback(selectedHistory)}>{rollingBackId === selectedHistory.id ? "Rolling back" : "Rollback"}</button>{/if}</header><div><section><h3>Change</h3><dl><div><dt>Previous version</dt><dd class="mono">{selectedHistory.previousVersion ?? "No installed driver"}</dd></div><div><dt>Installed version</dt><dd class="mono">{selectedHistory.installedVersion ?? "Not reported"}</dd></div><div><dt>Package</dt><dd>{selectedHistory.candidateName}</dd></div><div><dt>Source</dt><dd>{sourceLabel(selectedHistory.source)}</dd></div><div><dt>Result</dt><dd>{selectedHistory.message}</dd></div></dl></section><section><h3>Verification &amp; safety</h3><dl><div><dt>Signature</dt><dd>{selectedHistory.signatureVerified ? "Windows trust verified" : "Not verified"}</dd></div><div><dt>SHA-256</dt><dd class="mono wrap">{selectedHistory.packageSha256 ?? "Not available"}</dd></div><div><dt>Restore point</dt><dd>{selectedHistory.restorePointCreated ? "Created" : selectedHistory.restorePointAttempted ? "Attempted; unavailable" : "Not requested"}</dd></div><div><dt>Previous package backup</dt><dd class="wrap">{selectedHistory.backupPath ?? "Not available"}</dd></div><div><dt>Restart</dt><dd>{selectedHistory.rebootRequired ? "Required" : "Not required"}</dd></div></dl></section><section><h3>Rollback</h3><p>{selectedHistory.state === "rolledBack" ? "Windows restored the previous driver." : selectedHistory.state === "rollbackFailed" ? selectedHistory.message : selectedHistory.rollbackAvailable ? "A preserved previous package is available for Windows rollback." : "Rollback is not available for this change."}</p></section></div></aside>{/if}
        </div>
      </section>
    {/if}
    {#if section === "settings"}
      <section class="settings-workspace" aria-label="Application settings">
        <header><div><h1>Settings</h1><p>Appearance changes preview immediately. Save to keep changes.</p></div></header>
        {#if settingsLoading}<div class="message" role="status"><span class="spinner"></span><strong>Loading settings</strong></div>{:else}<div class="settings-layout"><nav aria-label="Settings categories">{#each (["appearance", "sources", "safety", "advanced", "about"] as SettingsCategory[]) as category}<button aria-current={settingsCategory === category ? "page" : undefined} onclick={() => settingsCategory = category}><span></span>{category === "safety" ? "Safety & Rollback" : category[0].toUpperCase() + category.slice(1)}</button>{/each}</nav><div class="settings-panel">{#if settingsError}<div class="inline-error" role="alert"><strong>Settings issue</strong><span>{settingsError}</span></div>{/if}
          {#if settingsCategory === "appearance"}<h2>Appearance</h2><p class="category-note">Choose the shell material and visual behavior without changing the information layout.</p><label class="setting-row"><span><strong>Theme</strong><small>Follow Windows or use a fixed light or dark appearance.</small></span><select value={settings.theme} onchange={(event) => updateTheme(event.currentTarget.value as ThemePreference)} aria-label="Application theme"><option value="system">System</option><option value="light">Light</option><option value="dark">Dark</option></select></label><label class="setting-row"><span><strong>Acrylic backdrop</strong><small>Use the Windows acrylic material behind the application shell.</small></span><input type="checkbox" checked={settings.acrylic} onchange={(event) => updateAcrylic(event.currentTarget.checked)} aria-label="Use acrylic backdrop" /></label><label class="setting-row"><span><strong>Use Windows accent color</strong><small>Read the current Windows colorization color for primary actions.</small></span><input type="checkbox" checked={settings.useWindowsAccent} onchange={(event) => { settings.useWindowsAccent = event.currentTarget.checked; void applyAccent(settings.useWindowsAccent); }} aria-label="Use Windows accent color" /></label><label class="setting-row"><span><strong>Reduce motion</strong><small>Disable non-essential interface transitions.</small></span><input type="checkbox" checked={settings.reduceMotion} onchange={(event) => { settings.reduceMotion = event.currentTarget.checked; applyReducedMotion(settings.reduceMotion); }} aria-label="Reduce interface motion" /></label>
          {:else if settingsCategory === "sources"}<h2>Driver sources</h2><p class="category-note">Sources contribute evidence; none overrides hardware suitability by name alone.</p>{#each allSources as source}{@const health = sourceHealth.find((entry) => entry.source === source)}<label class="source-setting-row"><span><strong>{sourceLabel(source)}</strong><small>{health ? `${sourceStateLabel(health.state)} · checked ${new Date(health.checkedAt * 1000).toLocaleString()} · ${health.candidateCount} candidates${health.cached ? " · cached" : ""}` : "Not checked yet"}</small>{#if health?.message}<small>{health.message}</small>{/if}</span><input type="checkbox" checked={settings.enabledSources.includes(source)} onchange={(event) => updateSource(source, event.currentTarget.checked)} aria-label={`Use ${sourceLabel(source)} source`} /></label>{/each}
          {:else if settingsCategory === "safety"}<h2>Safety &amp; rollback</h2><p class="category-note">These defaults apply to every reviewed installation.</p><label class="setting-row"><span><strong>Create a restore point</strong><small>Ask Windows for a system checkpoint before driver changes.</small></span><input type="checkbox" bind:checked={settings.createRestorePoint} aria-label="Create restore point before installation" /></label><label class="setting-row"><span><strong>Back up current package</strong><small>Export the current OEM package when Windows permits it.</small></span><input type="checkbox" bind:checked={settings.backupCurrentPackage} aria-label="Back up current driver package" /></label><label class="setting-row"><span><strong>Confirm optional drivers</strong><small>Keep an explicit review requirement for optional packages.</small></span><input type="checkbox" bind:checked={settings.confirmOptionalDrivers} aria-label="Require confirmation for optional drivers" /></label><label class="setting-row"><span><strong>Offer rollback after failure</strong><small>Show rollback only when a preserved prior package makes it real.</small></span><input type="checkbox" bind:checked={settings.offerRollbackAfterFailure} aria-label="Offer rollback after failed installation" /></label>
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
        <section class="review-safety"><h3>Safety</h3><p><span>✓</span> The package will be hashed with SHA-256 and its Windows signature verified.</p><p><span>✓</span> Windows will select the matching INF without a force-install flag.</p><p><span>{savedSettings.createRestorePoint ? "✓" : "—"}</span> {savedSettings.createRestorePoint ? "A restore point will be attempted." : "Restore-point creation is disabled."}</p><p><span>{savedSettings.backupCurrentPackage ? "✓" : "—"}</span> {savedSettings.backupCurrentPackage ? "The current package will be exported when available." : "Current-package backup is disabled."}</p></section>
        {#if installError}<div class="inline-error" role="alert"><strong>Could not start installation</strong><span>{installError}</span></div>{/if}
        <footer><button disabled={committingInstall} onclick={closeInstallReview}>Cancel</button><button id="confirm-install" class="primary-button" disabled={committingInstall} onclick={commitReviewedInstall}>{committingInstall ? "Starting" : `Install ${installReview.items.length} driver${installReview.items.length === 1 ? "" : "s"}`}</button></footer>
      </div>
    </div>
  {/if}

  <footer class="status-bar" class:operation={installStatus.phase !== "idle"}>
    <span class="ready-dot" class:error={error !== null || candidateError !== null || installStatus.phase === "failed"}></span>
    <span>{installStatus.phase !== "idle" ? installStatus.message : loading ? "Inspecting devices" : candidateLoading ? "Checking driver sources" : error ? "Inventory unavailable" : "Ready"}</span>
    {#if installStatus.phase !== "idle"}
      <div class="operation-progress" role="progressbar" aria-label="Driver installation progress" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(installStatus.progress * 100)}><span style:width={`${Math.round(installStatus.progress * 100)}%`}></span></div>
      <span>{installStatus.completedItems} of {installStatus.totalItems}</span>
      {#if installStatus.cancellable}<button onclick={cancelInstall}>Cancel</button>{/if}
      {#if installStatus.phase === "completed" && installHistory[0]?.rollbackAvailable}<button onclick={() => rollback(installHistory[0])}>Rollback</button>{/if}
    {:else}
      <span class="separator"></span><span>{devices.length ? `${devices.length} present devices` : "No inventory loaded"}</span><span class="status-time">{lastScanned ? `Scanned ${lastScanned.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}` : "Local machine"}</span>
    {/if}
  </footer>
</div>

<style>
  :global(#svelte) { height: 100%; }
  svg { width: 18px; height: 18px; fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round; }
  .shell { width: 100%; height: 100%; display: flex; flex-direction: column; overflow: hidden; border: 1px solid var(--stroke-strong); border-radius: var(--radius-window); background: var(--surface-shell); box-shadow: 0 14px 38px rgba(0,0,0,.18); backdrop-filter: blur(34px) saturate(125%); }
  .titlebar { height: 40px; flex: none; display: flex; align-items: center; justify-content: space-between; border-bottom: 1px solid var(--stroke); user-select: none; }
  .brand { height: 100%; display: flex; align-items: center; gap: 9px; padding-left: 13px; font-size: 12px; }
  .brand-mark { display: grid; place-items: center; width: 20px; height: 20px; border-radius: 5px; background: var(--accent); color: #fff; font-size: 12px; font-weight: 700; }
  .window-controls { height: 100%; display: flex; }
  .window-controls button { width: 46px; border: 0; background: transparent; display: grid; place-items: center; }
  .window-controls button:hover:not(:disabled) { background: var(--surface-hover); }
  .window-controls .close { border-radius: 0 var(--radius-window) 0 0; }
  .window-controls .close:hover { color: white; background: #c42b1c; }
  .window-controls button:disabled { color: var(--text-tertiary); opacity: .42; }
  .window-controls svg { width: 15px; height: 15px; stroke-width: 1.35; }
  .workspace { position: relative; flex: 1; min-height: 0; display: flex; }
  .navigation { width: 184px; flex: none; display: flex; flex-direction: column; padding: 12px 8px; border-right: 1px solid var(--stroke); background: var(--surface-nav); }
  .nav-label { margin: 3px 11px 10px; color: var(--text-tertiary); font-size: 11px; font-weight: 600; letter-spacing: .08em; text-transform: uppercase; }
  .navigation > button { position: relative; width: 100%; height: 38px; display: flex; align-items: center; gap: 11px; padding: 0 12px; border: 0; border-radius: var(--radius-control); color: var(--text-secondary); background: transparent; text-align: left; }
  .navigation > button:hover { color: var(--text-primary); background: var(--surface-hover); }
  .navigation > button[aria-current="page"] { color: var(--text-primary); background: var(--surface-selected); font-weight: 600; }
  .navigation > button:last-of-type { margin-top: auto; }
  .selection-indicator { position: absolute; left: 0; width: 3px; height: 16px; border-radius: 2px; background: transparent; }
  [aria-current="page"] .selection-indicator { background: var(--accent); }
  .nav-note { margin: 10px 8px 6px; padding-top: 14px; border-top: 1px solid var(--stroke); display: flex; flex-direction: column; gap: 3px; font-size: 11px; }
  .nav-note span { color: var(--text-tertiary); line-height: 1.4; }
  .content { flex: 1; min-width: 0; min-height: 0; background: var(--surface-content); }
  .page { height: 100%; min-height: 0; display: flex; flex-direction: column; }
  .page-header { min-height: 76px; flex: none; display: flex; align-items: center; justify-content: space-between; padding: 14px 22px; border-bottom: 1px solid var(--stroke); }
  h1 { margin: 0; font-family: "Segoe UI Variable Display", "Segoe UI", sans-serif; font-size: 24px; font-weight: 620; letter-spacing: -.02em; }
  .page-header p { margin: 3px 0 0; color: var(--text-tertiary); font-size: 12px; }
  .primary-button { height: 34px; display: flex; align-items: center; gap: 8px; padding: 0 13px; border: 1px solid color-mix(in srgb, var(--accent) 68%, transparent); border-radius: var(--radius-control); background: var(--accent); color: #fff; font-weight: 600; }
  :global(:root[data-theme="dark"]) .primary-button { color: #101215; }
  .primary-button:hover:not(:disabled) { background: var(--accent-hover); }
  .primary-button:disabled { opacity: .65; }
  .summary-bar { min-height: 44px; flex: none; display: flex; align-items: center; gap: 10px; padding: 0 22px; border-bottom: 1px solid var(--stroke); color: var(--text-secondary); font-size: 12px; }
  .summary-bar strong { color: var(--text-primary); }
  .separator { width: 1px; height: 13px; background: var(--stroke-strong); }
  .summary-copy { margin-left: auto; max-width: 450px; color: var(--text-tertiary); text-align: right; }
  .attention { color: var(--error); font-weight: 600; }
  .command-bar { min-height: 48px; flex: none; display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 7px 16px; border-bottom: 1px solid var(--stroke); }
  .search-box { width: min(390px, 52%); height: 32px; display: flex; align-items: center; gap: 8px; padding: 0 10px; border: 1px solid var(--stroke-strong); border-radius: var(--radius-control); background: var(--surface-control); color: var(--text-tertiary); }
  .search-box:focus-within { outline: 2px solid var(--focus); outline-offset: 1px; }
  .search-box svg { width: 15px; height: 15px; flex: none; }
  .search-box input { width: 100%; min-width: 0; border: 0; outline: 0; background: transparent; }
  .search-box input::placeholder { color: var(--text-tertiary); }
  .filter-control { display: flex; align-items: center; gap: 8px; color: var(--text-tertiary); font-size: 11px; }
  .filter-control select { min-width: 145px; }
  .split-view { flex: 1; min-height: 0; display: grid; grid-template-columns: minmax(0, 1fr); }
  .split-view.details-open { grid-template-columns: minmax(420px, 1fr) 360px; }
  .device-list { min-width: 0; overflow-y: auto; }
  .list-header, .device-row { display: grid; grid-template-columns: minmax(250px, 1.5fr) minmax(110px, .55fr) minmax(145px, .75fr); align-items: center; column-gap: 14px; }
  .list-header { position: sticky; top: 0; z-index: 1; height: 31px; padding: 0 16px; border-bottom: 1px solid var(--stroke); background: color-mix(in srgb, var(--surface-content) 96%, transparent); color: var(--text-tertiary); font-size: 11px; font-weight: 600; }
  .device-row { width: 100%; min-height: 53px; padding: 6px 16px; border: 0; border-bottom: 1px solid var(--stroke); background: var(--surface-row); color: var(--text-secondary); text-align: left; }
  .device-row:hover { background: var(--surface-hover); }
  .device-row.selected { background: var(--surface-selected); box-shadow: inset 3px 0 var(--accent); }
  .device-identity { min-width: 0; display: flex; align-items: center; gap: 10px; }
  .device-identity > span:last-child { min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .device-identity strong, .device-identity small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .device-identity strong { color: var(--text-primary); font-size: 13px; font-weight: 600; }
  .device-identity small { color: var(--text-tertiary); font: 10px/1.3 "Cascadia Code", Consolas, monospace; }
  .device-icon { width: 28px; height: 28px; flex: none; display: grid; place-items: center; border: 1px solid var(--stroke); border-radius: var(--radius-control); color: var(--text-secondary); background: var(--surface-control); }
  .condition { display: flex; align-items: center; gap: 7px; color: var(--text-secondary); font-size: 11px; }
  .condition > span { width: 7px; height: 7px; flex: none; border-radius: 50%; background: var(--success); }
  .condition.problem, .condition.missing { color: var(--error); font-weight: 600; }
  .condition.problem > span, .condition.missing > span, .status-dot.problem, .status-dot.missing { background: var(--error); }
  .driver-cell { min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .driver-cell strong, .driver-cell small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .driver-cell strong { color: var(--text-primary); font: 11.5px "Cascadia Code", Consolas, monospace; font-variant-numeric: tabular-nums; }
  .driver-cell small { color: var(--text-tertiary); font-size: 10px; }
  .filtered-empty { padding: 36px 18px; color: var(--text-tertiary); text-align: center; }
  .details-pane { min-width: 0; display: flex; flex-direction: column; border-left: 1px solid var(--stroke); background: color-mix(in srgb, var(--surface-content) 96%, transparent); }
  .details-pane > header { min-height: 64px; flex: none; display: flex; align-items: center; justify-content: space-between; gap: 10px; padding: 10px 13px 10px 16px; border-bottom: 1px solid var(--stroke); }
  .details-pane h2 { margin: 0; font-size: 14px; font-weight: 600; line-height: 1.35; }
  .details-pane header p { margin: 2px 0 0; color: var(--text-tertiary); font-size: 11px; }
  .icon-button { width: 30px; height: 30px; flex: none; display: grid; place-items: center; border: 0; border-radius: var(--radius-control); background: transparent; }
  .icon-button:hover { background: var(--surface-hover); }
  .tabs { height: 38px; flex: none; display: flex; gap: 2px; padding: 0 12px; border-bottom: 1px solid var(--stroke); }
  .tabs button { position: relative; padding: 0 10px; border: 0; background: transparent; color: var(--text-secondary); font-size: 12px; }
  .tabs button[aria-selected="true"] { color: var(--text-primary); font-weight: 600; }
  .tabs button[aria-selected="true"]::after { content: ""; position: absolute; left: 9px; right: 9px; bottom: -1px; height: 2px; background: var(--accent); }
  .details-content { flex: 1; min-height: 0; overflow-y: auto; padding: 16px; }
  .status-line { display: flex; align-items: center; gap: 10px; padding-bottom: 15px; border-bottom: 1px solid var(--stroke); }
  .status-line > span:last-child { display: flex; flex-direction: column; }
  .status-line small { color: var(--text-tertiary); }
  .status-dot, .ready-dot { width: 7px; height: 7px; flex: none; border-radius: 50%; background: var(--success); }
  dl { margin: 5px 0 0; }
  dl div { padding: 11px 0; border-bottom: 1px solid var(--stroke); }
  dt { margin-bottom: 3px; color: var(--text-tertiary); font-size: 11px; }
  dd { margin: 0; overflow-wrap: anywhere; font-size: 12px; }
  .technical dd, .id-section li { font-family: "Cascadia Code", Consolas, monospace; font-size: 10.5px; }
  .quiet-note { margin-top: 16px; padding: 12px; border: 1px solid var(--stroke); border-radius: var(--radius-layer); background: var(--surface-row); }
  .quiet-note p, .id-section p { margin: 4px 0 0; color: var(--text-tertiary); font-size: 11px; line-height: 1.45; }
  .recommendation-panel { display: grid; gap: 4px; margin-top: 16px; padding: 12px; border: 1px solid var(--stroke-strong); border-left: 3px solid var(--text-tertiary); border-radius: var(--radius-layer); background: var(--surface-row); }
  .recommendation-panel.recommended { border-left-color: var(--accent); }
  .recommendation-panel.current { border-left-color: var(--success); }
  .recommendation-panel > span { color: var(--text-secondary); font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: .04em; }
  .recommendation-panel > strong { font: 13px "Cascadia Code", Consolas, monospace; }
  .recommendation-panel p { margin: 2px 0 0; color: var(--text-secondary); font-size: 11px; line-height: 1.45; }
  .install-action { justify-self: end; min-height: 30px; margin-top: 8px; padding: 0 11px; border: 1px solid color-mix(in srgb, var(--accent) 68%, transparent); border-radius: var(--radius-control); background: var(--accent); color: #fff; font-size: 11px; font-weight: 600; }
  .install-action:hover:not(:disabled) { background: var(--accent-hover); }
  .install-action:disabled { opacity: .55; }
  .why-driver { margin-top: 17px; }
  .why-driver h3 { margin: 0 0 6px; font-size: 12px; }
  .why-driver ul { margin: 0; padding: 0; list-style: none; }
  .why-driver li { display: grid; grid-template-columns: 18px 1fr; gap: 4px; padding: 7px 0; border-bottom: 1px solid var(--stroke); color: var(--success); }
  .why-driver li.negative { color: var(--error); }
  .why-driver li > span:last-child { display: flex; flex-direction: column; gap: 2px; color: var(--text-primary); }
  .why-driver small { color: var(--text-tertiary); font-size: 10.5px; line-height: 1.4; }
  .id-section { margin-top: 16px; }
  .driver-summary { margin-top: 17px; }
  .driver-summary h3 { margin: 0; padding-bottom: 7px; border-bottom: 1px solid var(--stroke); font-size: 12px; }
  .driver-summary p { color: var(--text-tertiary); font-size: 11px; }
  .mono { font-family: "Cascadia Code", Consolas, monospace; font-variant-numeric: tabular-nums; }
  .id-section h3 { margin: 0 0 7px; font-size: 12px; }
  .id-section ul { margin: 0; padding: 0; list-style: none; }
  .id-section li { padding: 7px 0; border-bottom: 1px solid var(--stroke); overflow-wrap: anywhere; }
  .candidate-intro { display: grid; gap: 5px; padding-bottom: 14px; border-bottom: 1px solid var(--stroke); }
  .candidate-intro p, .candidate-intro small { margin: 0; color: var(--text-tertiary); font-size: 11px; line-height: 1.45; }
  .candidate-action { justify-self: start; min-height: 30px; margin-top: 5px; padding: 0 10px; border: 1px solid var(--stroke-strong); border-radius: var(--radius-control); background: var(--surface-control); color: var(--text-primary); font-size: 11px; font-weight: 600; }
  .candidate-action:hover:not(:disabled), .candidate-actions button:hover:not(:disabled) { background: var(--surface-hover); }
  .candidate-action:disabled, .candidate-actions button:disabled { opacity: .55; }
  .inline-error { display: flex; flex-direction: column; gap: 3px; margin-top: 12px; padding: 10px; border-left: 3px solid var(--error); background: color-mix(in srgb, var(--error) 8%, transparent); font-size: 11px; }
  .inline-error span { color: var(--text-tertiary); line-height: 1.4; }
  .candidate-loading { display: flex; align-items: center; gap: 10px; padding: 18px 0; }
  .candidate-loading > span:last-child { display: flex; flex-direction: column; gap: 3px; }
  .candidate-loading small { color: var(--text-tertiary); line-height: 1.4; }
  .source-health, .candidate-list { margin-top: 16px; }
  .source-health h3, .candidate-list h3 { margin: 0; padding-bottom: 7px; border-bottom: 1px solid var(--stroke); font-size: 12px; }
  .source-health > div { min-height: 48px; display: grid; grid-template-columns: 8px minmax(0, 1fr) auto; align-items: center; gap: 8px; border-bottom: 1px solid var(--stroke); }
  .source-health > div > span:nth-child(2) { min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .source-health > div > span:last-child { color: var(--text-tertiary); font-size: 10px; }
  .source-health small { color: var(--text-tertiary); font-size: 10px; line-height: 1.35; }
  .source-dot { width: 7px; height: 7px; border-radius: 50%; background: var(--success); }
  .source-dot.failed { background: var(--error); }
  .source-dot.skipped { background: var(--text-tertiary); }
  .candidate-list h3 { display: flex; justify-content: space-between; }
  .candidate-list h3 span { color: var(--text-tertiary); font-weight: 400; }
  .candidate-row { padding: 12px 0; border-bottom: 1px solid var(--stroke); }
  .candidate-row.not-recommended { opacity: .82; }
  .candidate-heading { display: flex; align-items: flex-start; justify-content: space-between; gap: 10px; }
  .candidate-heading > span:first-child { min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .candidate-heading strong { line-height: 1.35; }
  .candidate-heading small { color: var(--text-tertiary); font-size: 10px; }
  .candidate-heading > span:last-child { flex: none; color: var(--accent); font: 10.5px "Cascadia Code", Consolas, monospace; }
  .candidate-heading > span.review { color: var(--text-secondary); }
  .candidate-rank { display: flex; flex-direction: column; align-items: flex-end; gap: 2px; }
  .candidate-rank strong { color: var(--text-secondary); font: 600 10px/1.3 "Segoe UI", sans-serif; white-space: nowrap; }
  .candidate-rank.recommended strong { color: var(--accent); }
  .candidate-rank small { font: 10.5px "Cascadia Code", Consolas, monospace; }
  .candidate-summary { margin: 7px 0 0; color: var(--text-secondary); font-size: 10.5px; line-height: 1.45; }
  .candidate-row dl { display: grid; grid-template-columns: 1fr 1fr; column-gap: 12px; margin-top: 7px; }
  .candidate-row dl div { padding: 6px 0; }
  .candidate-row dl div:last-child { grid-column: 1 / -1; }
  .candidate-row dd { font-size: 10.5px; }
  .evidence { margin: 7px 0 0; padding-left: 17px; color: var(--text-secondary); font-size: 10.5px; line-height: 1.45; }
  .candidate-products { margin: 8px 0 0; color: var(--text-tertiary); font-size: 10px; line-height: 1.4; }
  .candidate-actions { display: flex; gap: 7px; margin-top: 9px; }
  .candidate-actions button { min-height: 28px; padding: 0 9px; border: 1px solid var(--stroke-strong); border-radius: var(--radius-control); background: var(--surface-control); color: var(--text-secondary); font-size: 10.5px; }
  .candidate-empty { padding: 18px 0; color: var(--text-secondary); }
  .candidate-empty p { margin: 5px 0 0; color: var(--text-tertiary); font-size: 11px; line-height: 1.45; }
  .factor-row { min-height: 44px; display: grid; grid-template-columns: 1fr auto; align-items: center; gap: 10px; border-bottom: 1px solid var(--stroke); }
  .factor-row > span:first-child { display: flex; flex-direction: column; gap: 2px; }
  .factor-row small, .score-disclaimer { color: var(--text-tertiary); font-size: 10px; line-height: 1.35; }
  .factor-row > span:last-child { color: var(--success); font: 11px "Cascadia Code", Consolas, monospace; }
  .factor-row > span.negative { color: var(--error); }
  .score-disclaimer { display: block; margin-top: 9px; }
  .message { flex: 1; display: flex; align-items: center; justify-content: center; flex-direction: column; gap: 7px; padding: 30px; color: var(--text-tertiary); text-align: center; }
  .message strong { color: var(--text-primary); font-size: 15px; }
  .message button { margin-top: 5px; padding: 7px 12px; border: 1px solid var(--stroke-strong); border-radius: var(--radius-control); background: var(--surface-control); }
  .spinner { width: 20px; height: 20px; margin-bottom: 6px; border: 2px solid var(--stroke-strong); border-top-color: var(--accent); border-radius: 50%; animation: spin .8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  .empty-section { flex: 1; display: grid; place-content: center; justify-items: center; padding: 30px; text-align: center; }
  .empty-section svg { width: 28px; height: 28px; color: var(--text-tertiary); }
  .empty-section h2 { margin: 12px 0 4px; font-size: 16px; }
  .empty-section p { max-width: 440px; margin: 0; color: var(--text-tertiary); line-height: 1.5; }
  .history-list { flex: 1; min-height: 0; overflow-y: auto; }
  .history-header, .history-row { display: grid; grid-template-columns: minmax(250px, 1.2fr) 90px minmax(220px, 1fr) 52px; align-items: center; gap: 14px; }
  .history-header { height: 32px; padding: 0 18px; border-bottom: 1px solid var(--stroke); color: var(--text-tertiary); font-size: 11px; font-weight: 600; }
  .history-row { width: 100%; min-height: 58px; padding: 7px 18px; border: 0; border-bottom: 1px solid var(--stroke); background: var(--surface-row); color: var(--text-secondary); text-align: left; }
  .history-row:hover { background: var(--surface-hover); }
  .history-row > span:first-child { display: flex; flex-direction: column; gap: 2px; }
  .history-row strong { color: var(--text-primary); font-weight: 600; }
  .history-row small { color: var(--text-tertiary); font-size: 10px; }
  .history-row > span:last-child { color: var(--accent); font-weight: 600; text-align: right; }
  .operation-history { position: absolute; inset: 0 0 0 184px; z-index: 3; display: flex; flex-direction: column; background: var(--surface-content); }
  .operation-history > header { min-height: 76px; display: flex; align-items: center; padding: 14px 22px; border-bottom: 1px solid var(--stroke); }
  .operation-history header p { margin: 3px 0 0; color: var(--text-tertiary); font-size: 12px; }
  .history-workspace { flex: 1; min-height: 0; display: grid; grid-template-columns: minmax(0, 1fr) 360px; }
  .history-scroll { min-width: 0; overflow-y: auto; }
  .history-scroll h2 { margin: 0; padding: 13px 18px 8px; border-bottom: 1px solid var(--stroke); color: var(--text-secondary); font-size: 11px; font-weight: 700; letter-spacing: .04em; text-transform: uppercase; }
  .operation-history-head, .operation-history-row { display: grid; grid-template-columns: minmax(190px, 1fr) minmax(145px, .7fr) minmax(170px, .9fr); align-items: center; gap: 14px; }
  .operation-history-head { height: 31px; padding: 0 18px; border-bottom: 1px solid var(--stroke); color: var(--text-tertiary); font-size: 11px; font-weight: 600; }
  .operation-history-row { width: 100%; min-height: 64px; padding: 8px 18px; border: 0; border-bottom: 1px solid var(--stroke); background: var(--surface-row); color: var(--text-secondary); text-align: left; }
  .operation-history-row:hover { background: var(--surface-hover); }
  .operation-history-row.selected { background: var(--surface-selected); box-shadow: inset 3px 0 var(--accent); }
  .operation-history-row > span { min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .operation-history-row small { overflow: hidden; color: var(--text-tertiary); font-size: 10px; text-overflow: ellipsis; white-space: nowrap; }
  .stored-scan-row { min-height: 28px; border: 1px solid var(--stroke-strong); border-radius: var(--radius-control); background: var(--surface-control); }
  .stored-scan-row { width: 100%; min-height: 52px; display: flex; align-items: center; justify-content: space-between; padding: 7px 18px; border-width: 0 0 1px; border-radius: 0; text-align: left; }
  .stored-scan-row > span:first-child { display: flex; flex-direction: column; gap: 2px; }
  .stored-scan-row small { color: var(--text-tertiary); font-size: 10px; }
  .history-details { min-width: 0; display: flex; flex-direction: column; border-left: 1px solid var(--stroke); background: color-mix(in srgb, var(--surface-content) 96%, transparent); }
  .history-details > header { min-height: 64px; display: flex; align-items: center; justify-content: space-between; gap: 10px; padding: 10px 14px 10px 16px; border-bottom: 1px solid var(--stroke); }
  .history-details > header h2, .history-details h3 { margin: 0; font-size: 13px; }
  .history-details > header p { margin: 3px 0 0; color: var(--text-tertiary); font-size: 10px; }
  .history-details > header button, .standard-button { min-height: 30px; padding: 0 10px; border: 1px solid var(--stroke-strong); border-radius: var(--radius-control); background: var(--surface-control); }
  .history-details > div { overflow-y: auto; padding: 14px 16px; }
  .history-details section + section { margin-top: 18px; }
  .history-details section > p { color: var(--text-secondary); font-size: 11px; line-height: 1.45; }
  .wrap { overflow-wrap: anywhere; word-break: break-word; }
  .settings-workspace { position: absolute; inset: 0 0 0 184px; z-index: 4; display: flex; flex-direction: column; background: var(--surface-content); }
  .settings-workspace > header { min-height: 76px; display: flex; align-items: center; padding: 14px 22px; border-bottom: 1px solid var(--stroke); }
  .settings-workspace > header p { margin: 3px 0 0; color: var(--text-tertiary); font-size: 12px; }
  .settings-layout { flex: 1; min-height: 0; display: grid; grid-template-columns: 208px minmax(0, 1fr); }
  .settings-layout > nav { display: flex; flex-direction: column; gap: 2px; padding: 12px; border-right: 1px solid var(--stroke); background: var(--surface-nav); }
  .settings-layout > nav button { position: relative; min-height: 38px; padding: 0 12px; border: 0; border-radius: var(--radius-control); background: transparent; color: var(--text-secondary); text-align: left; }
  .settings-layout > nav button:hover { background: var(--surface-hover); color: var(--text-primary); }
  .settings-layout > nav button[aria-current="page"] { background: var(--surface-selected); color: var(--text-primary); font-weight: 600; }
  .settings-layout > nav button > span { position: absolute; left: 0; width: 3px; height: 16px; border-radius: 2px; }
  .settings-layout > nav button[aria-current="page"] > span { background: var(--accent); }
  .settings-panel { min-width: 0; overflow-y: auto; padding: 20px 24px 90px; }
  .settings-panel > h2 { margin: 0; font-size: 15px; }
  .category-note { margin: 4px 0 12px; color: var(--text-tertiary); font-size: 11px; line-height: 1.45; }
  .source-setting-row, .management-row { min-height: 64px; display: flex; align-items: center; justify-content: space-between; gap: 18px; border-bottom: 1px solid var(--stroke); }
  .source-setting-row > span, .management-row > span:first-child { min-width: 0; display: flex; flex-direction: column; gap: 3px; }
  .source-setting-row small, .management-row small { color: var(--text-tertiary); font-size: 10.5px; line-height: 1.35; }
  .management-row button, .row-actions button { min-height: 30px; padding: 0 10px; border: 1px solid var(--stroke-strong); border-radius: var(--radius-control); background: var(--surface-control); }
  .row-actions { display: flex; gap: 6px; }
  .activity-log { max-height: 220px; margin: 12px 0 0; padding: 10px; overflow: auto; border: 1px solid var(--stroke); border-radius: var(--radius-control); background: var(--surface-row); font: 10.5px/1.5 "Cascadia Code", Consolas, monospace; white-space: pre-wrap; }
  .about-details { max-width: 640px; }
  .settings-save-bar { position: absolute; right: 0; bottom: 0; left: 208px; min-height: 64px; display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 10px 24px; border-top: 1px solid var(--stroke-strong); background: var(--surface-nav); box-shadow: 0 -8px 22px rgba(0, 0, 0, .08); }
  .settings-save-bar > span { display: flex; gap: 8px; }
  .settings-save-bar > span:first-child { flex-direction: column; gap: 2px; }
  .settings-save-bar small { color: var(--text-tertiary); }
  .settings-save-bar button { min-height: 32px; padding: 0 12px; border: 1px solid var(--stroke-strong); border-radius: var(--radius-control); background: var(--surface-control); }
  .settings-save-bar .primary-button { background: var(--accent); }
  :global(:root:not([data-show-exact-ids="true"])) .technical-identifiers { display: none; }
  :global(:root:not([data-show-scores="true"])) .ranking-technical dl div:not(:first-child), :global(:root:not([data-show-scores="true"])) .ranking-technical .factor-row > span:last-child { display: none; }
  .settings-content { width: min(720px, calc(100% - 44px)); margin: 20px 22px; }
  .settings-content section { margin-bottom: 20px; }
  .settings-content h2 { margin: 0; padding-bottom: 9px; border-bottom: 1px solid var(--stroke); font-size: 13px; }
  .setting-row { min-height: 61px; display: flex; align-items: center; justify-content: space-between; gap: 20px; border-bottom: 1px solid var(--stroke); }
  .setting-row > span:first-child { display: flex; flex-direction: column; gap: 3px; }
  .setting-row small { color: var(--text-tertiary); }
  .settings-note { margin: 9px 0 0; color: var(--text-tertiary); font-size: 12px; line-height: 1.45; }
  select { min-width: 120px; height: 32px; padding: 0 9px; border: 1px solid var(--stroke-strong); border-radius: var(--radius-control); background: var(--surface-control); }
  input[type="checkbox"] { width: 18px; height: 18px; accent-color: var(--accent); }
  .status-badge { padding: 3px 7px; border-radius: 10px; color: var(--text-secondary); background: var(--surface-control); font-size: 11px; }
  .status-bar { height: 26px; flex: none; display: flex; align-items: center; gap: 8px; padding: 0 11px; border-top: 1px solid var(--stroke); background: var(--surface-nav); color: var(--text-tertiary); font-size: 11px; }
  .status-bar.operation { height: 48px; color: var(--text-secondary); }
  .status-bar button { min-height: 28px; padding: 0 9px; border: 1px solid var(--stroke-strong); border-radius: var(--radius-control); background: var(--surface-control); }
  .operation-progress { width: 210px; height: 4px; margin-left: auto; overflow: hidden; border-radius: 2px; background: var(--stroke-strong); }
  .operation-progress span { display: block; height: 100%; background: var(--accent); transition: width 160ms ease-out; }
  .dialog-backdrop { position: fixed; inset: 0; z-index: 20; display: grid; place-items: center; border-radius: var(--radius-window); background: rgba(0, 0, 0, .36); }
  .install-dialog { width: 500px; max-height: 610px; overflow-y: auto; border: 1px solid var(--stroke-strong); border-radius: var(--radius-layer); background: var(--surface-content); box-shadow: 0 18px 48px rgba(0, 0, 0, .32); }
  .install-dialog > header { padding: 18px 20px 14px; border-bottom: 1px solid var(--stroke); }
  .install-dialog h2 { margin: 0; font-size: 17px; }
  .install-dialog header p { margin: 4px 0 0; color: var(--text-tertiary); font-size: 12px; }
  .review-items { padding: 4px 20px; }
  .review-items > div { display: grid; grid-template-columns: 1fr auto; gap: 3px 12px; padding: 12px 0; border-bottom: 1px solid var(--stroke); }
  .review-items small { grid-column: 1 / -1; color: var(--text-tertiary); }
  .review-safety { padding: 12px 20px 14px; }
  .review-safety h3 { margin: 0 0 8px; font-size: 12px; }
  .review-safety p { display: grid; grid-template-columns: 18px 1fr; margin: 7px 0; color: var(--text-secondary); font-size: 11px; line-height: 1.4; }
  .review-safety p span { color: var(--success); }
  .install-dialog > footer { display: flex; justify-content: flex-end; gap: 8px; padding: 12px 20px; border-top: 1px solid var(--stroke); }
  .install-dialog > footer > button { min-height: 32px; padding: 0 12px; border: 1px solid var(--stroke-strong); border-radius: var(--radius-control); background: var(--surface-control); }
  .install-dialog > footer > .primary-button { background: var(--accent); color: white; }
  .ready-dot.error { background: var(--error); }
  .status-time { margin-left: auto; }
</style>
