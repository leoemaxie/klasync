<script lang="ts">
  import type { ClaimRecord } from '$lib/api';
  import { searchLectures, type SearchHit } from '$lib/api/aiStudy';
  import { onDestroy } from 'svelte';
  import { Archive, GraduationCap, Search, X } from '@lucide/svelte';
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

  // Meaning-based matches inside lectures (debounced), shown under the filter.
  let hits = $state<SearchHit[]>([]);
  let searching = $state(false);
  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const query = searchQuery.trim();
    clearTimeout(searchTimer);
    if (query.length < 3) {
      hits = [];
      return;
    }
    searchTimer = setTimeout(async () => {
      searching = true;
      try {
        hits = await searchLectures(query);
      } catch {
        hits = [];
      } finally {
        searching = false;
      }
    }, 600);
  });
  onDestroy(() => clearTimeout(searchTimer));

  const clock = (seconds: number) =>
    `${String(Math.floor(seconds / 60)).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}`;
  function openHit(hit: SearchHit) {
    const claim = claims.find((c) => c.id === hit.session_id);
    if (claim) onSelectClaim(claim);
  }

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
        <Archive size={13} /><span>Lectures ({claims.length})</span>
      </button>
      <button
        type="button"
        role="tab"
        class="mode-btn"
        class:active={activeViewMode === 'courses'}
        onclick={() => (activeViewMode = 'courses')}
      >
        <GraduationCap size={13} /><span>Courses</span>
      </button>
    </div>

    <div class="search-input-wrap">
      <Search size={14} class="search-ico" />
      <input
        type="search"
        bind:value={searchQuery}
        placeholder="Search lectures..."
        class="sidebar-search-input"
        aria-label="Search archived lectures"
      />
      {#if searchQuery}
        <button
          type="button"
          class="clear-query-btn"
          onclick={() => (searchQuery = '')}
          aria-label="Clear filter"
        >
          <X size={12} />
        </button>
      {/if}
    </div>
  </div>

  <div class="sidebar-list-container">
    {#if searchQuery.trim().length >= 3 && (searching || hits.length)}
      <section class="semantic-hits" aria-label="Matches inside lectures">
        <p class="semantic-title">
          {searching ? 'Searching inside lectures…' : 'Found inside lectures'}
        </p>
        {#each hits.slice(0, 5) as hit}
          <button
            type="button"
            class="semantic-hit"
            onclick={() => openHit(hit)}
          >
            <span class="hit-meta"
              >{hit.course_code} · {hit.session_title} · {clock(
                hit.start_sec
              )}</span
            >
            <span class="hit-text">{hit.text.slice(0, 140)}…</span>
          </button>
        {/each}
      </section>
    {/if}
    {#if isLoading}
      <SkeletonCard lines={2} label="Loading lectures..." />
    {:else if activeViewMode === 'claims'}
      <div class="claims-list-track">
        {#if filteredClaims.length === 0}
          <div class="empty-claims">
            <p class="empty-title">No lectures found</p>
            <p class="empty-sub">
              {#if searchQuery}
                Try adjusting your search terms.
              {:else}
                Browse courses to view lectures.
              {/if}
            </p>
          </div>
        {:else}
          <div class="claims-stack" role="list">
            {#each filteredClaims as claim (claim.id)}
              <ArchiveClaimItem
                {claim}
                isSelected={selectedClaim?.id === claim.id}
                onSelect={onSelectClaim}
              />
            {/each}
          </div>
        {/if}
      </div>
    {:else}
      <div class="course-directory-shortcut">
        <p class="hint">Browse enrolled courses to open materials.</p>
      </div>
    {/if}
  </div>

  <div class="sidebar-footer">
    <div class="sync-status-row">
      <span class="sync-dot"></span><span class="sync-label">Synced</span>
    </div>
  </div>
</aside>

<style>
  .semantic-hits {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: var(--spacing-10);
  }
  .semantic-title {
    margin: 0;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--color-warm-cream-dim);
  }
  .semantic-hit {
    display: flex;
    flex-direction: column;
    gap: 2px;
    text-align: left;
    padding: 8px 10px;
    background: rgba(220, 80, 0, 0.07);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-controls, 6px);
    cursor: pointer;
    color: var(--color-warm-cream);
  }
  .hit-meta {
    font-size: 10px;
    color: var(--color-ember-accent);
    font-weight: 600;
  }
  .hit-text {
    font-size: 12px;
    line-height: 1.4;
    color: var(--color-warm-cream-dim);
  }
  .studio-sidebar {
    display: flex;
    flex-direction: column;
    background: rgba(16, 9, 4, 0.5);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards);
    padding: var(--spacing-12);
    gap: var(--spacing-10);
    max-height: calc(100vh - var(--nav-height) - 160px);
    position: sticky;
    top: calc(var(--nav-height) + 20px);
  }
  .sidebar-header {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-8);
  }
  .view-mode-tabs {
    display: flex;
    gap: 4px;
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    padding: 3px;
    border-radius: var(--radius-controls, 4px);
  }
  .mode-btn {
    flex: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 7px 8px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    background: transparent;
    border: none;
    color: var(--color-warm-cream-dim);
    border-radius: 3px;
    cursor: pointer;
    transition:
      color 0.15s ease,
      background 0.15s ease;
  }
  .mode-btn:hover {
    color: var(--color-warm-cream);
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
  :global(.search-input-wrap .search-ico) {
    position: absolute;
    left: 10px;
    color: var(--color-warm-cream-dim);
    pointer-events: none;
  }
  .sidebar-search-input {
    width: 100%;
    padding-left: 32px !important;
    font-size: 12px;
    height: 36px;
    min-height: 36px;
    margin: 0;
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    color: var(--color-warm-cream);
    border-radius: var(--radius-controls, 4px);
    box-sizing: border-box;
  }
  .sidebar-search-input::placeholder {
    color: rgba(255, 237, 215, 0.45);
  }
  .sidebar-search-input:focus {
    border-color: var(--color-warm-cream);
    outline: none;
  }
  .clear-query-btn {
    position: absolute;
    right: 8px;
    font-size: 10px;
    text-transform: uppercase;
    color: var(--color-ember-accent);
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 4px;
    display: inline-flex;
    align-items: center;
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
    padding: var(--spacing-12);
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
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--color-warm-cream-dim);
    text-transform: uppercase;
  }
  @media (max-width: 960px) {
    .studio-sidebar {
      position: static;
      max-height: none;
      padding: 10px;
    }
  }
</style>
