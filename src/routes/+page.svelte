<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import type { AppearanceSettings, DetailTab, Device, DriverFilter, InventorySnapshot, NavigationSection, ScanSummary, SignatureStatus, ThemePreference } from "$lib/types";

  const SETTINGS_KEY = "drvmatch.appearance";
  const defaultSettings: AppearanceSettings = { theme: "system", acrylic: true };

  let section = $state<NavigationSection>("drivers");
  let detailTab = $state<DetailTab>("overview");
  let devices = $state<Device[]>([]);
  let selectedId = $state<string | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let settings = $state<AppearanceSettings>({ ...defaultSettings });
  let lastScanned = $state<Date | null>(null);
  let scans = $state<ScanSummary[]>([]);
  let historyError = $state<string | null>(null);
  let activeSummary = $state<ScanSummary | null>(null);
  let viewingStoredScan = $state(false);
  let search = $state("");
  let filter = $state<DriverFilter>("all");

  const selectedDevice = $derived(devices.find((device) => device.instanceId === selectedId) ?? null);
  const classCount = $derived(new Set(devices.map((device) => device.className).filter(Boolean)).size);
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

  function persistSettings(): void {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
  }

  function updateTheme(value: ThemePreference): void {
    settings.theme = value;
    applyTheme(value);
    persistSettings();
  }

  async function updateAcrylic(enabled: boolean): Promise<void> {
    const previous = settings.acrylic;
    settings.acrylic = enabled;
    document.documentElement.dataset.material = enabled ? "acrylic" : "solid";
    try {
      if (isTauri()) await invoke("set_acrylic", { enabled });
      persistSettings();
    } catch (cause) {
      settings.acrylic = previous;
      document.documentElement.dataset.material = previous ? "acrylic" : "solid";
      error = String(cause);
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
    if (device.condition === "problem") return `Problem${device.problemCode ? ` · code ${device.problemCode}` : ""}`;
    if (device.condition === "missing") return "Missing";
    return "Current";
  }

  function handleWindowKeydown(event: KeyboardEvent): void {
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

  onMount(() => {
    try {
      const stored = JSON.parse(localStorage.getItem(SETTINGS_KEY) ?? "null") as Partial<AppearanceSettings> | null;
      settings = {
        theme: stored?.theme === "light" || stored?.theme === "dark" || stored?.theme === "system" ? stored.theme : "system",
        acrylic: stored?.acrylic ?? true,
      };
    } catch {
      settings = { ...defaultSettings };
    }
    applyTheme(settings.theme);
    document.documentElement.dataset.material = settings.acrylic ? "acrylic" : "solid";
    if (isTauri()) {
      void invoke("set_acrylic", { enabled: settings.acrylic }).catch((cause) => {
        error = String(cause);
      });
    }
    const systemTheme = matchMedia("(prefers-color-scheme: dark)");
    const themeListener = () => settings.theme === "system" && applyTheme("system");
    systemTheme.addEventListener("change", themeListener);
    window.addEventListener("keydown", handleWindowKeydown);
    void refreshHistory();
    void scanDevices();
    return () => {
      systemTheme.removeEventListener("change", themeListener);
      window.removeEventListener("keydown", handleWindowKeydown);
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
        <button aria-current={section === item ? "page" : undefined} onclick={() => section = item}>
          <span class="selection-indicator"></span><svg viewBox="0 0 24 24" aria-hidden="true"><path d={iconPath(item)} /></svg><span>{item[0].toUpperCase() + item.slice(1)}</span>
        </button>
      {/each}
      <div class="nav-note"><strong>Suitability first</strong><span>Newer does not always mean better.</span></div>
    </nav>

    <main class="content">
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
                  <button class="device-row" class:selected={selectedId === device.instanceId} role="option" aria-selected={selectedId === device.instanceId} onclick={() => { selectedId = device.instanceId; detailTab = "overview"; }}>
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
                  <div class="tabs" role="tablist" aria-label="Device information">{#each (["overview", "technical"] as DetailTab[]) as tab}<button role="tab" aria-selected={detailTab === tab} onclick={() => detailTab = tab}>{tab[0].toUpperCase() + tab.slice(1)}</button>{/each}</div>
                  <div class="details-content" role="tabpanel">
                    {#if detailTab === "overview"}
                      <div class="status-line"><span class="status-dot" class:problem={selectedDevice.condition === "problem"} class:missing={selectedDevice.condition === "missing"}></span><span><strong>{conditionLabel(selectedDevice)}</strong><small>{selectedDevice.condition === "current" ? "Windows reports an installed driver and no device problem." : selectedDevice.condition === "missing" ? "No installed driver package was associated with this hardware device." : "Windows reports a problem for this device."}</small></span></div>
                      <dl><div><dt>Description</dt><dd>{selectedDevice.description}</dd></div><div><dt>Manufacturer</dt><dd>{selectedDevice.manufacturer ?? "Not reported"}</dd></div><div><dt>Device class</dt><dd>{selectedDevice.className ?? "Not reported"}</dd></div></dl>
                      <section class="driver-summary"><h3>Installed driver</h3>{#if selectedDevice.installedDriver}<dl><div><dt>Provider</dt><dd>{selectedDevice.installedDriver.provider ?? "Not reported"}</dd></div><div><dt>Version</dt><dd class="mono">{selectedDevice.installedDriver.version ?? "Not reported"}</dd></div><div><dt>INF driver date</dt><dd>{formatDriverDate(selectedDevice.installedDriver.driverDate)}</dd></div><div><dt>Signature</dt><dd>{signatureLabel(selectedDevice.installedDriver.signature)}{selectedDevice.installedDriver.signer ? ` · ${selectedDevice.installedDriver.signer}` : ""}</dd></div></dl>{:else}<p>Windows did not associate an installed package with this hardware device.</p>{/if}</section>
                      {#if selectedDevice.installedDriver?.genericMicrosoft}<div class="quiet-note"><strong>Generic Microsoft driver detected</strong><p>The installed package explicitly identifies itself as generic. This is informational; DrvMatch has not yet compared alternatives.</p></div>{:else}<div class="quiet-note"><strong>No recommendation yet</strong><p>This inventory describes what is installed. Candidate discovery and suitability ranking begin in later milestones.</p></div>{/if}
                    {:else}
                      <dl class="technical"><div><dt>Device instance ID</dt><dd>{selectedDevice.instanceId}</dd></div><div><dt>Class GUID</dt><dd>{selectedDevice.classGuid ?? "Not reported"}</dd></div><div><dt>Problem code</dt><dd>{selectedDevice.problemCode ?? "None"}</dd></div><div><dt>Problem status</dt><dd>{selectedDevice.problemStatus === null ? "None" : `0x${(selectedDevice.problemStatus >>> 0).toString(16).padStart(8, "0")}`}</dd></div></dl>
                      {#if selectedDevice.installedDriver}<section class="id-section"><h3>Installed package</h3><dl class="technical"><div><dt>Published INF</dt><dd>{selectedDevice.installedDriver.publishedInfName ?? "Not reported"}</dd></div><div><dt>INF path</dt><dd>{selectedDevice.installedDriver.infPath ?? "Not reported"}</dd></div><div><dt>INF section</dt><dd>{selectedDevice.installedDriver.infSection ?? "Not reported"}</dd></div><div><dt>Matching ID</dt><dd>{selectedDevice.installedDriver.matchingId ?? "Not reported"}</dd></div><div><dt>Driver key</dt><dd>{selectedDevice.installedDriver.driverKey ?? "Not reported"}</dd></div><div><dt>Windows driver rank</dt><dd>{selectedDevice.installedDriver.driverRank === null ? "Not reported" : `0x${selectedDevice.installedDriver.driverRank.toString(16).padStart(8, "0")}`}</dd></div><div><dt>Signature class</dt><dd>{signatureLabel(selectedDevice.installedDriver.signature)}</dd></div><div><dt>INF signature verified</dt><dd>{selectedDevice.installedDriver.infSignatureVerified ? "Yes" : "Not verified"}</dd></div><div><dt>Signer</dt><dd>{selectedDevice.installedDriver.signer ?? "Not reported"}</dd></div><div><dt>Catalog / store identity</dt><dd>{selectedDevice.installedDriver.catalogFile ?? "Not reported"}</dd></div></dl></section>{/if}
                      <section class="id-section"><h3>Hardware IDs</h3>{#if selectedDevice.hardwareIds.length}<ul>{#each selectedDevice.hardwareIds as id}<li>{id}</li>{/each}</ul>{:else}<p>Windows did not expose hardware IDs for this device.</p>{/if}</section>
                      <section class="id-section"><h3>Compatible IDs</h3>{#if selectedDevice.compatibleIds.length}<ul>{#each selectedDevice.compatibleIds as id}<li>{id}</li>{/each}</ul>{:else}<p>Windows did not expose compatible IDs for this device.</p>{/if}</section>
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
        <section class="page"><header class="page-header"><div><h1>Settings</h1><p>Appearance and application behavior</p></div></header><div class="settings-content"><section><h2>Appearance</h2><label class="setting-row"><span><strong>Theme</strong><small>Follow Windows or choose a fixed appearance.</small></span><select value={settings.theme} onchange={(event) => updateTheme(event.currentTarget.value as ThemePreference)} aria-label="Application theme"><option value="system">System</option><option value="light">Light</option><option value="dark">Dark</option></select></label><label class="setting-row"><span><strong>Acrylic backdrop</strong><small>Use the Windows acrylic material behind the application shell.</small></span><input type="checkbox" checked={settings.acrylic} onchange={(event) => updateAcrylic(event.currentTarget.checked)} aria-label="Use acrylic backdrop" /></label></section><section><h2>About</h2><div class="setting-row"><span><strong>DrvMatch 0.2.0</strong><small>Find the right driver for this machine, not simply the newest driver.</small></span><span class="status-badge">Installed driver inventory</span></div></section></div></section>
      {/if}
    </main>
  </div>

  <footer class="status-bar"><span class="ready-dot" class:error={error !== null}></span><span>{loading ? "Inspecting devices" : error ? "Inventory unavailable" : "Ready"}</span><span class="separator"></span><span>{devices.length ? `${devices.length} present devices` : "No inventory loaded"}</span><span class="status-time">{lastScanned ? `Scanned ${lastScanned.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}` : "Local machine"}</span></footer>
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
  .workspace { flex: 1; min-height: 0; display: flex; }
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
  .id-section { margin-top: 16px; }
  .driver-summary { margin-top: 17px; }
  .driver-summary h3 { margin: 0; padding-bottom: 7px; border-bottom: 1px solid var(--stroke); font-size: 12px; }
  .driver-summary p { color: var(--text-tertiary); font-size: 11px; }
  .mono { font-family: "Cascadia Code", Consolas, monospace; font-variant-numeric: tabular-nums; }
  .id-section h3 { margin: 0 0 7px; font-size: 12px; }
  .id-section ul { margin: 0; padding: 0; list-style: none; }
  .id-section li { padding: 7px 0; border-bottom: 1px solid var(--stroke); overflow-wrap: anywhere; }
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
  .settings-content { width: min(720px, calc(100% - 44px)); margin: 20px 22px; }
  .settings-content section { margin-bottom: 20px; }
  .settings-content h2 { margin: 0; padding-bottom: 9px; border-bottom: 1px solid var(--stroke); font-size: 13px; }
  .setting-row { min-height: 61px; display: flex; align-items: center; justify-content: space-between; gap: 20px; border-bottom: 1px solid var(--stroke); }
  .setting-row > span:first-child { display: flex; flex-direction: column; gap: 3px; }
  .setting-row small { color: var(--text-tertiary); }
  select { min-width: 120px; height: 32px; padding: 0 9px; border: 1px solid var(--stroke-strong); border-radius: var(--radius-control); background: var(--surface-control); }
  input[type="checkbox"] { width: 18px; height: 18px; accent-color: var(--accent); }
  .status-badge { padding: 3px 7px; border-radius: 10px; color: var(--text-secondary); background: var(--surface-control); font-size: 11px; }
  .status-bar { height: 26px; flex: none; display: flex; align-items: center; gap: 8px; padding: 0 11px; border-top: 1px solid var(--stroke); background: var(--surface-nav); color: var(--text-tertiary); font-size: 11px; }
  .ready-dot.error { background: var(--error); }
  .status-time { margin-left: auto; }
</style>
