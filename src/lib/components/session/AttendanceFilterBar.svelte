<script lang="ts">
  import { Search, X } from '@lucide/svelte';

  let {
    searchQuery = $bindable(''),
    statusFilter = $bindable('all'),
    sortBy = $bindable('joined'),
    total = 0,
    verified = 0,
    provisional = 0,
  }: {
    searchQuery: string;
    statusFilter: 'all' | 'verified' | 'provisional';
    sortBy: 'joined' | 'name' | 'heartbeats';
    total: number;
    verified: number;
    provisional: number;
  } = $props();
</script>

<div class="controls-bar">
  <div class="search-wrap">
    <Search size={14} aria-hidden="true" class="search-icon" />
    <input
      type="text"
      placeholder="Search student name or matric number..."
      aria-label="Search student name or matric number"
      bind:value={searchQuery}
      class="search-input"
    />
    {#if searchQuery}
      <button
        type="button"
        class="clear-search-btn"
        onclick={() => (searchQuery = '')}
        aria-label="Clear search"
      >
        <X size={12} aria-hidden="true" />
      </button>
    {/if}
  </div>

  <div class="filter-group" role="group" aria-label="Status filter">
    <button
      type="button"
      class="pill-tab"
      class:active={statusFilter === 'all'}
      aria-pressed={statusFilter === 'all'}
      onclick={() => (statusFilter = 'all')}
    >
      All <span class="count-chip">{total}</span>
    </button>
    <button
      type="button"
      class="pill-tab"
      class:active={statusFilter === 'verified'}
      aria-pressed={statusFilter === 'verified'}
      onclick={() => (statusFilter = 'verified')}
    >
      Verified <span class="count-chip">{verified}</span>
    </button>
    <button
      type="button"
      class="pill-tab"
      class:active={statusFilter === 'provisional'}
      aria-pressed={statusFilter === 'provisional'}
      onclick={() => (statusFilter = 'provisional')}
    >
      Provisional <span class="count-chip">{provisional}</span>
    </button>
  </div>

  <div class="sort-group">
    <span class="sort-lbl">SORT:</span>
    <select
      bind:value={sortBy}
      class="sort-select"
      aria-label="Sort attendance list"
    >
      <option value="joined">Latest Joined</option>
      <option value="name">Name (A-Z)</option>
      <option value="heartbeats">Check-in Pulses</option>
    </select>
  </div>
</div>

<style>
  .controls-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: 8px;
    padding: 10px 14px;
    margin-bottom: 12px;
  }
  .search-wrap {
    position: relative;
    display: flex;
    align-items: center;
    flex: 1;
    min-width: 200px;
    max-width: 320px;
  }
  :global(.search-icon) {
    position: absolute;
    left: 10px;
    color: var(--color-warm-cream-dim);
    pointer-events: none;
  }
  .search-input {
    width: 100%;
    padding: 7px 28px 7px 32px;
    background: rgba(10, 5, 2, 0.8);
    border: 1px solid var(--color-cork-border);
    border-radius: 6px;
    color: var(--color-warm-cream);
    font-size: 12px;
  }
  .clear-search-btn {
    position: absolute;
    right: 8px;
    background: none;
    border: none;
    color: var(--color-driftwood);
    cursor: pointer;
  }
  .filter-group {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .pill-tab {
    background: rgba(10, 5, 2, 0.4);
    border: 1px solid var(--color-cork-border);
    border-radius: 20px;
    color: var(--color-warm-cream-dim);
    font-size: 11px;
    padding: 5px 12px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .pill-tab.active {
    background: var(--color-bark-brown);
    color: var(--color-warm-cream);
    border-color: var(--color-warm-cream-dim);
    font-weight: 600;
  }
  .count-chip {
    font-size: 11px;
    font-weight: 700;
    background: rgba(255, 255, 255, 0.1);
    padding: 1px 6px;
    border-radius: 9999px;
  }
  .sort-group {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .sort-lbl {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: var(--color-warm-cream-dim);
  }
  .sort-select {
    background: rgba(10, 5, 2, 0.8);
    border: 1px solid var(--color-cork-border);
    border-radius: 6px;
    color: var(--color-warm-cream);
    font-size: 11px;
    padding: 6px 10px;
    cursor: pointer;
  }
</style>
