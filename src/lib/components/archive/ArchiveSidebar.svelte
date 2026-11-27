<script lang="ts">
  import type { ClaimRecord } from '$lib/api';
  import { searchLectures, type SearchHit } from '$lib/api/aiStudy';
  import { onDestroy } from 'svelte';
  import {
    Archive,
    GraduationCap,
    Search,
    X,
    Play,
    Inbox,
    ClipboardCheck,
  } from '@lucide/svelte';
  import SkeletonCard from '$lib/components/shared/SkeletonCard.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import EmptyState from '$lib/components/ui/EmptyState.svelte';
  import Segmented from '$lib/components/ui/Segmented.svelte';
  import { clock } from '$lib/studio/time';
  import ArchiveClaimItem from './ArchiveClaimItem.svelte';

  let {
    activeViewMode = $bindable<'courses' | 'claims' | 'assessments'>('claims'),
    searchQuery = $bindable(''),
    claims = [],
    selectedClaim = null,
    isLoading = false,
    resumeVersion = 0,
    onSelectClaim,
  }: {
    activeViewMode: 'courses' | 'claims' | 'assessments';
    searchQuery: string;
    claims: ClaimRecord[];
    selectedClaim: ClaimRecord | null;
    isLoading: boolean;
    resumeVersion?: number;
    /** `startAt` is set when opening from a "found inside lectures" hit. */
    onSelectClaim: (claim: ClaimRecord, startAt?: number | null) => void;
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

  /** Opens the lecture at the moment the match was said. */
  function openHit(hit: SearchHit) {
    const claim = claims.find((c) => c.id === hit.session_id);
    if (claim) onSelectClaim(claim, hit.start_sec);
  }

  const q = $derived(searchQuery.trim().toLowerCase());
  const filteredClaims = $derived(
    claims.filter(
      (c) =>
        !q ||
        c.course_code.toLowerCase().includes(q) ||
        c.session_title.toLowerCase().includes(q)
    )
  );
</script>

<aside class="sidebar" aria-label="Lecture archive">
  <div class="head">
    <Segmented
      label="Archive view"
      bind:value={activeViewMode}
      options={[
        { value: 'claims', label: `Lectures ${claims.length}`, icon: Archive },
        { value: 'courses', label: 'Courses', icon: GraduationCap },
        { value: 'assessments', label: 'Assessments', icon: ClipboardCheck },
      ]}
    />

    {#if activeViewMode === 'claims'}
      <div class="search">
        <Search size={15} />
        <input
          type="search"
          bind:value={searchQuery}
          placeholder="Search lectures and what was said"
          aria-label="Search archived lectures"
          autocomplete="off"
        />
        {#if searchQuery}
          <Button
            icon
            size="sm"
            class="clear"
            aria-label="Clear search"
            onclick={() => (searchQuery = '')}
          >
            <X size={14} />
          </Button>
        {/if}
      </div>
    {/if}
  </div>

  <div class="scroll">
    {#if activeViewMode === 'claims'}
      {#if searchQuery.trim().length >= 3 && (searching || hits.length)}
        <section class="hits" aria-label="Matches inside lectures">
          <h2 class="section">
            {searching ? 'Searching inside lectures…' : 'Found inside lectures'}
          </h2>
          {#each hits.slice(0, 5) as hit}
            <button type="button" class="hit" onclick={() => openHit(hit)}>
              <span class="hit-meta">
                <span class="hit-course">{hit.course_code}</span>
                <span class="hit-title">{hit.session_title}</span>
                <span class="hit-time">
                  <Play size={10} fill="currentColor" />
                  {clock(hit.start_sec)}
                </span>
              </span>
              <span class="hit-text">
                {hit.text.slice(0, 140)}{hit.text.length > 140 ? '…' : ''}
              </span>
            </button>
          {/each}
        </section>
      {/if}

      {#if isLoading}
        <SkeletonCard lines={2} label="Loading lectures…" />
      {:else if filteredClaims.length === 0}
        <EmptyState
          icon={Inbox}
          title="No lectures found"
          description={searchQuery
            ? 'Try different words, or clear the search.'
            : 'Lectures you attend appear here.'}
        />
      {:else}
        <ul class="list">
          {#each filteredClaims as claim (claim.id)}
            <li>
              <ArchiveClaimItem
                {claim}
                {resumeVersion}
                isSelected={selectedClaim?.id === claim.id}
                onSelect={(c) => onSelectClaim(c)}
              />
            </li>
          {/each}
        </ul>
      {/if}
    {:else}
      <p class="note">Browse your enrolled courses to open their lectures.</p>
    {/if}
  </div>
</aside>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-width: 0;
    max-height: calc(100dvh - 120px);
    padding: 14px;
    background: var(--surface-1);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-xl);
    box-shadow: var(--elev-2);
  }
  .head {
    display: flex;
    flex-direction: column;
    gap: 10px;
    flex: none;
  }
  .head :global(.ui-seg) {
    align-self: stretch;
  }
  .head :global(.ui-seg button) {
    flex: 1;
    justify-content: center;
  }
  .search {
    position: relative;
    display: flex;
    align-items: center;
  }
  .search :global(svg:first-child) {
    position: absolute;
    left: 13px;
    color: var(--text-tertiary);
    pointer-events: none;
  }
  .search input {
    width: 100%;
    height: var(--control-h);
    min-height: var(--control-h);
    margin: 0;
    padding: 0 36px 0 38px;
    border: 1px solid var(--border-default);
    border-radius: 999px;
    background: var(--surface-inset);
    color: var(--text-primary);
    font-family: var(--font-body);
    font-size: 13.5px;
    letter-spacing: 0;
    text-transform: none;
  }
  .search input::placeholder {
    color: var(--text-tertiary);
    font-style: normal;
  }
  .search input::-webkit-search-cancel-button {
    display: none;
  }
  .search input:focus {
    outline: none;
    border-color: var(--border-strong);
    box-shadow: var(--focus-ring);
  }
  .search :global(.clear) {
    position: absolute;
    right: 4px;
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    display: flex;
    flex-direction: column;
    gap: 12px;
    scrollbar-width: thin;
    scrollbar-color: var(--color-cork-border) transparent;
    padding: 2px;
    margin: -2px;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .note {
    margin: 0;
    font-size: 13.5px;
    line-height: 1.55;
    color: var(--text-secondary);
  }
  .hits {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .section {
    margin: 0 0 2px;
    font-family: var(--font-body);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--text-tertiary);
  }
  .hit {
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 100%;
    padding: 10px 12px;
    border: 1px solid var(--border-accent);
    border-radius: var(--radius-md);
    background: var(--surface-accent);
    color: var(--text-primary);
    text-align: left;
    font-family: var(--font-body);
    letter-spacing: 0;
    text-transform: none;
    white-space: normal;
    transition: background var(--motion-fast) var(--ease-out);
  }
  .hit:hover {
    background: rgba(220, 80, 0, 0.24);
  }
  .hit:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }
  .hit-meta {
    display: flex;
    align-items: baseline;
    gap: 8px;
    font-size: 12px;
    color: var(--text-secondary);
  }
  .hit-course {
    font-family: var(--font-mono, ui-monospace, monospace);
    color: var(--text-accent);
  }
  .hit-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hit-time {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-family: var(--font-mono, ui-monospace, monospace);
    font-variant-numeric: tabular-nums;
    color: var(--text-primary);
  }
  .hit-text {
    font-size: 13.5px;
    line-height: 1.45;
    color: var(--text-primary);
  }
  @media (max-width: 960px) {
    .sidebar {
      max-height: none;
    }
  }
</style>
