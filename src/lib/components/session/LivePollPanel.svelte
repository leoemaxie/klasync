<script lang="ts">
  import { onMount } from 'svelte';
  import {
    closePoll,
    createPoll,
    fetchPolls,
    generatePoll,
    respondToPoll,
    type Poll,
  } from '$lib/api/live';
  import ButtonSpinner from '$lib/components/shared/ButtonSpinner.svelte';
  import { triggerHaptic } from '$lib/native/haptics';
  import { Check, Sparkles, X } from '@lucide/svelte';

  let {
    sessionCode = '',
    participantId = '',
    isLecturer = false,
  }: {
    sessionCode: string;
    participantId?: string;
    isLecturer?: boolean;
  } = $props();

  let polls = $state<Poll[]>([]);
  let errorMessage = $state('');
  let busy = $state(false);
  let generating = $state(false);

  // Lecturer composer
  let showComposer = $state(false);
  let kind = $state<Poll['kind']>('quiz');
  let question = $state('');
  let optionsText = $state('');
  let correct = $state(0);

  const active = $derived(polls.find((p) => p.status === 'open'));
  const history = $derived(
    polls.filter((p) => p.status === 'closed').slice(0, 3)
  );

  async function load() {
    if (!sessionCode) return;
    try {
      polls = await fetchPolls(sessionCode, participantId || undefined);
    } catch {
      /* transient; the next poll retries */
    }
  }

  onMount(() => {
    void load();
    const timer = setInterval(load, 4000);
    return () => clearInterval(timer);
  });

  async function answer(poll: Poll, index: number) {
    if (!participantId || poll.my_choice !== null) return;
    triggerHaptic('medium');
    busy = true;
    errorMessage = '';
    try {
      const updated = await respondToPoll(
        sessionCode,
        poll.id,
        participantId,
        index
      );
      polls = polls.map((p) => (p.id === updated.id ? updated : p));
    } catch (error) {
      errorMessage =
        error instanceof Error
          ? error.message
          : 'Unable to submit your answer.';
      void load();
    } finally {
      busy = false;
    }
  }

  async function publish(event: SubmitEvent) {
    event.preventDefault();
    const options = optionsText
      .split('\n')
      .map((o) => o.trim())
      .filter(Boolean);
    busy = true;
    errorMessage = '';
    try {
      await createPoll(sessionCode, {
        kind,
        question: question.trim(),
        options,
        correct_index: kind === 'poll' ? undefined : correct,
      });
      question = '';
      optionsText = '';
      correct = 0;
      showComposer = false;
      await load();
    } catch (error) {
      errorMessage =
        error instanceof Error
          ? error.message
          : 'Unable to publish the question.';
    } finally {
      busy = false;
    }
  }

  async function aiQuiz() {
    generating = true;
    errorMessage = '';
    try {
      await generatePoll(sessionCode, 'quiz');
      await load();
    } catch (error) {
      errorMessage =
        error instanceof Error
          ? error.message
          : 'Could not generate a question.';
    } finally {
      generating = false;
    }
  }

  async function close(poll: Poll) {
    try {
      await closePoll(sessionCode, poll.id);
      await load();
    } catch (error) {
      errorMessage =
        error instanceof Error
          ? error.message
          : 'Unable to close the question.';
    }
  }

  const pct = (poll: Poll, i: number) =>
    poll.total ? Math.round(((poll.counts[i] ?? 0) / poll.total) * 100) : 0;
</script>

{#snippet results(poll: Poll)}
  <ul class="options" aria-label="Results">
    {#each poll.options as option, i}
      <li
        class="result-row"
        class:correct={poll.correct_index === i}
        class:mine={poll.my_choice === i}
      >
        <span class="bar" style="width:{pct(poll, i)}%"></span>
        <span class="label">
          {#if poll.correct_index === i}<Check size={12} />{/if}
          {option}
          {#if poll.my_choice === i}<em>(your answer)</em>{/if}
        </span>
        <span class="count">{poll.counts[i] ?? 0} · {pct(poll, i)}%</span>
      </li>
    {/each}
  </ul>
{/snippet}

{#if isLecturer || active || history.length}
  <section
    class="panel live-poll-panel"
    aria-label="Live questions and quizzes"
  >
    <div class="poll-head">
      <div>
        <p class="eyebrow">
          {isLecturer ? 'CHECK UNDERSTANDING' : 'LIVE CHECK'}
        </p>
        <h2>{isLecturer ? 'Quizzes & polls' : 'Quick question'}</h2>
      </div>
      {#if isLecturer}
        <div class="lecturer-actions">
          <button
            type="button"
            class="outline"
            onclick={aiQuiz}
            disabled={generating}
            title="Write a question from what was just said"
          >
            {#if generating}<ButtonSpinner label="Generating" />{:else}<Sparkles
                size={13}
              />{/if}
            <span>AI quiz</span>
          </button>
          <button
            type="button"
            class="primary"
            onclick={() => (showComposer = !showComposer)}
          >
            {showComposer ? 'Cancel' : 'New'}
          </button>
        </div>
      {/if}
    </div>

    {#if errorMessage}<p class="poll-error" role="alert">{errorMessage}</p>{/if}

    {#if isLecturer && showComposer}
      <form class="composer" onsubmit={publish}>
        <div class="row">
          <select bind:value={kind} aria-label="Question type">
            <option value="quiz">Quiz (has a right answer)</option>
            <option value="poll">Poll (opinion)</option>
            <option value="comprehension">Comprehension check</option>
          </select>
        </div>
        <input
          bind:value={question}
          placeholder="Question"
          aria-label="Question"
          required
        />
        <textarea
          bind:value={optionsText}
          rows="4"
          placeholder={'One answer per line (2 to 6)'}
          aria-label="Answer options"
          required></textarea>
        {#if kind !== 'poll'}
          <label class="correct-pick">
            Correct answer (line #)
            <input
              type="number"
              min="1"
              max="6"
              value={correct + 1}
              oninput={(e) =>
                (correct = Math.max(0, Number(e.currentTarget.value) - 1))}
            />
          </label>
        {/if}
        <button
          type="submit"
          class="primary"
          disabled={busy || !question.trim()}
        >
          Publish to students
        </button>
      </form>
    {/if}

    {#if active}
      <article class="poll" aria-live="polite">
        <h3>{active.question}</h3>
        {#if isLecturer || active.my_choice !== null}
          {@render results(active)}
          {#if active.explanation}<p class="why">{active.explanation}</p>{/if}
        {:else}
          <ul class="options">
            {#each active.options as option, i}
              <li>
                <button
                  type="button"
                  class="outline option-btn"
                  disabled={busy || !participantId}
                  onclick={() => answer(active, i)}
                >
                  {option}
                </button>
              </li>
            {/each}
          </ul>
          {#if !participantId}<p class="poll-hint">
              Join the session to answer.
            </p>{/if}
        {/if}
        <div class="poll-foot">
          <span>
            {active.total}
            {active.total === 1 ? 'answer' : 'answers'}
          </span>
          {#if isLecturer}
            <button type="button" class="text" onclick={() => close(active)}>
              <X size={12} /> Close &amp; reveal
            </button>
          {/if}
        </div>
      </article>
    {:else if isLecturer && !showComposer}
      <p class="poll-hint">
        No live question. Publish one, or let the AI write one from the last few
        minutes.
      </p>
    {/if}

    {#each history as poll (poll.id)}
      <article class="poll closed">
        <h3>{poll.question}</h3>
        {@render results(poll)}
        {#if poll.explanation}<p class="why">{poll.explanation}</p>{/if}
      </article>
    {/each}
  </section>
{/if}

<style>
  .live-poll-panel {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-10);
  }
  .poll-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: var(--spacing-8);
    flex-wrap: wrap;
  }
  .poll-head h2 {
    margin: 2px 0 0;
    font-size: 16px;
  }
  .lecturer-actions {
    display: flex;
    gap: 6px;
  }
  .lecturer-actions button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .poll-error {
    margin: 0;
    font-size: 12px;
    color: var(--color-error);
  }
  .poll-hint {
    margin: 0;
    font-size: 12px;
    color: var(--color-warm-cream-dim);
  }
  .composer {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .composer input,
  .composer textarea,
  .composer select {
    width: 100%;
    box-sizing: border-box;
  }
  .correct-pick {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--color-warm-cream-dim);
  }
  .correct-pick input {
    width: 70px;
  }
  .poll {
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-controls, 6px);
    padding: var(--spacing-12);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-8);
  }
  .poll.closed {
    opacity: 0.8;
  }
  .poll h3 {
    margin: 0;
    font-size: 14px;
    line-height: 1.4;
  }
  .options {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .option-btn {
    width: 100%;
    text-align: left;
    min-height: 40px;
  }
  .result-row {
    position: relative;
    display: flex;
    justify-content: space-between;
    gap: 8px;
    padding: 8px 10px;
    border: 1px solid var(--color-cork-border);
    border-radius: 4px;
    overflow: hidden;
    font-size: 13px;
  }
  .result-row .bar {
    position: absolute;
    inset: 0 auto 0 0;
    background: rgba(220, 80, 0, 0.18);
  }
  .result-row.correct {
    border-color: #7fb069;
  }
  .result-row.correct .bar {
    background: rgba(127, 176, 105, 0.28);
  }
  .result-row.mine {
    outline: 1px solid var(--color-warm-cream);
  }
  .result-row .label,
  .result-row .count {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .result-row em {
    font-style: normal;
    color: var(--color-warm-cream-dim);
    font-size: 11px;
  }
  .why {
    margin: 0;
    font-size: 12px;
    color: var(--color-warm-cream-dim);
  }
  .poll-foot {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 11px;
    color: var(--color-warm-cream-dim);
  }
</style>
