<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import type { Tweak, TweakCategory } from "$lib/types";
    import { activeCategory } from "$lib/stores";
    import Badge from "./ui/Badge.svelte";

    export let tweaks: Tweak[] = [];
    export let showHeader = true;

    // Track which tweaks are currently being toggled
    let loadingIds: Set<string> = new Set();

    // Filter displayed tweaks based on active category
    let displayedTweaks: Tweak[] = [];
    let currentCategory: TweakCategory | null = null;
    let categoryTitle = "";

    // Subscribe to store
    activeCategory.subscribe((cat) => {
        currentCategory = cat;
        if (cat) {
            displayedTweaks = tweaks.filter((t) => t.category === cat);
            const rawTitle =
                displayedTweaks.length > 0 ? getCategoryName(cat) : "Tweaks";
            // Strip emoji
            categoryTitle = rawTitle.replace(/^[^\w\s]+/, "").trim();
        } else {
            displayedTweaks = [];
            categoryTitle = "";
        }
    });

    $: if (showHeader && currentCategory) {
        // Reactive update if list changes but category stays same
        displayedTweaks = tweaks.filter((t) => t.category === currentCategory);
    } else if (!showHeader) {
        // If header is hidden, we assume the parent is controlling the list filter (like TcpSection)
        // So we just show all passed 'tweaks'
        displayedTweaks = tweaks;
    }

    function getCategoryName(cat: TweakCategory): string {
        switch (cat) {
            case "Network":
                return "🌐 Network Optimization";
            case "CpuPerformance":
                return "🚀 CPU & Performance";
            case "Privacy":
                return "🛡️ Privacy & Telemetry";
            case "DebloatTelemetry":
                return "🧹 Debloat & Apps";
            case "StartupServices":
                return "⚡ Startup & Services";
            case "System":
                return "🎨 System & Visuals";
            case "GameOptimizations":
                return "🎮 Gaming Optimization";
            case "MouseInput":
                return "🖱️ Mouse & Input";
            case "DisplayMonitor":
                return "🖥️ Display & Monitor";
            case "FileSystem":
                return "💾 Storage & Filesystem";
            case "BackupRestore":
                return "↺ Backup & Restore";
            case "Monitoring":
                return "📊 System Monitoring";
            case "SecurityPrivacy":
                return "🔒 Security & Privacy";
            case "Activation":
                return "🔑 Windows Activation";
            default:
                return "Tweaks";
        }
    }

    $: getBadgeLevel = (level: string) => level?.toLowerCase() as "safe" | "careful" | "dangerous" | "default";

    import { onDestroy, onMount } from "svelte";
    import TerminalModal from "./TerminalModal.svelte";
    import { listen } from "@tauri-apps/api/event";

    // Modal state
    let showModal = false;
    let modalTitle = "";
    let modalLogs: string[] = [];
    let processingTweakId: string | null = null;
    let unlistenOutput: (() => void) | null = null;
    let failedIds = new Set<string>();

    onMount(async () => {
        // Listen for streaming output
        unlistenOutput = await listen<any>("tweak-output", (event) => {
            const { id, type, line } = event.payload;
            // Only show logs if we are processing this tweak
            if (id === processingTweakId) {
                // If modal is not open, open it (for cases where we didn't explicitly open it yet)
                if (!showModal) {
                    showModal = true;
                    const tweak = tweaks.find((t) => t.id === id);
                    modalTitle = tweak
                        ? `Executing: ${tweak.name}`
                        : "Execution Output";
                }
                modalLogs = [...modalLogs, line];
            }
        });
    });

    onDestroy(() => {
        if (unlistenOutput) unlistenOutput();
    });

    async function toggleTweak(tweak: Tweak) {
        if (loadingIds.has(tweak.id)) return; // Already loading
        let dangerousAcknowledgement: string | null = null;
        if (!tweak.enabled && tweak.warning_level === "Dangerous") {
            const expected = `APPLY ${tweak.id}`;
            dangerousAcknowledgement = prompt(
                `POWER USER CONTROL\n\n${tweak.name}\n\n${tweak.description}\n\n` +
                `Only continue for a specific reason and with a recovery plan. Type ${expected} to apply.`
            );
            if (dangerousAcknowledgement !== expected) return;
        }

        // Add to loading set
        loadingIds.add(tweak.id);
        failedIds.delete(tweak.id);
        failedIds = failedIds;
        loadingIds = loadingIds; // Trigger reactivity

        // Setup modal for Action types or Activation category
        if (tweak.tweak_type === "Action" || tweak.category === "Activation") {
            showModal = true;
            modalTitle = `Executing: ${tweak.name}`;
            modalLogs = [];
            processingTweakId = tweak.id;
        }

        try {
            if (tweak.enabled && tweak.tweak_type !== "Action") {
                tweak.enabled = await invoke<boolean | null>("undo_tweak", { id: tweak.id });
            } else {
                tweak.enabled = await invoke<boolean | null>("apply_tweak", { id: tweak.id, dangerousAcknowledgement });
            }
            tweaks = tweaks; // Trigger reactivity
        } catch (e) {
            failedIds.add(tweak.id);
            failedIds = failedIds;
            tweak.enabled = await invoke<boolean | null>("get_tweak_state", { id: tweak.id }).catch(() => null);
            tweaks = tweaks;
            console.error("Failed to toggle tweak:", e);
            showModal = true;
            modalTitle = `Failed: ${tweak.name}`;
            modalLogs = [...modalLogs, `Error: ${e}`];
        } finally {
            // Remove from loading set
            loadingIds.delete(tweak.id);
            loadingIds = loadingIds; // Trigger reactivity
            processingTweakId = null; // Done processing
        }
    }

    async function closeModal() {
        if (processingTweakId) {
            // If still processing, kill the process
            try {
                await invoke("kill_tweak_process", { id: processingTweakId });
            } catch (e) {
                console.error("Failed to kill process:", e);
            }
        }
        showModal = false;
        processingTweakId = null;
    }
</script>

<TerminalModal
    bind:show={showModal}
    title={modalTitle}
    logs={modalLogs}
    processing={loadingIds.has(processingTweakId || "")}
    on:close={closeModal}
/>

<div class="list-container">
    {#if showHeader && currentCategory}
        <div class="header">
            <h1>{categoryTitle}</h1>
            <p class="subtitle">
                {displayedTweaks.length} optimizations available
            </p>
        </div>
    {/if}

    <div class="scroll-area">
        {#each displayedTweaks as tweak (tweak.id)}
            <div class="tweak-item">
                <div class="info">
                    <div class="top-row">
                        <span class="name">{tweak.name}</span>
                        <Badge level={getBadgeLevel(tweak.warning_level)} />
                    </div>
                    <p class="description">{tweak.description}</p>
                </div>

                <button
                    class="toggle-btn"
                    class:on={tweak.enabled === true && !failedIds.has(tweak.id)}
                    class:failed={failedIds.has(tweak.id)}
                    class:loading={loadingIds.has(tweak.id)}
                    disabled={loadingIds.has(tweak.id)}
                    on:click={() => toggleTweak(tweak)}
                >
                    {#if loadingIds.has(tweak.id)}
                        <span class="btn-spinner"></span>
                        {tweak.tweak_type === "Action" ? "Running..." : tweak.enabled ? "Reverting..." : "Applying..."}
                    {:else}
                        {failedIds.has(tweak.id) ? "Failed · Retry" : tweak.tweak_type === "Action" ? (tweak.check && tweak.enabled ? "Enabled · Run" : "Run") : tweak.enabled == null ? "Check / Apply" : tweak.enabled ? "Enabled" : "Disabled"}
                    {/if}
                </button>
            </div>
        {/each}

        {#if displayedTweaks.length === 0}
            <div class="empty-state">
                <p>No tweaks available in this category yet.</p>
            </div>
        {/if}
    </div>
</div>

<style>
    .toggle-btn.failed {
        background: var(--toggle-off-bg);
        color: #f87171;
        border-color: #f87171;
        box-shadow: none;
    }
    .list-container {
        display: flex;
        flex-direction: column;
        height: 100%;
        overflow: hidden;
        padding: 0;
    }

    .header {
        margin-bottom: 24px;
        flex-shrink: 0;
    }

    h1 {
        font-size: 24px;
        font-weight: 600;
        margin-bottom: 8px;
    }

    .subtitle {
        margin: 0;
        color: var(--text-muted);
        font-size: 13px;
    }

    .scroll-area {
        flex: 1;
        overflow-y: auto;
        padding-right: 8px;
        padding-top: 4px; /* Prevent hover clipping at top */
        padding-bottom: 32px;
    }

    .tweak-item {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 16px 20px;
        background: rgba(255, 255, 255, 0.02);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-card);
        margin-bottom: 12px;
        transition: all 0.2s;
    }

    .tweak-item:hover {
        background: rgba(255, 255, 255, 0.04);
        border-color: var(--accent-color);
        transform: translateY(-1px);
    }

    .info {
        flex: 1;
        min-width: 0;
        overflow: hidden;
    }

    .top-row {
        display: flex;
        align-items: center;
        gap: 12px;
        margin-bottom: 6px;
    }

    .name {
        font-size: 15px;
        font-weight: 600;
        color: var(--text-color);
    }

    .description {
        font-size: 13px;
        color: var(--text-muted);
        line-height: 1.4;
        margin: 0;
        display: -webkit-box;
        -webkit-line-clamp: 2;
        line-clamp: 2;
        -webkit-box-orient: vertical;
        overflow: hidden;
    }

    /* Toggle Button */
    .toggle-btn {
        min-width: 90px;
        padding: 7px 16px;
        border-radius: var(--radius-sm);
        border: 1px solid var(--toggle-off-border);
        background: var(--toggle-off-bg);
        color: var(--toggle-off-color);
        cursor: pointer;
        font-size: 13px;
        font-weight: 600;
        transition: all 0.18s ease;
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 6px;
    }
    .toggle-btn:hover {
        border-color: rgba(255, 255, 255, 0.18);
        color: var(--text-color);
        background: rgba(255, 255, 255, 0.07);
    }
    .toggle-btn.on {
        background: var(--toggle-on-bg);
        border-color: var(--toggle-on-border);
        color: var(--toggle-on-color);
        box-shadow: var(--toggle-on-glow);
    }
    .toggle-btn.on:hover {
        background: rgba(129, 140, 248, 0.26);
        box-shadow: var(--toggle-on-glow);
    }
    .toggle-btn.loading { opacity: 0.7; cursor: wait; pointer-events: none; }
    .toggle-btn:disabled { cursor: not-allowed; }

    .empty-state {
        padding: 40px;
        text-align: center;
        color: var(--text-muted);
        font-style: italic;
    }

    .btn-spinner {
        display: inline-block;
        width: 12px;
        height: 12px;
        border: 2px solid currentColor;
        border-top-color: transparent;
        border-radius: 50%;
        animation: btn-spin 0.8s linear infinite;
        margin-right: 6px;
        vertical-align: middle;
    }

    @keyframes btn-spin {
        to {
            transform: rotate(360deg);
        }
    }

    @media (max-width: 940px) {
        .list-container { padding: 0; }
        .tweak-item { padding: 14px 15px; gap: 12px; }
        .description { -webkit-line-clamp: 3; line-clamp: 3; }
    }
</style>
