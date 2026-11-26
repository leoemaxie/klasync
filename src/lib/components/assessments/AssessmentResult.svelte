<script lang="ts">
  import { onMount } from 'svelte';
  import { ArrowLeft, Check, X, Sparkles } from '@lucide/svelte';
  import {
    getAttemptResult,
    type ResultQuestion,
    type StudentResult,
  } from '$lib/api/assessments';
  import { renderMarkdown } from '$lib/utils/markdown';
  import Spinner from '$lib/components/ui/Spinner.svelte';

  let {
    attemptId,
    title = '',
    onBack,
  }: { attemptId: string; title?: string; onBack: () => void } = $props();

  let result = $state<StudentResult | null>(null);
  let message = $state('');

  onMount(async () => {
    try {
      result = await getAttemptResult(attemptId);
    } catch (error) {
      message =
        error instanceof Error ? error.message : 'Could not load the result';
    }
  });

  const stamp = (sec: number) =>
    `${Math.floor(sec / 60)}:${String(sec % 60).padStart(2, '0')}`;

  function yourAnswer(q: ResultQuestion): string {
    const r = q.response ?? {};
    if (q.question_type === 'true_false') {
      return typeof r.value === 'boolean' ? (r.value ? 'True' : 'False') : '—';
    }
    if (q.question_type === 'mcq_single' || q.question_type === 'mcq_multi') {
      const picked = (r.selected ?? []).map(
        (id) => q.options.find((o) => o.id === id)?.text ?? id
      );
      return picked.length ? picked.join('; ') : '—';
    }
    return r.text?.trim() ? r.text : '—';
  }

  function correctText(q: ResultQuestion): string | null {
    const a = q.correct_answer;
    if (a === undefined || a === null) return null;
    if (typeof a === 'boolean') return a ? 'True' : 'False';
    if (Array.isArray(a)) {
      return a
        .map((x) =>
          typeof x === 'string'
            ? (q.options.find((o) => o.id === x)?.text ?? x)
            : String(x)
        )
        .join('; ');
    }
    return String(a);
  }
</script>

<div class="result">
  <button class="back" type="button" onclick={onBack}
    ><ArrowLeft size={15} /> Assessments</button
  >
  {#if message}
    <p class="notice" role="alert">{message}</p>
  {:else if !result}
    <Spinner />
  {:else}
    <header class="head panel">
      <div>
        <p class="eyebrow">Attempt {result.attempt_no}</p>
        <h2>{result.title || title}</h2>
      </div>
      <div class="score">
        {#if result.complete && result.score !== null}
          <strong>{result.percent}%</strong>
          <span>{result.score} / {result.max_score} points</span>
        {:else}
          <strong>Marking</strong>
          <span>Some answers are still being marked.</span>
        {/if}
      </div>
    </header>

    {#if result.late}
      <p class="note">
        Submitted late{result.penalty_pct > 0
          ? ` · ${result.penalty_pct}% late penalty applied`
          : ''}.
      </p>
    {/if}
    {#if result.ai_assisted}
      <p class="note">
        <Sparkles size={14} /> Some answers were marked with AI assistance and reviewed
        by your lecturer.
      </p>
    {/if}

    {#each result.questions as q, i (q.id)}
      <article class="q panel" data-correct={q.correct}>
        <div class="q-head">
          <span class="num">Question {i + 1}</span>
          {#if q.correct === true}
            <span class="mark ok"><Check size={14} /> Correct</span>
          {:else if q.correct === false}
            <span class="mark bad"><X size={14} /> Not quite</span>
          {/if}
          {#if q.score !== null && q.score !== undefined}
            <span class="pts">{q.score} / {q.points}</span>
          {/if}
        </div>
        <div class="md">{@html renderMarkdown(q.prompt_md)}</div>
        <dl>
          <dt>Your answer</dt>
          <dd class="yours">{yourAnswer(q)}</dd>
          {#if correctText(q)}
            <dt>Correct answer</dt>
            <dd>{correctText(q)}</dd>
          {/if}
          {#if q.feedback_md}
            <dt>Feedback</dt>
            <dd class="md">{@html renderMarkdown(q.feedback_md)}</dd>
          {/if}
          {#if q.explanation_md}
            <dt>Explanation</dt>
            <dd class="md">{@html renderMarkdown(q.explanation_md)}</dd>
          {/if}
        </dl>
        {#if q.source_ranges?.length}
          <p class="moment">
            From the lecture at {stamp(q.source_ranges[0].start_sec)}
          </p>
        {/if}
      </article>
    {/each}
  {/if}
</div>

<style>
  .result {
    display: flex;
    flex-direction: column;
    gap: 12px;
    max-width: 860px;
    margin: 0 auto;
    width: 100%;
  }
  .panel {
    background: var(--surface-2);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    padding: 18px;
  }
  .back {
    align-self: flex-start;
    display: inline-flex;
    gap: 6px;
    align-items: center;
    background: none;
    border: 0;
    color: var(--text-secondary);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
    padding: 0;
  }
  .head {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
    align-items: center;
  }
  .eyebrow {
    margin: 0;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--text-accent);
  }
  h2 {
    margin: 2px 0 0;
    font-size: 24px;
    font-weight: 500;
  }
  .score {
    text-align: right;
    display: flex;
    flex-direction: column;
  }
  .score strong {
    font-size: 34px;
    font-weight: 500;
    line-height: 1.1;
  }
  .score span,
  .note,
  .moment {
    font-size: 13px;
    color: var(--text-tertiary);
  }
  .note {
    margin: 0;
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .notice {
    color: #e3a23c;
  }
  .q-head {
    display: flex;
    gap: 12px;
    align-items: center;
    margin-bottom: 8px;
    font-size: 12.5px;
  }
  .num {
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-accent);
  }
  .mark {
    display: inline-flex;
    gap: 4px;
    align-items: center;
  }
  .mark.ok {
    color: #5fae7a;
  }
  .mark.bad {
    color: #d9715c;
  }
  .pts {
    margin-left: auto;
    font-variant-numeric: tabular-nums;
    color: var(--text-secondary);
  }
  dl {
    margin: 10px 0 0;
    display: grid;
    gap: 4px 14px;
    grid-template-columns: max-content 1fr;
    font-size: 14px;
  }
  dt {
    color: var(--text-tertiary);
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    padding-top: 2px;
  }
  dd {
    margin: 0;
    white-space: pre-wrap;
  }
  .moment {
    margin: 10px 0 0;
  }
  @media (max-width: 560px) {
    dl {
      grid-template-columns: 1fr;
    }
  }
</style>
