<script lang="ts">
  import { onDestroy } from 'svelte';
  import { Sparkles, X } from '@lucide/svelte';
  import {
    generateQuestions,
    listAssessmentJobs,
    QUESTION_TYPE_LABEL,
    type AssessmentJob,
    type QuestionType,
  } from '$lib/api/assessments';
  import Button from '$lib/components/ui/Button.svelte';
  import Spinner from '$lib/components/ui/Spinner.svelte';

  let {
    assessmentId,
    onDone,
    onClose,
  }: {
    assessmentId: string;
    /** Called when new questions were added, so the editor reloads. */
    onDone: () => void;
    onClose: () => void;
  } = $props();

  const ALL: QuestionType[] = [
    'mcq_single',
    'mcq_multi',
    'true_false',
    'short_answer',
    'long_answer',
  ];

  let count = $state(6);
  let types = $state<QuestionType[]>([
    'mcq_single',
    'true_false',
    'short_answer',
  ]);
  let easy = $state(1);
  let medium = $state(2);
  let hard = $state(1);
  let topics = $state('');
  let points = $state(1);
  let phase = $state<'form' | 'running' | 'done' | 'failed'>('form');
  let message = $state('');
  let job = $state<AssessmentJob | null>(null);
  let timer: ReturnType<typeof setInterval> | undefined;

  onDestroy(() => clearInterval(timer));

  function toggle(t: QuestionType) {
    types = types.includes(t) ? types.filter((x) => x !== t) : [...types, t];
  }

  async function start() {
    if (types.length === 0) {
      message = 'Choose at least one question type.';
      return;
    }
    message = '';
    phase = 'running';
    try {
      const { job_id } = await generateQuestions(assessmentId, {
        count,
        types,
        difficulty_mix: [easy, medium, hard],
        topics: topics
          .split(',')
          .map((t) => t.trim())
          .filter(Boolean),
        points_per_question: points,
      });
      timer = setInterval(() => poll(job_id), 2000);
      void poll(job_id);
    } catch (e) {
      phase = 'form';
      message = e instanceof Error ? e.message : 'Could not start generation';
    }
  }

  async function poll(jobId: string) {
    try {
      const jobs = await listAssessmentJobs(assessmentId);
      job = jobs.find((j) => j.id === jobId) ?? null;
      if (job?.status === 'completed') {
        clearInterval(timer);
        phase = 'done';
        onDone();
      } else if (job?.status === 'failed' || job?.status === 'cancelled') {
        clearInterval(timer);
        phase = 'failed';
        message = job.error ?? 'Generation failed';
      }
    } catch {
      // Keep polling; a blip should not abandon a running job.
    }
  }
</script>

<div class="scrim" role="presentation">
  <div
    class="dialog as-scope"
    role="dialog"
    aria-modal="true"
    aria-labelledby="gen-title"
  >
    <header>
      <h3 id="gen-title"><Sparkles size={16} /> Generate questions with AI</h3>
      <button type="button" class="x" aria-label="Close" onclick={onClose}
        ><X size={16} /></button
      >
    </header>

    {#if phase === 'form'}
      <p class="lead">
        Questions are drawn from this lecture's transcript, each linked to the
        moment it came from. They arrive as drafts for you to check, edit or
        delete; students see nothing until you publish.
      </p>
      <div class="grid">
        <label>
          <span>How many</span>
          <input type="number" min="1" max="30" bind:value={count} />
        </label>
        <label>
          <span>Points each</span>
          <input type="number" min="0" step="0.5" bind:value={points} />
        </label>
      </div>
      <fieldset>
        <legend>Question types</legend>
        <div class="chips">
          {#each ALL as t (t)}
            <label class="chip" class:on={types.includes(t)}>
              <input
                type="checkbox"
                checked={types.includes(t)}
                onchange={() => toggle(t)}
              />
              {QUESTION_TYPE_LABEL[t]}
            </label>
          {/each}
        </div>
      </fieldset>
      <fieldset>
        <legend>Difficulty mix <em>relative weights</em></legend>
        <div class="grid three">
          <label
            ><span>Easy</span><input
              type="number"
              min="0"
              max="10"
              bind:value={easy}
            /></label
          >
          <label
            ><span>Medium</span><input
              type="number"
              min="0"
              max="10"
              bind:value={medium}
            /></label
          >
          <label
            ><span>Hard</span><input
              type="number"
              min="0"
              max="10"
              bind:value={hard}
            /></label
          >
        </div>
      </fieldset>
      <label class="wide">
        <span>Focus topics <em>optional, comma separated</em></span>
        <input
          type="text"
          bind:value={topics}
          placeholder="e.g. Fourier's law, convection"
        />
      </label>
      {#if message}<p class="oops" role="alert">{message}</p>{/if}
      <div class="actions">
        <Button variant="ghost" onclick={onClose}>Cancel</Button>
        <Button variant="accent" onclick={start}
          ><Sparkles size={14} /> Generate</Button
        >
      </div>
    {:else if phase === 'running'}
      <div class="status">
        <Spinner />
        <p>
          Writing questions from the transcript… this usually takes under a
          minute.
        </p>
      </div>
    {:else if phase === 'done'}
      <div class="status col">
        <p class="big">
          {job?.result?.added ?? 0} question{job?.result?.added === 1
            ? ''
            : 's'} added
        </p>
        <ul class="notes">
          {#if job?.result?.dropped_invalid}<li>
              {job.result.dropped_invalid} were unusable and dropped.
            </li>{/if}
          {#if job?.result?.duplicates}<li>
              {job.result.duplicates} repeated an existing question and were skipped.
            </li>{/if}
          {#if job?.result?.solve_back_disagreements}
            <li>
              {job.result.solve_back_disagreements}
              {job.result.solve_back_disagreements === 1 ? 'is' : 'are'} marked “Check
              this”: an independent read of the transcript didn't agree with the key.
            </li>
          {/if}
        </ul>
        <Button variant="accent" onclick={onClose}>Review the questions</Button>
      </div>
    {:else}
      <div class="status col">
        <p class="oops" role="alert">{message}</p>
        <div class="actions">
          <Button variant="ghost" onclick={onClose}>Close</Button>
          <Button variant="outline" onclick={() => (phase = 'form')}
            >Try again</Button
          >
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 70;
    background: rgb(0 0 0 / 0.55);
    display: grid;
    place-items: center;
    padding: 16px;
  }
  .dialog {
    width: min(560px, 100%);
    max-height: 90vh;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 20px;
    background: var(--surface-2);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-xl);
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  h3 {
    margin: 0;
    display: flex;
    gap: 8px;
    align-items: center;
    font-size: 17px;
    font-weight: 500;
  }
  .x {
    background: none;
    border: 0;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 4px;
  }
  .lead {
    margin: 0;
    font-size: 13.5px;
    color: var(--text-secondary);
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .grid.three {
    grid-template-columns: repeat(3, 1fr);
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
  }
  em {
    font-weight: 400;
    font-style: normal;
    color: var(--text-tertiary);
    margin-left: 6px;
  }
  fieldset {
    border: 0;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  legend {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    padding: 0;
    margin-bottom: 6px;
  }
  input[type='number'],
  input[type='text'] {
    padding: 9px 11px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-default);
    background: var(--surface-inset);
    color: var(--text-primary);
    font: inherit;
    font-size: 14px;
    text-transform: none;
    letter-spacing: 0;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .chip {
    flex-direction: row;
    align-items: center;
    padding: 7px 13px;
    border-radius: 99px;
    border: 1px solid var(--border-default);
    font-weight: 500;
    cursor: pointer;
    color: var(--text-secondary);
  }
  .chip.on {
    background: var(--surface-accent);
    border-color: var(--border-accent);
    color: var(--text-primary);
  }
  .chip input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  .actions {
    display: flex;
    gap: 10px;
    justify-content: flex-end;
  }
  .status {
    display: flex;
    gap: 12px;
    align-items: center;
    padding: 18px 0;
  }
  .status.col {
    flex-direction: column;
    align-items: stretch;
  }
  .status p {
    margin: 0;
  }
  .big {
    font-size: 20px;
  }
  .notes {
    margin: 0;
    padding-left: 18px;
    color: var(--text-secondary);
    font-size: 13.5px;
  }
  .oops {
    color: #d9715c;
    margin: 0;
    font-size: 13.5px;
  }
</style>
