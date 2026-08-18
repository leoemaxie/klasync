<script lang="ts">
  import type { RosterStudent } from '$lib/types';
  import { Trash2, CheckCircle2 } from '@lucide/svelte';
  import { triggerHaptic } from '$lib/native/haptics';

  let {
    filteredRoster = [],
    onRemoveStudent,
  }: {
    filteredRoster: RosterStudent[];
    onRemoveStudent?: (matric: string) => void;
  } = $props();

  function handleRemove(matric: string) {
    triggerHaptic('warning');
    onRemoveStudent?.(matric);
  }
</script>

<!-- Desktop Table -->
<div class="table-container desktop-only">
  <table class="students-table">
    <thead>
      <tr>
        <th scope="col" style="width: 48px;">#</th>
        <th scope="col">Matric</th>
        <th scope="col">Name</th>
        <th scope="col">Status</th>
        {#if onRemoveStudent}<th scope="col" style="text-align: right;"
            ></th
          >{/if}
      </tr>
    </thead>
    <tbody>
      {#each filteredRoster as student, index (student.matric)}
        <tr>
          <td class="row-num">{index + 1}</td>
          <td class="matric-cell"><code>{student.matric}</code></td>
          <td class="name-cell">{student.name}</td>
          <td><span class="status-pill verified">Verified</span></td>
          {#if onRemoveStudent}
            <td class="action-cell">
              <button
                type="button"
                class="remove-btn"
                onclick={() => handleRemove(student.matric)}
                title="Remove from roster"
                aria-label="Remove {student.name} from roster"
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

<!-- Mobile Cards List -->
<div class="mobile-cards-list mobile-only">
  {#each filteredRoster as student, index (student.matric)}
    <div class="student-card">
      <div class="card-left">
        <span class="student-index">#{index + 1}</span>
        <div class="student-details">
          <span class="student-name">{student.name}</span>
          <code class="student-matric">{student.matric}</code>
        </div>
      </div>
      <div class="card-right">
        <span class="status-pill verified"
          ><CheckCircle2 size={10} /> Verified</span
        >
        {#if onRemoveStudent}
          <button
            type="button"
            class="remove-btn"
            onclick={() => handleRemove(student.matric)}
            title="Remove from roster"
            aria-label="Remove {student.name} from roster"
          >
            <Trash2 size={13} />
          </button>
        {/if}
      </div>
    </div>
  {/each}
</div>

<style>
  .desktop-only {
    display: block;
  }
  .mobile-only {
    display: none;
  }

  .table-container {
    overflow-x: auto;
    border: 1px solid var(--color-cork-border, #40372e);
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
    color: var(--color-driftwood, #b8a794);
    border-bottom: 1px solid var(--color-cork-border, #40372e);
    background: rgba(8, 4, 2, 0.4);
    text-transform: uppercase;
  }
  .students-table td {
    padding: 10px 14px;
    border-bottom: 1px solid rgba(64, 55, 46, 0.25);
    color: var(--color-warm-cream, #ffedd7);
    vertical-align: middle;
  }
  .row-num {
    color: var(--color-driftwood, #b8a794);
    font-size: 11px;
  }
  .matric-cell code {
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    color: var(--color-warm-cream, #ffedd7);
  }
  .name-cell {
    font-weight: 500;
  }
  .status-pill {
    font-size: 10px;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: rgba(40, 167, 69, 0.15);
    border: 1px solid rgba(74, 222, 128, 0.25);
    color: #4ade80;
    padding: 2px 8px;
    border-radius: 999px;
    font-weight: 600;
    white-space: nowrap;
  }
  .action-cell {
    text-align: right;
  }
  .remove-btn {
    background: transparent;
    border: none;
    color: var(--color-driftwood, #b8a794);
    cursor: pointer;
    padding: 6px;
    border-radius: 4px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
  }
  .remove-btn:hover {
    color: var(--color-ember-accent, #dc5000);
    background: rgba(220, 80, 0, 0.12);
  }

  /* Mobile cards view */
  .mobile-cards-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .student-card {
    background: rgba(16, 9, 4, 0.5);
    border: 1px solid var(--color-cork-border, #40372e);
    border-radius: 8px;
    padding: 10px 12px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .card-left {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .student-index {
    font-size: 11px;
    color: var(--color-driftwood, #b8a794);
    font-family: var(--font-mono, monospace);
    font-weight: 600;
  }
  .student-details {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .student-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--color-warm-cream, #ffedd7);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .student-matric {
    font-family: var(--font-mono, monospace);
    font-size: 11px;
    color: var(--color-driftwood, #b8a794);
  }
  .card-right {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  @media (max-width: 640px) {
    .desktop-only {
      display: none;
    }
    .mobile-only {
      display: flex;
    }
  }
</style>
