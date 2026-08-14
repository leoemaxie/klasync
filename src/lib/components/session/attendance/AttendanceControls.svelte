<script lang="ts">
  import { Search, Download, RefreshCw, X } from '@lucide/svelte';
  import ButtonSpinner from '$lib/components/shared/ButtonSpinner.svelte';

  let {
    searchQuery = $bindable(''),
    statusFilter = $bindable('all'),
    sortBy = $bindable('joined'),
    isExporting = false,
    isRefreshing = false,
    onExportCsv,
    onRefresh,
  }: {
    searchQuery: string;
    statusFilter: 'all' | 'verified' | 'provisional';
    sortBy: 'joined' | 'name' | 'heartbeats';
    isExporting?: boolean;
    isRefreshing?: boolean;
    onExportCsv: () => Promise<void> | void;
    onRefresh: () => Promise<void> | void;
  } = $props();
</script>

<div class="attendance-controls-bar">
  <div class="search-input-box">
    <Search size={14} class="search-ico" />
    <input
      type="search"
      bind:value={searchQuery}
      placeholder="Search by student name or matric..."
      class="attendance-search"
    />
    {#if searchQuery}
      <button type="button" class="text clear-btn" onclick={() => (searchQuery = '')} aria-label="Clear search">
        <X size={13} />
      </button>
    {/if}
  </div>

  <div class="filter-actions-group">
    <div class="segmented-filters" role="group" aria-label="Filter attendance status">
      <button
        type="button"
        class={statusFilter === 'all' ? 'active' : ''}
        onclick={() => (statusFilter = 'all')}
      >All</button>
      <button
        type="button"
        class={statusFilter === 'verified' ? 'active' : ''}
        onclick={() => (statusFilter = 'verified')}
      >Verified</button>
      <button
        type="button"
        class={statusFilter === 'provisional' ? 'active' : ''}
        onclick={() => (statusFilter = 'provisional')}
      >Provisional</button>
    </div>

    <select bind:value={sortBy} class="sort-select" aria-label="Sort attendance list">
      <option value="joined">Sort: Recent Joined</option>
      <option value="name">Sort: Name (A-Z)</option>
      <option value="heartbeats">Sort: Active Heartbeats</option>
    </select>

    <button type="button" class="outline icon-btn" onclick={onRefresh} disabled={isRefreshing} aria-label="Refresh attendance feed">
      <RefreshCw size={13} class={isRefreshing ? 'spinning' : ''} />
    </button>

    <button type="button" class="primary-bark export-btn" onclick={onExportCsv} disabled={isExporting}>
      {#if isExporting}
        <ButtonSpinner />
      {:else}
        <Download size={13} />
      {/if}
      <span>Export CSV</span>
    </button>
  </div>
</div>

<style>
  .attendance-controls-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--spacing-12);
    flex-wrap: wrap;
  }
  .search-input-box {
    position: relative;
    display: flex;
    align-items: center;
    flex: 1;
    min-width: 240px;
  }
  :global(.search-input-box .search-ico) {
    position: absolute;
    left: 10px;
    color: var(--color-driftwood);
  }
  .attendance-search {
    width: 100%;
    padding-left: 32px !important;
    padding-right: 28px !important;
    font-size: 13px;
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: 4px;
    color: var(--color-warm-cream);
    margin: 0;
  }
  .clear-btn {
    position: absolute;
    right: 6px;
    padding: 4px;
    color: var(--color-driftwood);
  }
  .filter-actions-group {
    display: flex;
    align-items: center;
    gap: var(--spacing-8);
    flex-wrap: wrap;
  }
  .segmented-filters {
    display: flex;
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: 4px;
    padding: 2px;
  }
  .segmented-filters button {
    background: transparent;
    border: none;
    color: var(--color-driftwood);
    font-size: 11px;
    padding: 4px 10px;
    border-radius: 3px;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .segmented-filters button.active {
    background: var(--color-ember-accent);
    color: var(--color-warm-cream);
    font-weight: 600;
  }
  .sort-select {
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    color: var(--color-warm-cream);
    padding: 6px 10px;
    font-size: 11px;
    border-radius: 4px;
    outline: none;
  }
  .icon-btn {
    padding: 7px 10px;
  }
  .export-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    padding: 6px 14px;
  }
  :global(.spinning) {
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
</style>
