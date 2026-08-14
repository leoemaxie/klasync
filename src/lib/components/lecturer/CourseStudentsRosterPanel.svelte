<script lang="ts">
  import type { RosterStudent } from '$lib/types';
  import {
    Users,
    Search,
    Download,
    Trash2,
    CheckCircle2,
    FileSpreadsheet,
    RefreshCw,
  } from '@lucide/svelte';
  import RosterStudentTable from './roster/RosterStudentTable.svelte';

  let {
    roster = $bindable([]),
    courseCode = '',
    onRemoveStudent,
    onClearRoster,
    onReloadFromCloud,
  }: {
    roster: RosterStudent[];
    courseCode?: string;
    onRemoveStudent?: (matric: string) => void;
    onClearRoster?: () => void;
    onReloadFromCloud?: () => Promise<void> | void;
  } = $props();

  let searchQuery = $state('');
  let isReloading = $state(false);

  async function handleReload() {
    if (!onReloadFromCloud) return;
    isReloading = true;
    try {
      await onReloadFromCloud();
    } finally {
      isReloading = false;
    }
  }

  let filteredRoster = $derived(
    roster.filter((s) => {
      if (!searchQuery.trim()) return true;
      const q = searchQuery.toLowerCase().trim();
      return (
        s.name.toLowerCase().includes(q) || s.matric.toLowerCase().includes(q)
      );
    })
  );

  function exportRosterCsv() {
    if (roster.length === 0) return;
    const header = 'matric_number,full_name\n';
    const rows = roster
      .map(
        (s) =>
          `"${s.matric.replace(/"/g, '""')}","${s.name.replace(/"/g, '""')}"`
      )
      .join('\n');
    const blob = new Blob([header + rows], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${courseCode || 'course'}_roster.csv`;
    a.click();
    URL.revokeObjectURL(url);
  }
</script>

<div class="panel course-students-panel">
  <div class="panel-header">
    <div class="header-main">
      <div class="header-icon-circle">
        <Users size={18} color="var(--color-ember-accent)" />
      </div>
      <div>
        <p class="eyebrow">COURSE ROSTER &amp; ENROLLED STUDENTS</p>
        <h3 class="panel-title">
          {courseCode
            ? `Enrolled Students for ${courseCode}`
            : 'Enrolled Course Roster'}
        </h3>
      </div>
    </div>

    <div class="header-actions">
      <span class="count-badge">
        <CheckCircle2 size={13} />
        {roster.length}
        {roster.length === 1 ? 'Student' : 'Students'} Enrolled
      </span>
      {#if onReloadFromCloud}
        <button
          type="button"
          class="outline-btn small"
          onclick={handleReload}
          disabled={isReloading}
          title="Reload roster from cloud API"
        >
          <RefreshCw size={13} class={isReloading ? 'spin' : ''} />
          {isReloading ? 'Syncing...' : 'Reload Cloud'}
        </button>
      {/if}
      {#if roster.length > 0}
        <button
          type="button"
          class="outline-btn small"
          onclick={exportRosterCsv}
          title="Export roster as CSV"
        >
          <Download size={13} /> Export CSV
        </button>
        {#if onClearRoster}
          <button
            type="button"
            class="danger-btn small"
            onclick={onClearRoster}
            title="Clear all roster entries"
          >
            <Trash2 size={13} /> Clear
          </button>
        {/if}
      {/if}
    </div>
  </div>

  {#if roster.length > 0}
    <div class="filter-bar">
      <div class="search-box">
        <Search size={14} class="search-icon" />
        <input
          type="text"
          bind:value={searchQuery}
          placeholder="Search by student name or matric number..."
          aria-label="Search enrolled students"
        />
        {#if searchQuery}
          <button
            type="button"
            class="clear-search-btn"
            onclick={() => (searchQuery = '')}>✕</button
          >
        {/if}
      </div>
      <div class="showing-count">
        Showing {filteredRoster.length} of {roster.length}
      </div>
    </div>

    <RosterStudentTable {filteredRoster} {onRemoveStudent} />
  {:else}
    <div class="empty-roster-state">
      <div class="empty-icon-wrap">
        <FileSpreadsheet size={32} color="var(--color-driftwood)" />
      </div>
      <h4>No Students Enrolled Yet</h4>
      <p class="hint">
        Import a <code>.csv</code> or <code>.xlsx</code> file, or paste CSV rows in
        the panel above to populate the enrolled student list for this course.
      </p>
    </div>
  {/if}
</div>

<style>
  .course-students-panel {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-16);
    padding: var(--spacing-20);
  }
  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--spacing-12);
    border-bottom: 1px solid var(--color-cork-border);
    padding-bottom: var(--spacing-14);
  }
  .header-main {
    display: flex;
    align-items: center;
    gap: var(--spacing-12);
  }
  .header-icon-circle {
    width: 34px;
    height: 34px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(220, 80, 0, 0.08);
    border: 1px solid rgba(220, 80, 0, 0.15);
  }
  .panel-title {
    font-family: var(--font-display);
    font-size: 18px;
    color: var(--color-warm-cream);
    margin: 2px 0 0 0;
  }
  .header-actions {
    display: flex;
    align-items: center;
    gap: var(--spacing-8);
    flex-wrap: wrap;
  }
  .count-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    color: #4ade80;
    background: rgba(40, 167, 69, 0.12);
    padding: 4px 10px;
    border-radius: 999px;
  }
  .outline-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 5px 10px;
    font-size: 11px;
  }
  .danger-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 5px 10px;
    font-size: 11px;
    background: rgba(220, 53, 69, 0.15);
    color: #ff8585;
    border: 1px solid rgba(220, 53, 69, 0.3);
    border-radius: 4px;
    cursor: pointer;
  }
  .filter-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--spacing-12);
    flex-wrap: wrap;
  }
  .search-box {
    position: relative;
    flex: 1;
    display: flex;
    align-items: center;
    min-width: 220px;
  }
  :global(.search-box .search-icon) {
    position: absolute;
    left: 10px;
    color: var(--color-driftwood);
  }
  .search-box input {
    width: 100%;
    padding: 6px 30px 6px 32px;
    font-size: 12px;
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: 4px;
    color: var(--color-warm-cream);
  }
  .clear-search-btn {
    position: absolute;
    right: 8px;
    background: none;
    border: none;
    color: var(--color-driftwood);
    cursor: pointer;
  }
  .showing-count {
    font-size: 11px;
    color: var(--color-driftwood);
  }
  .empty-roster-state {
    padding: var(--spacing-28);
    text-align: center;
    border: 1px dashed var(--color-cork-border);
    border-radius: var(--radius-cards, 6px);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
  }
  .empty-icon-wrap {
    margin-bottom: 4px;
  }
  .empty-roster-state h4 {
    margin: 0;
    font-size: 15px;
    color: var(--color-warm-cream);
  }
  :global(.spin) {
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
</style>
