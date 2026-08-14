<script lang="ts">
  import type { ClaimRecord } from '$lib/api';
  import { Archive, GraduationCap, Search } from '@lucide/svelte';
  import SkeletonCard from '$lib/components/shared/SkeletonCard.svelte';
  import ArchiveClaimItem from './ArchiveClaimItem.svelte';

  let {
    activeViewMode = $bindable<'courses' | 'claims'>('claims'),
    searchQuery = $bindable(''),
    claims = [],
    selectedClaim = null,
    isLoading = false,
    onSelectClaim,
  }: {
    activeViewMode: 'courses' | 'claims';
    searchQuery: string;
    claims: ClaimRecord[];
    selectedClaim: ClaimRecord | null;
    isLoading: boolean;
    onSelectClaim: (claim: ClaimRecord) => void;
  } = $props();

  const filteredClaims = $derived(
    claims.filter(
      (c) =>
        c.course_code.toLowerCase().includes(searchQuery.toLowerCase()) ||
        c.session_title.toLowerCase().includes(searchQuery.toLowerCase())
    )
  );
</script>

<aside class="studio-sidebar panel">
  <div class="sidebar-header">
    <div class="view-mode-tabs" role="tablist" aria-label="Archive mode">
      <button
        type="button"
        role="tab"
        class="mode-btn"
        class:active={activeViewMode === 'claims'}
        onclick={() => (activeViewMode = 'claims')}
      >
        <Archive size={13} />
        <span>Lectures ({claims.length})</span>
      </button>
      <button
        type="button"
        role="tab"
        class="mode-btn"
        class:active={activeViewMode === 'courses'}
        onclick={() => (activeViewMode = 'courses')}
      >
        <GraduationCap size={13} />
        <span>Courses</span>
      </button>
    </div>

    <div class="search-input-wrap">
      <Search size={13} class="search-icon" />
      <input
        type="search"
        bind:value={searchQuery}
        placeholder="Filter code or title..."
        class="sidebar-search-input"
        aria-label="Filter lecture archives"
      />
      {#if searchQuery}
        <button
          type="button"
          class="clear-query-btn text"
          onclick={() => (searchQuery = '')}>Clear</button
        >
      {/if}
    </div>
  </div>

  <div class="sidebar-list-container">
    {#if isLoading}
      <SkeletonCard lines={2} label="Loading lectures..." />
    {:else if activeViewMode === 'claims'}
      {#if filteredClaims.length > 0}
        <div class="claims-stack" role="list">
          {#each filteredClaims as claim (claim.id)}
            <ArchiveClaimItem
              {claim}
              isSelected={selectedClaim?.id === claim.id}
              onSelect={onSelectClaim}
            />
          {/each}
        </div>
      {:else}
        <div class="empty-sidebar-box">
          <p class="hint">No lectures match query</p>
        </div>
      {/if}
    {:else}
      <div class="course-directory-shortcut">
        <p class="hint">Browse enrolled courses to open materials.</p>
      </div>
    {/if}
  </div>

  <div class="sidebar-footer">
    <div class="sync-status-row">
      <span class="sync-dot"></span><span class="sync-label">Sync Active</span>
    </div>
  </div>
</aside>

<style>
  .studio-sidebar {
    display: flex;
    flex-direction: column;
    background: rgba(16, 9, 4, 0.5);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards);
    padding: var(--spacing-14);
    gap: var(--spacing-12);
    max-height: calc(100vh - var(--nav-height) - 160px);
    position: sticky;
    top: calc(var(--nav-height) + 20px);
  }
  .sidebar-header {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-10);
  }
  .view-mode-tabs {
    display: flex;
    gap: 4px;
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    padding: 3px;
    border-radius: 4px;
  }
  .mode-btn {
    flex: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 6px;
    font-size: 11px;
    text-transform: uppercase;
    background: transparent;
    border: none;
    color: var(--color-driftwood);
    border-radius: 3px;
  }
  .mode-btn.active {
    background: var(--color-bark-brown);
    color: var(--color-warm-cream);
  }
  .search-input-wrap {
    position: relative;
    display: flex;
    align-items: center;
  }
  :global(.search-input-wrap .search-icon) {
    position: absolute;
    left: 10px;
    color: var(--color-driftwood);
  }
  .sidebar-search-input {
    width: 100%;
    padding-left: 30px !important;
    font-size: 12px;
    margin: 0;
  }
  .clear-query-btn {
    position: absolute;
    right: 8px;
    font-size: 10px;
    text-transform: uppercase;
    color: var(--color-ember-accent);
  }
  .sidebar-list-container {
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--spacing-8);
  }
  .claims-stack {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-8);
  }
  .empty-sidebar-box,
  .course-directory-shortcut {
    padding: var(--spacing-14);
    text-align: center;
  }
  .sidebar-footer {
    border-top: 1px solid var(--color-cork-border);
    padding-top: var(--spacing-8);
    margin-top: auto;
  }
  .sync-status-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .sync-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #4ade80;
  }
  .sync-label {
    font-size: 10px;
    color: var(--color-driftwood);
    text-transform: uppercase;
  }
  @media (max-width: 960px) {
    .studio-sidebar {
      position: static;
      max-height: none;
    }
  }
</style>
