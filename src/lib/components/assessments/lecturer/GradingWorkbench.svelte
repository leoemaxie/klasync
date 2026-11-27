<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { Check, EyeOff, Sparkles, TriangleAlert } from '@lucide/svelte';
  import {
    confirmAllGrades,
    confirmGrade,
    getGradingQueue,
    listAssessmentJobs,
    putGrade,
    runAiGrading,
    type AssessmentJob,
    type GraderAnswer,
    type GradingQueue,
  } from '$lib/api/assessments';
  import { renderMarkdown } from '$lib/utils/markdown';
  import Button from '$lib/components/ui/Button.svelte';
  import Spinner from '$lib/components/ui/Spinner.svelte';

  let {
    assessmentId,
    locked = false,
    aiMode = false,
    onChanged,
  }: {
    assessmentId: string;
    /** Finalised: grades can no longer change. */
    locked?: boolean;
    /** The assessment uses AI marking. */
    aiMode?: boolean;
    onChanged: () => void;
  } = $props();

  let queue = $state<GradingQueue | null>(null);
  let questionId = $state<string | null>(null);
  let pendingOnly = $state(false);
  let loading = $state(true);
  let error = $state('');
  let notice = $state('');
  let job = $state<AssessmentJob | null>(null);
  let timer: ReturnType<typeof setInterval> | undefined;

  // Working scores, per answer, before they are saved.
  let scores = $state<Record<string, string>>({});
  let feedback = $state<Record<string, string>>({});

  const question = $derived(
    queue?.questions.find((q) => q.id === questionId) ?? null
  );
  const shown = $derived(
    (queue?.answers ?? []).filter(
      (a) => !questionId || a.question_id === questionId
    )
  );

  async function load() {
    try {
      queue = await getGradingQueue(assessmentId, { pendingOnly: false });
      if (!questionId || !queue.questions.some((q) => q.id === questionId)) {
        questionId =
          queue.questions.find((q) => q.pending > 0)?.id ??
          queue.questions[0]?.id ??
          null;
      }
      error = '';
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not load answers';
    } finally {
      loading = false;
    }
  }

  async function checkJob() {
    try {
      const jobs = await listAssessmentJobs(assessmentId);
      const latest =
        jobs.find((j) => j.job_type === 'assessment_grade') ?? null;
      const wasRunning =
        job && (job.status === 'queued' || job.status === 'running');
      job = latest;
      if (
        wasRunning &&
        latest &&
        latest.status !== 'queued' &&
        latest.status !== 'running'
      ) {
        await load();
        onChanged();
      }
    } catch {
      /* keep polling */
    }
  }

  onMount(() => {
    void load();
    void checkJob();
    timer = setInterval(checkJob, 4000);
  });
  onDestroy(() => clearInterval(timer));

  const running = $derived(
    job?.status === 'queued' || job?.status === 'running'
  );

  async function runAi() {
    notice = '';
    try {
      const r = await runAiGrading(assessmentId, questionId ?? undefined);
      notice = r.already_running
        ? 'AI marking is already running.'
        : 'AI marking started. Proposals will appear here.';
      await checkJob();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not start AI marking';
    }
  }

  async function confirmHigh() {
    notice = '';
    try {
      const r = await confirmAllGrades(assessmentId, 0.8);
      notice = `${r.confirmed} confident proposal${r.confirmed === 1 ? '' : 's'} confirmed.`;
      await load();
      onChanged();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not confirm';
    }
  }

  async function save(a: GraderAnswer) {
    const raw = scores[a.answer_id];
    const value =
      raw === undefined || raw === '' ? a.proposal?.score : Number(raw);
    if (value === undefined || Number.isNaN(value)) {
      error = 'Enter a score first.';
      return;
    }
    error = '';
    try {
      await putGrade(assessmentId, a.answer_id, {
        score: value,
        feedback_md:
          feedback[a.answer_id] ??
          a.proposal?.feedback_md ??
          a.feedback_md ??
          '',
      });
      delete scores[a.answer_id];
      await load();
      onChanged();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not save the grade';
    }
  }

  async function accept(a: GraderAnswer) {
    error = '';
    try {
      await confirmGrade(assessmentId, a.answer_id);
      await load();
      onChanged();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not confirm';
    }
  }

  function responseText(a: GraderAnswer) {
    return a.response.text?.trim() ? a.response.text : '(no answer)';
  }

  const graded = (a: GraderAnswer) =>
    a.grade_status === 'confirmed' || a.grade_status === 'overridden';
  const rows = $derived(pendingOnly ? shown.filter((a) => !graded(a)) : shown);
</script>

<div class="wb as-scope">
  <p class="blind">
    <EyeOff size={15} /> You're marking blind. Answers carry a code, not a name, and
    names appear only after you finalise.
  </p>

  {#if loading}
    <Spinner />
  {:else if !queue || queue.questions.length === 0}
    <p class="empty">
      There are no written answers to mark. Choice and true/false answers are
      scored automatically.
    </p>
  {:else}
    <div class="top">
      <div class="qs" role="tablist" aria-label="Question to mark">
        {#each queue.questions as q (q.id)}
          <button
            type="button"
            role="tab"
            class="qtab"
            class:on={q.id === questionId}
            aria-selected={q.id === questionId}
            onclick={() => (questionId = q.id)}
          >
            Q{q.position + 1}
            {#if q.pending > 0}<span class="pend">
                {q.pending}
              </span>{:else}<Check size={13} />{/if}
          </button>
        {/each}
      </div>
      <div class="tools">
        <label class="inline">
          <input type="checkbox" bind:checked={pendingOnly} /> Only unmarked
        </label>
        {#if aiMode && !locked}
          <Button variant="soft" onclick={runAi} disabled={running}>
            {#if running}<Spinner />{:else}<Sparkles size={14} />{/if}
            {running ? 'AI is marking…' : 'Run AI marking'}
          </Button>
          <Button variant="outline" onclick={confirmHigh}>
            Confirm confident proposals
          </Button>
        {/if}
      </div>
    </div>

    {#if job?.status === 'failed'}
      <p class="oops" role="alert">AI marking failed: {job.error}</p>
    {/if}
    {#if notice}<p class="notice" role="status">{notice}</p>{/if}
    {#if error}<p class="oops" role="alert">{error}</p>{/if}

    {#if question}
      <section class="qcard">
        <div class="md">{@html renderMarkdown(question.prompt_md)}</div>
        <p class="meta">
          {question.points} point{question.points === 1 ? '' : 's'} · {queue.total}
          written answers, {queue.pending} still to mark
        </p>
        {#if question.rubric.length > 0}
          <ul class="rubric">
            {#each question.rubric as c (c.id)}
              <li><strong>{c.points}</strong> {c.criterion}</li>
            {/each}
          </ul>
        {/if}
        {#if question.reference}
          <details>
            <summary>Reference answer</summary>
            <p class="ref">
              {Array.isArray(question.reference)
                ? question.reference.join('; ')
                : question.reference}
            </p>
          </details>
        {/if}
      </section>

      <ul class="answers">
        {#each rows as a (a.answer_id)}
          <li class="ans" class:done={graded(a)}>
            <div class="ans-head">
              <strong>{a.alias}</strong>
              {#if graded(a)}
                <span class="tag ok">
                  <Check size={12} />
                  {a.score} / {a.max_score}
                  {#if a.grade_source === 'ai'}· AI, confirmed{:else if a.grade_source === 'lecturer'}·
                    you{/if}
                </span>
              {:else if a.proposal}
                <span class="tag ai">
                  <Sparkles size={12} /> AI proposes {a.proposal.score} / {a.max_score}
                  {#if a.proposal.confidence !== null}· {Math.round(
                      a.proposal.confidence * 100
                    )}% sure{/if}
                </span>
              {/if}
            </div>
            <p class="resp">{responseText(a)}</p>

            {#if a.proposal?.evidence?.flag}
              <p class="warn">
                <TriangleAlert size={14} /> This answer seems to address the marker
                rather than the question. Mark only what answers it.
              </p>
            {/if}
            {#if a.proposal}
              <div class="proposal">
                {#if a.proposal.feedback_md}<p>{a.proposal.feedback_md}</p>{/if}
                {#if a.proposal.rubric_scores?.length}
                  <ul>
                    {#each a.proposal.rubric_scores as r (r.id)}<li>
                        {r.points}/{r.max}: {r.comment}
                      </li>{/each}
                  </ul>
                {/if}
                {#if a.proposal.evidence?.quotes?.length}
                  <blockquote>“{a.proposal.evidence.quotes[0]}”</blockquote>
                {/if}
              </div>
            {/if}

            {#if !locked}
              <div class="grade">
                <label>
                  <span>Score</span>
                  <input
                    type="number"
                    min="0"
                    max={a.max_score}
                    step="0.5"
                    placeholder={String(a.proposal?.score ?? a.score ?? '')}
                    value={scores[a.answer_id] ?? ''}
                    oninput={(e) =>
                      (scores[a.answer_id] = e.currentTarget.value)}
                  />
                  <em>of {a.max_score}</em>
                </label>
                <input
                  class="fb"
                  type="text"
                  placeholder="Feedback for the student (optional)"
                  value={feedback[a.answer_id] ?? a.feedback_md ?? ''}
                  oninput={(e) =>
                    (feedback[a.answer_id] = e.currentTarget.value)}
                />
                {#if a.proposal && !graded(a)}
                  <Button variant="soft" onclick={() => accept(a)}>
                    <Check size={14} /> Accept
                  </Button>
                {/if}
                <Button variant="accent" onclick={() => save(a)}>
                  Save grade
                </Button>
              </div>
            {/if}
          </li>
        {/each}
        {#if rows.length === 0}
          <li class="empty">Nothing to show for this question.</li>
        {/if}
      </ul>
    {/if}
  {/if}
</div>

<style>
  .wb {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .blind {
    margin: 0;
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 9px 14px;
    border-radius: var(--radius-md);
    background: var(--surface-accent);
    font-size: 13px;
    color: var(--text-secondary);
  }
  .top {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
    align-items: center;
  }
  .qs {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .qtab {
    display: inline-flex;
    gap: 6px;
    align-items: center;
    padding: 6px 12px;
    border-radius: 99px;
    border: 1px solid var(--border-default);
    background: var(--surface-2);
    color: var(--text-secondary);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .qtab.on {
    border-color: var(--border-accent);
    background: var(--surface-accent);
    color: var(--text-primary);
  }
  .pend {
    background: var(--fill-accent);
    color: var(--text-on-accent);
    border-radius: 99px;
    font-size: 11px;
    padding: 0 6px;
  }
  .tools {
    display: flex;
    gap: 10px;
    align-items: center;
    flex-wrap: wrap;
  }
  .inline {
    display: flex;
    gap: 6px;
    align-items: center;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .qcard,
  .ans {
    padding: 16px;
    background: var(--surface-2);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
  }
  .meta {
    margin: 8px 0 0;
    font-size: 12.5px;
    color: var(--text-tertiary);
  }
  .rubric {
    margin: 8px 0 0;
    padding-left: 18px;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .ref {
    font-size: 13px;
    color: var(--text-secondary);
  }
  .answers {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .ans.done {
    opacity: 0.85;
  }
  .ans-head {
    display: flex;
    gap: 10px;
    align-items: center;
    flex-wrap: wrap;
  }
  .tag {
    display: inline-flex;
    gap: 5px;
    align-items: center;
    font-size: 12px;
    padding: 2px 9px;
    border-radius: 99px;
    background: var(--surface-inset);
    color: var(--text-secondary);
  }
  .tag.ok {
    color: #5fae7a;
  }
  .tag.ai {
    color: var(--text-accent);
    background: var(--surface-accent);
  }
  .resp {
    margin: 10px 0;
    white-space: pre-wrap;
    line-height: 1.55;
    padding: 12px 14px;
    background: var(--surface-inset);
    border-radius: var(--radius-md);
  }
  .proposal {
    font-size: 13px;
    color: var(--text-secondary);
    margin-bottom: 10px;
  }
  .proposal p,
  .proposal ul,
  .proposal blockquote {
    margin: 4px 0;
  }
  .proposal blockquote {
    border-left: 2px solid var(--border-accent);
    padding-left: 10px;
    color: var(--text-tertiary);
  }
  .warn {
    display: flex;
    gap: 6px;
    align-items: center;
    margin: 0 0 8px;
    font-size: 13px;
    color: #e3a23c;
  }
  .grade {
    display: flex;
    gap: 10px;
    align-items: center;
    flex-wrap: wrap;
  }
  .grade label {
    display: flex;
    gap: 6px;
    align-items: center;
    font-size: 12px;
    color: var(--text-secondary);
  }
  .grade input[type='number'] {
    width: 80px;
  }
  .grade input {
    padding: 8px 10px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-default);
    background: var(--surface-inset);
    color: var(--text-primary);
    font: inherit;
    font-size: 14px;
    text-transform: none;
    letter-spacing: 0;
  }
  .fb {
    flex: 1;
    min-width: 200px;
  }
  em {
    font-style: normal;
    color: var(--text-tertiary);
  }
  .empty {
    color: var(--text-tertiary);
    font-size: 14px;
  }
  .oops {
    margin: 0;
    color: #d9715c;
    font-size: 13.5px;
  }
  .notice {
    margin: 0;
    color: #5fae7a;
    font-size: 13.5px;
  }
</style>
