<script lang="ts">
    import { Check } from "lucide-svelte";
    import Button from "./Button.svelte";

    export let icon: any = null;
    export let title: string;
    export let description: string = "";
    export let actionLabel: string = "";
    export let onAction: (() => void) | null = null;
</script>

<div class="section-header">
    <div class="header-text">
        <h2>
            {#if icon}
                <span class="icon"><svelte:component this={icon} size={18} /></span>
            {/if}
            {title}
        </h2>
        {#if description}<p>{description}</p>{/if}
    </div>
    <div class="actions">
        {#if actionLabel && onAction}
            <Button variant="safe" on:click={onAction}>
                <Check size={13} /> {actionLabel}
            </Button>
        {/if}
        <slot />
    </div>
</div>

<style>
    .section-header {
        margin-bottom: 20px;
        padding-bottom: 16px;
        border-bottom: 1px solid var(--border-color);
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        gap: 16px;
        flex-shrink: 0;
    }
    .header-text h2 {
        font-size: 18px;
        font-weight: 600;
        margin: 0 0 6px 0;
        color: var(--text-color);
        display: flex;
        align-items: center;
        gap: 10px;
    }
    .icon { color: var(--accent-color); display: flex; align-items: center; }
    .header-text p { margin: 0; color: var(--text-muted); font-size: 13px; }
    .actions { flex-shrink: 0; display: flex; align-items: center; gap: 8px; }
</style>
