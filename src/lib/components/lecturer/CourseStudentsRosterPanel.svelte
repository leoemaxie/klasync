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
            onclick={() => (searchQuery = '')}
          >
            ✕
          </button>
        {/if}
      </div>
      <div class="showing-count">
        Showing {filteredRoster.length} of {roster.length}
      </div>
    </div>

    <div class="table-container">
      <table class="students-table">
        <thead>
          <tr>
            <th scope="col" style="width: 48px;">#</th>
            <th scope="col">Matric Number</th>
            <th scope="col">Student Name</th>
            <th scope="col">Roster Verification</th>
            {#if onRemoveStudent}<th scope="col" style="text-align: right;"
                >Action</th
              >{/if}
          </tr>
        </thead>
        <tbody>
          {#each filteredRoster as student, index (student.matric)}
            <tr>
              <td class="row-num">{index + 1}</td>
              <td class="matric-cell">
                <code>{student.matric}</code>
              </td>
              <td class="name-cell">{student.name}</td>
              <td>
                <span class="status-pill verified"> Verified Match </span>
              </td>
              {#if onRemoveStudent}
                <td class="action-cell">
                  <button
                    type="button"
                    class="remove-btn"
                    onclick={() => onRemoveStudent?.(student.matric)}
                    title="Remove from roster"
                  >
                    <Trash2 size={13} />
                  </button>
                </td>
              {/if}
            </tr>
          {:else}
            <tr>
              <td colspan={onRemoveStudent ? 5 : 4} class="no-results-cell">
                No students match "{searchQuery}"
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
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
    margin-top: var(--spacing-24);
  }
  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--spacing-16);
    flex-wrap: wrap;
  }
  .header-main {
    display: flex;
    align-items: center;
    gap: var(--spacing-12);
  }
  .header-icon-circle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 38px;
    height: 38px;
    background: rgba(220, 80, 0, 0.1);
    border: 1px solid rgba(220, 80, 0, 0.2);
    border-radius: 50%;
  }
  .panel-title {
    margin: 0;
    font-size: 15px;
    font-weight: 500;
    color: var(--color-warm-cream);
  }
  .header-actions {
    display: flex;
    align-items: center;
    gap: var(--spacing-10);
    flex-wrap: wrap;
  }
  .count-badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    font-weight: 600;
    padding: 4px 10px;
    border-radius: 9999px;
    background: rgba(74, 183, 114, 0.12);
    color: #4ab772;
    border: 1px solid rgba(74, 183, 114, 0.3);
    letter-spacing: 0.04em;
  }
  .outline-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: transparent;
    border: 1px solid var(--color-cork-border);
    color: var(--color-warm-cream);
    padding: 5px 12px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .outline-btn:hover {
    background: rgba(255, 237, 215, 0.08);
    border-color: var(--color-warm-cream-dim);
  }
  .danger-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: rgba(220, 80, 0, 0.1);
    border: 1px solid rgba(220, 80, 0, 0.3);
    color: var(--color-ember-accent);
    padding: 5px 12px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .danger-btn:hover {
    background: rgba(220, 80, 0, 0.2);
    border-color: var(--color-ember-accent);
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
    display: flex;
    align-items: center;
    flex: 1;
    max-width: 380px;
  }
  .search-box input {
    width: 100%;
    padding: 8px 30px 8px 32px;
    font-size: 12px;
    background: rgba(10, 5, 2, 0.5);
    border: 1px solid var(--color-cork-border);
    border-radius: 6px;
    color: var(--color-warm-cream);
  }
  .search-box input:focus {
    border-color: var(--color-ember-accent);
    outline: none;
  }
  :global(.search-icon) {
    position: absolute;
    left: 10px;
    color: var(--color-driftwood);
    pointer-events: none;
  }
  .clear-search-btn {
    position: absolute;
    right: 8px;
    background: transparent;
    border: none;
    color: var(--color-driftwood);
    cursor: pointer;
    font-size: 12px;
    padding: 2px 4px;
  }
  .showing-count {
    font-size: 11px;
    color: var(--color-driftwood);
  }

  .table-container {
    background: rgba(10, 5, 2, 0.4);
    border: 1px solid var(--color-cork-border);
    border-radius: 6px;
    overflow-x: auto;
    max-height: 420px;
    overflow-y: auto;
  }
  .students-table {
    width: 100%;
    border-collapse: collapse;
    text-align: left;
    font-size: 12px;
  }
  .students-table th {
    position: sticky;
    top: 0;
    background: rgba(24, 14, 7, 0.95);
    backdrop-filter: blur(8px);
    color: var(--color-driftwood);
    font-size: 10px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    padding: 10px 12px;
    border-bottom: 1px solid var(--color-cork-border);
    z-index: 2;
  }
  .students-table td {
    padding: 10px 12px;
    border-bottom: 1px solid rgba(64, 55, 46, 0.35);
    color: var(--color-warm-cream);
  }
  .students-table tr:hover td {
    background: rgba(255, 237, 215, 0.03);
  }
  .row-num {
    color: var(--color-driftwood);
    font-size: 11px;
  }
  .matric-cell code {
    background: rgba(255, 255, 255, 0.06);
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 11px;
    border: 1px solid rgba(255, 255, 255, 0.08);
  }
  .name-cell {
    font-weight: 500;
  }
  .status-pill {
    display: inline-flex;
    align-items: center;
    font-size: 10px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 2px 8px;
    border-radius: 4px;
  }
  .status-pill.verified {
    background: rgba(74, 183, 114, 0.12);
    color: #4ab772;
    border: 1px solid rgba(74, 183, 114, 0.25);
  }
  .action-cell {
    text-align: right;
  }
  .remove-btn {
    background: transparent;
    border: none;
    color: var(--color-driftwood);
    cursor: pointer;
    padding: 4px 6px;
    border-radius: 4px;
    transition: color 0.15s ease;
  }
  .remove-btn:hover {
    color: var(--color-ember-accent);
    background: rgba(220, 80, 0, 0.1);
  }
  .no-results-cell {
    text-align: center;
    padding: 24px;
    color: var(--color-driftwood);
    font-style: italic;
  }

  .empty-roster-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 32px 16px;
    text-align: center;
    background: rgba(10, 5, 2, 0.3);
    border: 1px dashed var(--color-cork-border);
    border-radius: 8px;
  }
  .empty-icon-wrap {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 56px;
    height: 56px;
    background: rgba(255, 255, 255, 0.03);
    border-radius: 50%;
    margin-bottom: 12px;
  }
  .empty-roster-state h4 {
    margin: 0 0 6px 0;
    font-size: 15px;
    font-weight: 500;
    color: var(--color-warm-cream);
  }
  .empty-roster-state .hint {
    max-width: 440px;
    font-size: 12px;
    margin: 0;
    line-height: 1.5;
  }
  :global(.spin) {
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    100% {
      transform: rotate(360deg);
    }
  }
</style>
