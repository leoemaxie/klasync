<script lang="ts">
  import { Check, TriangleAlert } from '@lucide/svelte';

  let {
    parsedStudents = [],
    rawLinesCount = 0,
  }: {
    parsedStudents: { matric: string; name: string }[];
    rawLinesCount: number;
  } = $props();

  let validCount = $derived(parsedStudents.length);
  let invalidCount = $derived(Math.max(0, rawLinesCount - validCount));
</script>

{#if parsedStudents.length > 0}
  <div class="roster-preview-summary">
    <div class="summary-badge valid">
      <Check size={12} aria-hidden="true" style="vertical-align: middle; display: inline-block;" />
      {validCount} Valid Student{validCount === 1 ? '' : 's'}
    </div>
    {#if invalidCount > 0}
      <div class="summary-badge invalid">
        <TriangleAlert
          size={12}
          aria-hidden="true"
          style="vertical-align: middle; display: inline-block;"
        />
        {invalidCount} Header/Ignored Row{invalidCount === 1 ? '' : 's'}
      </div>
    {/if}
  </div>
  <div class="mapping-table-wrap">
    <table class="mapping-table">
      <thead>
        <tr><th scope="col">Matric No.</th><th scope="col">Student Name</th><th scope="col">Status</th></tr>
      </thead>
      <tbody>
        {#each parsedStudents.slice(0, 5) as student}
          <tr>
            <td><code>{student.matric}</code></td>
            <td>{student.name}</td>
            <td><span class="status-tag match">Verified Match</span></td>
          </tr>
        {/each}
      </tbody>
    </table>
    {#if parsedStudents.length > 5}
      <p class="table-more-hint">
        + {parsedStudents.length - 5} more student records parsed
      </p>
    {/if}
  </div>
{/if}

<style>
  .roster-preview-summary {
    display: flex;
    gap: var(--spacing-8);
    margin-block: 4px;
  }
  .summary-badge {
    font-size: 11px;
    padding: 3px 8px;
    border-radius: 4px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .summary-badge.valid {
    background: rgba(74, 183, 114, 0.15);
    color: var(--color-warm-cream);
    border: 1px solid #4ab772;
  }
  .summary-badge.invalid {
    background: rgba(220, 80, 0, 0.15);
    color: var(--color-ember-accent);
    border: 1px solid var(--color-ember-accent);
  }
  .mapping-table-wrap {
    background: rgba(16, 9, 4, 0.4);
    border: 1px solid var(--color-cork-border);
    border-radius: 6px;
    padding: 8px;
    overflow-x: auto;
  }
  .mapping-table {
    width: 100%;
    font-size: 11px;
    border-collapse: collapse;
    text-align: left;
  }
  .mapping-table th {
    color: var(--color-driftwood);
    font-size: 11px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    padding: 6px 8px;
    border-bottom: 1px solid var(--color-cork-border);
  }
  .mapping-table td {
    padding: 6px 8px;
    border-bottom: 1px solid rgba(64, 55, 46, 0.4);
    color: var(--color-warm-cream);
  }
  .status-tag.match {
    font-size: 11px;
    text-transform: uppercase;
    color: var(--color-warm-cream);
  }
  .table-more-hint {
    font-size: 10px;
    color: var(--color-driftwood);
    margin-top: 6px;
    text-align: center;
  }
</style>
