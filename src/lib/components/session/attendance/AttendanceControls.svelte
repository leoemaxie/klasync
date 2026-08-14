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
      aria-label="Search attendance feed"
    />
    {#if searchQuery}
      <button
        type="button"
        class="clear-btn"
        onclick={() => (searchQuery = '')}
        aria-label="Clear search"
      >
        <X size={13} />
      </button>
    {/if}
  </div>

  <div class="filter-actions-group">
    <div
      class="segmented-filters"
      role="group"
      aria-label="Filter attendance status"
    >
      <button
        type="button"
        class={statusFilter === 'all' ? 'active' : ''}
        onclick={() => (statusFilter = 'all')}>All</button
      >
      <button
        type="button"
        class={statusFilter === 'verified' ? 'active' : ''}
        onclick={() => (statusFilter = 'verified')}>Verified</button
      >
      <button
        type="button"
        class={statusFilter === 'provisional' ? 'active' : ''}
        onclick={() => (statusFilter = 'provisional')}>Provisional</button
      >
    </div>

    <select
      bind:value={sortBy}
      class="sort-select"
      aria-label="Sort attendance list"
    >
      <option value="joined">Sort: Recent Joined</option>
      <option value="name">Sort: Name (A-Z)</option>
      <option value="heartbeats">Sort: Active Heartbeats</option>
    </select>

    <button
      type="button"
      class="control-btn icon-btn"
      onclick={onRefresh}
      disabled={isRefreshing}
      aria-label="Refresh attendance feed"
      title="Refresh attendance feed"
    >
      <RefreshCw size={13} class={isRefreshing ? 'spinning' : ''} />
    </button>

    <button
      type="button"
      class="control-btn export-btn"
      onclick={onExportCsv}
      disabled={isExporting}
      title="Export attendance records as CSV"
    >
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
    align-items: center;
    justify-content: space-between;
    gap: var(--spacing-10, 10px);
    flex-wrap: wrap;
    width: 100%;
  }

  .search-input-box {
    position: relative;
    display: flex;
    align-items: center;
    flex: 1;
    min-width: 240px;
    height: 38px;
  }

  :global(.search-input-box .search-ico) {
    position: absolute;
    left: 12px;
    color: var(--color-driftwood, #b8a794);
    pointer-events: none;
  }

  .attendance-search {
    height: 38px;
    width: 100%;
    box-sizing: border-box;
    margin: 0 !important;
    padding: 0 32px 0 34px !important;
    font-family: var(--font-body, sans-serif);
    font-size: 13px;
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border, #40372e) !important;
    border-radius: 4px !important;
    color: var(--color-warm-cream, #ffedd7);
    line-height: 36px;
    transition: border-color 0.15s ease, background 0.15s ease;
  }

  .attendance-search:focus {
    border-color: var(--color-warm-cream, #ffedd7) !important;
    background: rgba(24, 14, 8, 0.85);
    outline: none;
    box-shadow: none !important;
  }

  .clear-btn {
    position: absolute;
    right: 8px;
    height: 24px;
    width: 24px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--color-driftwood, #b8a794);
    cursor: pointer;
    border-radius: 2px;
    transition: color 0.15s ease;
  }

  .clear-btn:hover {
    color: var(--color-warm-cream, #ffedd7);
  }

  .filter-actions-group {
    display: flex;
    align-items: center;
    gap: var(--spacing-8, 8px);
    flex-wrap: wrap;
  }

  .segmented-filters {
    height: 38px;
    box-sizing: border-box;
    display: inline-flex;
    align-items: stretch;
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border, #40372e);
    border-radius: 4px;
    padding: 3px;
    gap: 3px;
  }

  .segmented-filters button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    box-sizing: border-box;
    background: transparent;
    border: none;
    color: var(--color-driftwood, #b8a794);
    font-size: 11px;
    font-family: var(--font-body, sans-serif);
    padding: 0 12px;
    border-radius: 2px;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-weight: 600;
    transition: all 0.15s ease;
    white-space: nowrap;
  }

  .segmented-filters button:hover:not(.active) {
    color: var(--color-warm-cream, #ffedd7);
    background: rgba(255, 237, 215, 0.06);
  }

  .segmented-filters button.active {
    background: var(--color-ember-accent, #dc5000);
    color: var(--color-warm-cream, #ffedd7);
    font-weight: 700;
  }

  .sort-select {
    height: 38px;
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border, #40372e);
    color: var(--color-warm-cream, #ffedd7);
    padding: 0 28px 0 12px;
    font-size: 11px;
    font-family: var(--font-body, sans-serif);
    font-weight: 500;
    letter-spacing: 0.04em;
    border-radius: 4px;
    outline: none;
    cursor: pointer;
    appearance: none;
    -webkit-appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='6' viewBox='0 0 10 6' fill='none'%3E%3Cpath d='M1 1L5 5L9 1' stroke='%23b8a794' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 10px center;
    transition: border-color 0.15s ease, background 0.15s ease;
  }

  .sort-select:hover,
  .sort-select:focus {
    border-color: var(--color-warm-cream, #ffedd7);
    background-color: rgba(24, 14, 8, 0.85);
  }

  .control-btn {
    height: 38px;
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--color-cork-border, #40372e);
    border-radius: 4px;
    cursor: pointer;
    font-family: var(--font-body, sans-serif);
    font-size: 11px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    font-weight: 600;
    transition: all 0.15s ease;
    white-space: nowrap;
    margin: 0;
  }

  .icon-btn {
    width: 38px;
    padding: 0;
    background: rgba(16, 9, 4, 0.6);
    color: var(--color-driftwood, #b8a794);
  }

  .icon-btn:hover:not(:disabled) {
    border-color: var(--color-warm-cream, #ffedd7);
    color: var(--color-warm-cream, #ffedd7);
    background: rgba(255, 237, 215, 0.06);
  }

  .icon-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .export-btn {
    gap: 6px;
    padding: 0 14px;
    background: rgba(56, 36, 22, 0.85);
    color: var(--color-warm-cream, #ffedd7);
  }

  .export-btn:hover:not(:disabled) {
    background: var(--color-bark-glow, #4a3020);
    border-color: var(--color-warm-cream-dim, rgba(255, 237, 215, 0.75));
    color: #ffffff;
  }

  .export-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  :global(.spinning) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  @media (max-width: 768px) {
    .attendance-controls-bar {
      flex-direction: column;
      align-items: stretch;
    }
    .search-input-box {
      width: 100%;
    }
    .filter-actions-group {
      justify-content: flex-start;
    }
  }
</style>
