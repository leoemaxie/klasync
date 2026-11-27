<script lang="ts">
  import { onMount } from 'svelte';
  import { GitCompare } from '@lucide/svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import EmptyState from '$lib/components/ui/EmptyState.svelte';
  import Spinner from '$lib/components/ui/Spinner.svelte';
  import { getCourses } from '$lib/api/courses';
  import type { Course } from '$lib/types';
  import {
    compareOfferings,
    type ComparisonReport,
    type SectionChange,
  } from '$lib/api/materials';

  let { courseId, courseCode }: { courseId: string; courseCode: string } =
    $props();

  let offerings = $state<Course[]>([]);
  let other = $state('');
  let narrative = $state(false);
  let busy = $state(false);
  let error = $state('');
  let report = $state<ComparisonReport | null>(null);
  let loading = $state(true);

  onMount(async () => {
    try {
      const all = await getCourses();
      offerings = all.filter(
        (c) =>
          c.id !== courseId && c.code.toLowerCase() === courseCode.toLowerCase()
      );
      other = offerings[0]?.id ?? '';
    } catch {
      offerings = [];
    } finally {
      loading = false;
    }
  });

  const labelOf = (c: Course) =>
    [c.academic_session, c.semester].filter(Boolean).join(' · ') || c.title;

  async function run() {
    if (!other) return;
    busy = true;
    error = '';
    report = null;
    try {
      // The earlier offering is "a", this course "b".
      report = await compareOfferings(other, courseId, narrative);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Comparison failed';
    } finally {
      busy = false;
    }
  }

  const order: SectionChange[] = [
    'unchanged',
    'modified',
    'moved',
    'added',
    'removed',
  ];
  const changed = $derived(
    report?.changes.filter((c) => c.change !== 'unchanged') ?? []
  );
</script>

{#if loading}
  <div class="center"><Spinner /></div>
{:else if offerings.length === 0}
  <EmptyState
    icon={GitCompare}
    title="No other offering to compare"
    description={`Comparison needs another offering of ${courseCode} that you also teach, with published handouts.`}
  />
{:else}
  <div class="wrap">
    <div class="controls">
      <label class="field">
        <span>Compare this offering with</span>
        <select bind:value={other}>
          {#each offerings as c}<option value={c.id}>
              {labelOf(c)}
            </option>{/each}
        </select>
      </label>
      <label class="check">
        <input type="checkbox" bind:checked={narrative} />
        Write a plain-language summary (uses AI)
      </label>
      <Button variant="accent" disabled={busy} onclick={() => void run()}>
        {busy ? 'Comparing…' : 'Compare'}
      </Button>
    </div>
    {#if error}<p class="err" role="alert">{error}</p>{/if}

    {#if report}
      <p class="muted">
        Method: {report.method === 'embeddings'
          ? 'semantic similarity'
          : 'keyword overlap'}
      </p>
      <div class="summary">
        {#each order as k}
          <div class="stat" data-kind={k}>
            <strong>{report.summary[k] ?? 0}</strong>
            <span>{k}</span>
          </div>
        {/each}
      </div>

      {#if report.narrative}
        <section class="card">
          <h3>Summary</h3>
          <p>{report.narrative}</p>
        </section>
      {/if}

      {#if report.new_topics.length}
        <section class="card">
          <h3>New this time</h3>
          <ul>
            {#each report.new_topics as t}<li>{t}</li>{/each}
          </ul>
        </section>
      {/if}
      {#if report.drift.length}
        <section class="card">
          <h3>Dropped since last time</h3>
          <ul>
            {#each report.drift as t}<li>{t}</li>{/each}
          </ul>
        </section>
      {/if}

      {#if changed.length}
        <section class="card">
          <h3>What changed</h3>
          {#each changed as c}
            <details>
              <summary>
                <span class="badge" data-kind={c.change}>{c.change}</span>
                {c.topic}
              </summary>
              {#each c.diff.added as a}<p class="add">+ {a}</p>{/each}
              {#each c.diff.removed as r}<p class="del">− {r}</p>{/each}
              {#each c.diff.changed as ch}
                <p class="chg"><s>{ch.before}</s><br />{ch.after}</p>
              {/each}
            </details>
          {/each}
        </section>
      {/if}
    {/if}
  </div>
{/if}

<style>
  .center {
    display: grid;
    place-items: center;
    padding: 40px;
  }
  .wrap {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .controls {
    display: flex;
    gap: 14px;
    align-items: flex-end;
    flex-wrap: wrap;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .field span {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-tertiary);
  }
  select {
    padding: 7px 9px;
    border: 1px solid var(--border-default);
    border-radius: 8px;
    background: var(--surface-inset);
    color: var(--text-primary);
    font: inherit;
  }
  .check {
    display: flex;
    gap: 8px;
    align-items: center;
    font-size: 14px;
    color: var(--text-secondary);
  }
  .summary {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(110px, 1fr));
    gap: 8px;
  }
  .stat {
    padding: 10px;
    border: 1px solid var(--border-subtle);
    border-radius: 10px;
    background: var(--surface-1);
    text-align: center;
  }
  .stat strong {
    display: block;
    font-size: 22px;
    color: var(--text-primary);
  }
  .stat span {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-tertiary);
  }
  .card {
    padding: 12px 14px;
    border: 1px solid var(--border-subtle);
    border-radius: 12px;
    background: var(--surface-1);
    color: var(--text-secondary);
    font-size: 14px;
  }
  .card h3 {
    margin: 0 0 6px;
    font-size: 15px;
    color: var(--text-primary);
  }
  ul {
    margin: 0;
    padding-left: 20px;
  }
  details {
    padding: 4px 0;
  }
  summary {
    cursor: pointer;
  }
  .badge {
    padding: 0 7px;
    border: 1px solid var(--border-default);
    border-radius: 999px;
    font-size: 11px;
    text-transform: uppercase;
  }
  .badge[data-kind='added'] {
    color: #7fd3a0;
    border-color: #4c9a68;
  }
  .badge[data-kind='removed'] {
    color: #ff9b7a;
    border-color: #ff9b7a;
  }
  .badge[data-kind='modified'],
  .badge[data-kind='moved'] {
    color: #f0b25c;
    border-color: #c9892b;
  }
  .add {
    color: #7fd3a0;
  }
  .del {
    color: #ff9b7a;
  }
  .chg {
    color: var(--text-secondary);
  }
  p {
    margin: 2px 0 2px 14px;
  }
  .muted {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-tertiary);
  }
  .err {
    margin: 0;
    padding: 8px 12px;
    border: 1px solid #ff9b7a;
    border-radius: 8px;
    color: #ff9b7a;
    font-size: 13px;
  }
  /* The global label/input styles are for the sign-in forms; undo them here. */
  .check {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 8px;
    margin: 0;
    font-size: 14px;
    font-weight: 400;
    letter-spacing: 0;
    text-transform: none;
    color: var(--text-secondary);
  }
  .check input[type='checkbox'] {
    width: auto;
    margin: 0;
    flex: none;
  }
</style>
