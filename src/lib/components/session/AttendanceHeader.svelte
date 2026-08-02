<script lang="ts">
  import { Download, RefreshCw } from "@lucide/svelte";
  import ButtonSpinner from "$lib/components/shared/ButtonSpinner.svelte";

  let {
    participantCount = 0,
    sessionCode = "",
    isExporting = false,
    isRefreshing = false,
    isLoading = false,
    onExportCsv,
    onRefresh
  }: {
    participantCount?: number;
    sessionCode?: string;
    isExporting?: boolean;
    isRefreshing?: boolean;
    isLoading?: boolean;
    onExportCsv: () => void;
    onRefresh: () => void;
  } = $props();
</script>

<div class="panel-header">
  <div class="header-titles">
    <p class="eyebrow">SESSION ATTENDANCE MANAGEMENT</p>
    <h2 class="panel-title">
      {participantCount} Participant{participantCount === 1 ? "" : "s"} Registered
    </h2>
  </div>

  <div class="header-actions">
    {#if sessionCode}
      <button type="button" class="outline action-btn" onclick={onExportCsv} disabled={isExporting || participantCount === 0}>
        {#if isExporting}<ButtonSpinner label="Exporting..." /> Exporting...{:else}<Download size={13} /> Export CSV{/if}
      </button>
    {/if}
    <button type="button" class="outline action-btn" onclick={onRefresh} disabled={isRefreshing || isLoading}>
      <RefreshCw size={13} class={isRefreshing ? "spin-icon" : ""} />
      {#if isRefreshing}Refreshing...{:else}Refresh Attendance{/if}
    </button>
  </div>
</div>

<style>
  .panel-header { display: flex; justify-content: space-between; align-items: flex-start; gap: var(--spacing-14); flex-wrap: wrap; margin-bottom: 8px; }
  .header-titles { display: flex; flex-direction: column; gap: 4px; }
  .eyebrow { font-size: 10px; letter-spacing: 0.1em; color: var(--color-warm-cream-dim); margin: 0; font-weight: 700; text-transform: uppercase; }
  .panel-title { font-size: 24px; font-weight: 500; color: var(--color-warm-cream); margin: 4px 0 0 0; font-family: var(--font-display); line-height: 1.2; }
  .header-actions { display: flex; align-items: center; gap: 8px; }
  .action-btn { display: inline-flex; align-items: center; gap: 6px; font-size: 11px; padding: 7px 14px; text-transform: uppercase; }
  :global(.spin-icon) { animation: spin 1s linear infinite; }
  @keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
  @media (max-width: 640px) { .panel-header { flex-direction: column; align-items: stretch; } .header-actions { width: 100%; justify-content: flex-start; } }
</style>
