<script lang="ts">
  import type { RosterStudent } from '$lib/types';
  import { Trash2 } from '@lucide/svelte';

  let {
    filteredRoster = [],
    onRemoveStudent,
  }: {
    filteredRoster: RosterStudent[];
    onRemoveStudent?: (matric: string) => void;
  } = $props();
</script>

<div class="table-container">
  <table class="students-table">
    <thead>
      <tr>
        <th scope="col" style="width: 48px;">#</th>
        <th scope="col">Matric Number</th>
        <th scope="col">Student Name</th>
        <th scope="col">Roster Verification</th>
        {#if onRemoveStudent}<th scope="col" style="text-align: right;">Action</th>{/if}
      </tr>
    </thead>
    <tbody>
      {#each filteredRoster as student, index (student.matric)}
        <tr>
          <td class="row-num">{index + 1}</td>
          <td class="matric-cell"><code>{student.matric}</code></td>
          <td class="name-cell">{student.name}</td>
          <td><span class="status-pill verified">Verified Match</span></td>
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
      {/each}
    </tbody>
  </table>
</div>

<style>
  .table-container {
    overflow-x: auto;
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards, 6px);
  }
  .students-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
    text-align: left;
  }
  .students-table th {
    padding: 10px 14px;
    font-size: 10px;
    letter-spacing: 0.08em;
    color: var(--color-driftwood);
    border-bottom: 1px solid var(--color-cork-border);
    background: rgba(8, 4, 2, 0.4);
    text-transform: uppercase;
  }
  .students-table td {
    padding: 10px 14px;
    border-bottom: 1px solid rgba(64, 55, 46, 0.25);
    color: var(--color-warm-cream);
    vertical-align: middle;
  }
  .row-num {
    color: var(--color-driftwood);
    font-size: 11px;
  }
  .matric-cell code {
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    color: var(--color-warm-cream);
  }
  .name-cell {
    font-weight: 500;
  }
  .status-pill {
    font-size: 10px;
    background: rgba(40, 167, 69, 0.15);
    color: #4ade80;
    padding: 2px 8px;
    border-radius: 999px;
  }
  .action-cell {
    text-align: right;
  }
  .remove-btn {
    background: transparent;
    border: none;
    color: var(--color-driftwood);
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
  }
  .remove-btn:hover {
    color: var(--color-ember-accent);
    background: rgba(220, 80, 0, 0.12);
  }
</style>
