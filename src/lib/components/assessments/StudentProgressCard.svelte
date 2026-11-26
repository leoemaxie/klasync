<script lang="ts">
  import { onMount } from 'svelte';
  import {
    getAssessmentProgress,
    type StudentProgress,
  } from '$lib/api/assessments';
  import Spinner from '$lib/components/ui/Spinner.svelte';

  let { courseId }: { courseId: string } = $props();

  let progress = $state<StudentProgress | null>(null);
  let failed = $state(false);

  onMount(async () => {
    try {
      progress = await getAssessmentProgress(courseId);
    } catch {
      failed = true;
    }
  });

  const pct = (v: number) => `${Math.round(v * 100)}%`;
</script>

<div class="progress">
  {#if failed}
    <p class="muted">Progress isn't available right now.</p>
  {:else if !progress}
    <Spinner />
  {:else if progress.assessments.length === 0}
    <p class="muted">
      Your results will show here once your lecturer releases them.
    </p>
  {:else}
    <div class="cols">
      <section>
        <h4>Results</h4>
        <ul class="bars">
          {#each progress.assessments as a (a.assessment_id)}
            <li>
              <div class="row">
                <span class="name">{a.title}</span>
                <span class="val">{a.percent ?? '–'}%</span>
              </div>
              <div
                class="bar"
                role="img"
                aria-label={`${a.title}: ${a.percent ?? 0} percent`}
              >
                <span style={`width:${Math.min(100, a.percent ?? 0)}%`}></span>
              </div>
            </li>
          {/each}
        </ul>
      </section>
      <section>
        <h4>Topics, weakest first</h4>
        <ul class="bars">
          {#each progress.topics as t (t.topic)}
            <li>
              <div class="row">
                <span class="name">{t.topic}</span>
                <span class="val">{pct(t.mastery)}</span>
              </div>
              <div
                class="bar topic"
                class:weak={t.mastery < 0.5}
                role="img"
                aria-label={`${t.topic}: ${pct(t.mastery)}`}
              >
                <span style={`width:${Math.min(100, t.mastery * 100)}%`}></span>
              </div>
            </li>
          {/each}
        </ul>
      </section>
    </div>
  {/if}
</div>

<style>
  .progress {
    padding: 14px 0 4px;
  }
  .muted {
    margin: 0;
    font-size: 13px;
    color: var(--text-tertiary);
  }
  .cols {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: 20px;
  }
  h4 {
    margin: 0 0 8px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    color: var(--text-tertiary);
  }
  .bars {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .row {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    font-size: 13px;
    margin-bottom: 4px;
  }
  .val {
    font-variant-numeric: tabular-nums;
    color: var(--text-secondary);
  }
  .bar {
    height: 8px;
    border-radius: 99px;
    background: var(--surface-inset);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    background: var(--fill-accent);
    border-radius: 99px;
  }
  .bar.topic span {
    background: #5fae7a;
  }
  .bar.weak span {
    background: #e3a23c;
  }
</style>
