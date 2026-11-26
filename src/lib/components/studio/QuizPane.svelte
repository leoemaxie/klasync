<script lang="ts">
  import { onMount } from 'svelte';
  import {
    ClipboardCheck,
    Sparkles,
    Check,
    X,
    RotateCcw,
    Trophy,
  } from '@lucide/svelte';
  import SkeletonCard from '$lib/components/shared/SkeletonCard.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import EmptyState from '$lib/components/ui/EmptyState.svelte';
  import { triggerHaptic } from '$lib/native/haptics';
  import { useStudio } from '$lib/studio/lectureStudio.svelte';
  import { QuizState } from '$lib/studio/quizState.svelte';

  const studio = useStudio();
  const quiz = new QuizState(studio.sessionKey);
  onMount(() => void quiz.load());

  const QUICK = ['Key concepts', 'Hardest ideas', 'Exam practice'];
  let topic = $state('');

  async function generate(value = topic) {
    triggerHaptic('medium');
    await quiz.generate(value.trim() || undefined);
    if (!quiz.error) topic = '';
  }

  function pick(i: number) {
    triggerHaptic('light');
    quiz.answer(i);
  }

  const letters = ['A', 'B', 'C', 'D', 'E', 'F'];
  const percent = $derived(
    quiz.questions.length
      ? Math.round((quiz.score / quiz.questions.length) * 100)
      : 0
  );
</script>

<div class="quiz" role="region" aria-label="Lecture quiz">
  {#if quiz.generating}
    <p class="status" role="status">
      Writing questions from the lecture. This can take up to a minute…
    </p>
  {/if}
  {#if quiz.error}
    <p class="status err" role="alert">{quiz.error}</p>
  {/if}

  {#if quiz.status === 'loading'}
    <SkeletonCard lines={3} label="Loading quiz…" />
  {:else if !quiz.questions.length}
    <EmptyState
      icon={ClipboardCheck}
      title="No quiz yet"
      description="Test yourself with questions written from this lecture."
    >
      {#if !quiz.generating}
        <Button variant="accent" onclick={() => void generate('')}>
          <Sparkles size={14} /> Generate quiz
        </Button>
      {/if}
    </EmptyState>
  {:else if quiz.finished}
    <div class="result" data-testid="quiz-result">
      <Trophy size={28} aria-hidden="true" />
      <h3>{quiz.score} of {quiz.questions.length} correct</h3>
      <p>
        {percent}% — {percent >= 80
          ? 'Great recall.'
          : percent >= 50
            ? 'Getting there. Review the misses.'
            : 'Worth another look at the transcript.'}
      </p>
      <div class="result-actions">
        <Button variant="outline" onclick={() => quiz.restart()}>
          <RotateCcw size={14} /> Try again
        </Button>
        <Button
          variant="accent"
          disabled={quiz.generating}
          onclick={() => void generate('Hardest ideas')}
        >
          <Sparkles size={14} /> More questions
        </Button>
      </div>
    </div>
  {:else if quiz.current}
    {@const q = quiz.current}
    <div class="progress" aria-hidden="true">
      <span style:width="{(quiz.index / quiz.questions.length) * 100}%"></span>
    </div>
    <p class="count" aria-live="polite">
      Question {quiz.index + 1} of {quiz.questions.length}{q.topic_tag
        ? ` · ${q.topic_tag}`
        : ''}
    </p>
    <h3 class="question">{q.question}</h3>
    <div class="options" role="group" aria-label="Answer options">
      {#each q.options as option, i}
        <button
          type="button"
          class="option"
          class:right={quiz.picked !== null && i === q.correct_index}
          class:wrong={quiz.picked === i && i !== q.correct_index}
          disabled={quiz.picked !== null}
          data-testid="quiz-option"
          onclick={() => pick(i)}
        >
          <span class="letter">{letters[i]}</span>
          <span class="text">{option}</span>
          {#if quiz.picked !== null && i === q.correct_index}
            <Check size={16} aria-label="Correct" />
          {:else if quiz.picked === i}
            <X size={16} aria-label="Incorrect" />
          {/if}
        </button>
      {/each}
    </div>
    {#if quiz.picked !== null}
      <div class="explain" role="status">
        <strong
          >{quiz.picked === q.correct_index ? 'Correct.' : 'Not quite.'}</strong
        >
        {q.explanation ?? ''}
      </div>
      <Button variant="accent" onclick={() => quiz.next()}>
        {quiz.index + 1 === quiz.questions.length ? 'See results' : 'Next'}
      </Button>
    {/if}
  {/if}

  {#if quiz.questions.length && !quiz.finished}
    <form
      class="make"
      onsubmit={(e) => {
        e.preventDefault();
        void generate();
      }}
    >
      <input
        type="text"
        bind:value={topic}
        placeholder="Add questions on a topic…"
        aria-label="Topic for new quiz questions"
        maxlength="200"
      />
      <Button
        size="sm"
        variant="outline"
        type="submit"
        disabled={quiz.generating}><Sparkles size={14} /> Add</Button
      >
      {#each QUICK as q}
        <Button
          size="sm"
          variant="ghost"
          disabled={quiz.generating}
          onclick={() => void generate(q)}>{q}</Button
        >
      {/each}
    </form>
  {/if}
</div>

<style>
  .quiz {
    display: flex;
    flex-direction: column;
    gap: 12px;
    height: 100%;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    scrollbar-width: thin;
    scrollbar-color: var(--color-cork-border) transparent;
  }
  .status {
    margin: 0;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .status.err {
    color: var(--color-error);
  }
  .progress {
    flex: none;
    height: 4px;
    border-radius: 999px;
    background: rgba(255, 237, 215, 0.12);
    overflow: hidden;
  }
  .progress span {
    display: block;
    height: 100%;
    background: var(--color-ember-accent);
    transition: width var(--motion-slow) var(--ease-out);
  }
  .count {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-tertiary);
    font-variant-numeric: tabular-nums;
  }
  .question {
    margin: 0;
    font-size: 17px;
    line-height: 1.4;
    color: var(--text-primary);
  }
  .options {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .option {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    min-height: 48px;
    padding: 10px 14px;
    text-align: left;
    background: var(--surface-1);
    color: var(--text-primary);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-lg);
    font: inherit;
    font-size: 14px;
    cursor: pointer;
  }
  .option:not(:disabled):hover {
    border-color: var(--border-strong);
  }
  .option:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }
  .option:disabled {
    cursor: default;
  }
  .option .text {
    flex: 1;
  }
  .letter {
    flex: none;
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: 999px;
    background: var(--surface-inset);
    font-size: 12px;
    font-weight: 600;
  }
  .option.right {
    border-color: #3fa66b;
    background: rgba(63, 166, 107, 0.16);
  }
  .option.wrong {
    border-color: var(--color-error);
    background: rgba(214, 69, 65, 0.14);
  }
  .explain {
    padding: 10px 12px;
    font-size: 13.5px;
    line-height: 1.45;
    color: var(--text-secondary);
    background: var(--surface-inset);
    border-radius: var(--radius-lg);
  }
  .explain strong {
    color: var(--text-primary);
  }
  .result {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 24px 12px;
    text-align: center;
    color: var(--text-primary);
  }
  .result h3 {
    margin: 0;
    font-size: 22px;
  }
  .result p {
    margin: 0;
    color: var(--text-secondary);
  }
  .result-actions {
    display: flex;
    gap: 8px;
    margin-top: 12px;
    flex-wrap: wrap;
    justify-content: center;
  }
  .make {
    margin-top: auto;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
  }
  .make input {
    flex: 1;
    min-width: 140px;
    height: 36px;
    padding: 0 14px;
    border: 1px solid var(--border-default);
    border-radius: 999px;
    background: var(--surface-1);
    color: var(--text-primary);
    font: inherit;
    font-size: 13px;
  }
  .make input:focus {
    outline: none;
    border-color: var(--border-strong);
    box-shadow: var(--focus-ring);
  }
</style>
