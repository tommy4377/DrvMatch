<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import type { AppearanceSettings, DetailTab, Device, NavigationSection, ThemePreference } from "$lib/types";

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

  const selectedDevice = $derived(devices.find((device) => device.instanceId === selectedId) ?? null);
  const classCount = $derived(new Set(devices.map((device) => device.className).filter(Boolean)).size);

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
      devices = await invoke<Device[]>("enumerate_devices");
      lastScanned = new Date();
      if (!devices.some((device) => device.instanceId === selectedId)) selectedId = devices[0]?.instanceId ?? null;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
      devices = [];
      selectedId = null;
    } finally {
      loading = false;
    }
  }

  function handleWindowKeydown(event: KeyboardEvent): void {
    if (event.key === "F5" && section === "drivers") {
      event.preventDefault();
      void scanDevices();
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
          <div class="summary-bar" aria-live="polite"><span><strong>{devices.length}</strong> present devices</span><span class="separator"></span><span>{classCount} device classes</span><span class="summary-copy">Recommendations appear only after installed packages and trusted candidates can be verified.</span></div>

          {#if error}
            <div class="message" role="alert"><strong>Device inventory unavailable</strong><span>{error}</span><button onclick={scanDevices}>Try again</button></div>
          {:else if loading}
            <div class="message" role="status"><span class="spinner"></span><strong>Inspecting Windows devices</strong><span>Reading present Plug and Play devices through Windows SetupAPI.</span></div>
          {:else if devices.length === 0}
            <div class="message"><strong>No present devices were returned</strong><span>Run the scan again. DrvMatch has not inferred or fabricated any inventory.</span></div>
          {:else}
            <div class="split-view" class:details-open={selectedDevice !== null}>
              <div class="device-list" role="listbox" aria-label="Detected devices">
                <div class="list-header"><span>Device</span><span>Class</span><span>Manufacturer</span></div>
                {#each devices as device (device.instanceId)}
                  <button class="device-row" class:selected={selectedId === device.instanceId} role="option" aria-selected={selectedId === device.instanceId} onclick={() => { selectedId = device.instanceId; detailTab = "overview"; }}>
                    <span class="device-identity"><span class="device-icon"><svg viewBox="0 0 24 24" aria-hidden="true"><path d={iconPath("device")} /></svg></span><span><strong>{device.friendlyName}</strong><small>{device.instanceId}</small></span></span><span>{device.className ?? "Other"}</span><span>{device.manufacturer ?? "Not reported"}</span>
                  </button>
                {/each}
              </div>

              {#if selectedDevice}
                <aside class="details-pane" aria-label="Device details">
                  <header><div><h2>{selectedDevice.friendlyName}</h2><p>{selectedDevice.className ?? "Other device"}</p></div><button class="icon-button" aria-label="Close device details" onclick={() => selectedId = null}><svg viewBox="0 0 24 24" aria-hidden="true"><path d={iconPath("close")} /></svg></button></header>
                  <div class="tabs" role="tablist" aria-label="Device information">{#each (["overview", "technical"] as DetailTab[]) as tab}<button role="tab" aria-selected={detailTab === tab} onclick={() => detailTab = tab}>{tab[0].toUpperCase() + tab.slice(1)}</button>{/each}</div>
                  <div class="details-content" role="tabpanel">
                    {#if detailTab === "overview"}
                      <div class="status-line"><span class="status-dot"></span><span><strong>Present</strong><small>Windows reports this device as connected.</small></span></div>
                      <dl><div><dt>Description</dt><dd>{selectedDevice.description}</dd></div><div><dt>Manufacturer</dt><dd>{selectedDevice.manufacturer ?? "Not reported"}</dd></div><div><dt>Device class</dt><dd>{selectedDevice.className ?? "Not reported"}</dd></div></dl>
                      <div class="quiet-note"><strong>No recommendation yet</strong><p>DrvMatch will not call a driver current, missing, or recommended until installed package evidence is available.</p></div>
                    {:else}
                      <dl class="technical"><div><dt>Device instance ID</dt><dd>{selectedDevice.instanceId}</dd></div><div><dt>Class GUID</dt><dd>{selectedDevice.classGuid ?? "Not reported"}</dd></div></dl>
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
        <section class="page"><header class="page-header"><div><h1>History</h1><p>Scans and driver changes</p></div></header><div class="empty-section"><svg viewBox="0 0 24 24" aria-hidden="true"><path d={iconPath("history")} /></svg><h2>No history recorded</h2><p>Scan persistence and installation history arrive with the installed-driver inventory. Nothing has been recorded yet.</p></div></section>
      {:else}
        <section class="page"><header class="page-header"><div><h1>Settings</h1><p>Appearance and application behavior</p></div></header><div class="settings-content"><section><h2>Appearance</h2><label class="setting-row"><span><strong>Theme</strong><small>Follow Windows or choose a fixed appearance.</small></span><select value={settings.theme} onchange={(event) => updateTheme(event.currentTarget.value as ThemePreference)} aria-label="Application theme"><option value="system">System</option><option value="light">Light</option><option value="dark">Dark</option></select></label><label class="setting-row"><span><strong>Acrylic backdrop</strong><small>Use the Windows acrylic material behind the application shell.</small></span><input type="checkbox" checked={settings.acrylic} onchange={(event) => updateAcrylic(event.currentTarget.checked)} aria-label="Use acrylic backdrop" /></label></section><section><h2>About</h2><div class="setting-row"><span><strong>DrvMatch 0.1.0</strong><small>Find the right driver for this machine, not simply the newest driver.</small></span><span class="status-badge">Early build</span></div></section></div></section>
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
  .selection-indicator { position: absolute; left: 0; width: 3px; height: 16px; border-radius: 2px; background: transparent; }
  [aria-current="page"] .selection-indicator { background: var(--accent); }
  .nav-note { margin: auto 8px 6px; padding-top: 14px; border-top: 1px solid var(--stroke); display: flex; flex-direction: column; gap: 3px; font-size: 11px; }
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
  .split-view { flex: 1; min-height: 0; display: grid; grid-template-columns: minmax(0, 1fr); }
  .split-view.details-open { grid-template-columns: minmax(420px, 1fr) 360px; }
  .device-list { min-width: 0; overflow-y: auto; }
  .list-header, .device-row { display: grid; grid-template-columns: minmax(260px, 1.5fr) minmax(100px, .55fr) minmax(120px, .7fr); align-items: center; column-gap: 14px; }
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
