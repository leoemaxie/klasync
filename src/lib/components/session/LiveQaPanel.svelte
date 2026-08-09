<script lang="ts">
  import { onMount } from 'svelte';
  import {
    fetchSessionQuestions,
    submitQuestion,
    upvoteQuestion,
    resolveQuestion,
    type Question,
  } from '$lib/api';
  import SkeletonCard from '$lib/components/shared/SkeletonCard.svelte';
  import ButtonSpinner from '$lib/components/shared/ButtonSpinner.svelte';
  import { Check, ChevronUp } from '@lucide/svelte';

  let {
    sessionCode = '',
    isLecturer = false,
    participantId = '',
  }: {
    sessionCode: string;
    isLecturer?: boolean;
    participantId?: string;
  } = $props();

  let questions = $state<Question[]>([]);
  let isLoading = $state(true);
  let newQuestionText = $state('');
  let isSubmitting = $state(false);
  let activeUpvotingId = $state<string | null>(null);

  async function loadQuestions() {
    if (!sessionCode) return;
    try {
      questions = await fetchSessionQuestions(sessionCode);
    } catch {
      // Fallback state
    } finally {
      isLoading = false;
    }
  }

  onMount(() => {
    loadQuestions();
    const interval = setInterval(loadQuestions, 5000);
    return () => clearInterval(interval);
  });

  async function handleAddQuestion(e: SubmitEvent) {
    e.preventDefault();
    if (!newQuestionText.trim() || !sessionCode) return;
    isSubmitting = true;
    try {
      const q = await submitQuestion(
        sessionCode,
        newQuestionText.trim(),
        participantId
      );
      questions = [q, ...questions];
      newQuestionText = '';
    } finally {
      isSubmitting = false;
    }
  }

  async function handleUpvote(qId: string) {
    if (!sessionCode) return;
    activeUpvotingId = qId;
    try {
      const res = await upvoteQuestion(sessionCode, qId);
      questions = questions.map((q) =>
        q.id === qId ? { ...q, upvote_count: res.new_upvote_count } : q
      );
    } finally {
      activeUpvotingId = null;
    }
  }

  async function handleResolve(qId: string) {
    if (!sessionCode) return;
    try {
      await resolveQuestion(sessionCode, qId);
      questions = questions.map((q) =>
        q.id === qId ? { ...q, is_resolved: true } : q
      );
    } catch {
      // Toggle locally
    }
  }
</script>

<div class="panel live-qa-panel">
  <div class="qa-header">
    <div>
      <p class="eyebrow">ASSISTIVE STUDENT Q&amp;A</p>
      <h2>
        Live Q&amp;A ({questions.filter((q) => !q.is_resolved).length})
      </h2>
    </div>
    <button type="button" class="text" onclick={loadQuestions}
      >Refresh Q&amp;A</button
    >
  </div>

  {#if !isLecturer}
    <form class="qa-submit-form" onsubmit={handleAddQuestion}>
      <input
        bind:value={newQuestionText}
        placeholder="Ask a quick question linked to live captions..."
        aria-label="Ask a question linked to live captions"
        required
      />
      <button
        type="submit"
        class="primary"
        disabled={isSubmitting || !newQuestionText.trim()}
      >
        {#if isSubmitting}
          <ButtonSpinner label="Submitting question..." /> Submitting...
        {:else}
          Ask Question
        {/if}
      </button>
    </form>
  {/if}

  {#if isLoading}
    <SkeletonCard lines={2} label="Loading live Q&A feed..." />
  {:else if questions.length}
    <div
      class="questions-list"
      role="region"
      aria-live="polite"
      aria-label="Live questions feed"
    >
      {#each questions as q}
        <div class="question-item" class:resolved={q.is_resolved}>
          <div class="q-content">
            <p class="q-text">{q.question_text}</p>
            <span class="q-meta">
              Submitted at {new Date(q.created_at).toLocaleTimeString([], {
                hour: '2-digit',
                minute: '2-digit',
              })}
              {#if q.is_resolved}
                · <strong class="success"
                  ><Check
                    size={12}
                    aria-hidden="true"
                    style="vertical-align: middle; display: inline-block;"
                  /> Resolved</strong
                >{/if}
            </span>
          </div>

          <div class="q-actions">
            <button
              type="button"
              class="outline upvote-btn"
              onclick={() => handleUpvote(q.id)}
              disabled={activeUpvotingId === q.id}
              aria-label="Upvote question. Current upvotes: {q.upvote_count}"
            >
              {#if activeUpvotingId === q.id}
                <ButtonSpinner label="Upvoting..." />
              {:else}
                <ChevronUp
                  size={14}
                  aria-hidden="true"
                  style="vertical-align: middle; display: inline-block;"
                />
                {q.upvote_count}
              {/if}
            </button>

            {#if isLecturer && !q.is_resolved}
              <button
                type="button"
                class="outline"
                onclick={() => handleResolve(q.id)}
              >
                Mark Resolved
              </button>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <p class="hint">No questions submitted.</p>
  {/if}
</div>

<style>
  .live-qa-panel {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-14);
    margin-top: var(--spacing-18);
  }
  .qa-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .qa-submit-form {
    display: flex;
    gap: var(--spacing-12);
  }
  .qa-submit-form input {
    flex: 1;
    min-width: 0;
  }
  .qa-submit-form button {
    white-space: nowrap;
  }
  .questions-list {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-12);
  }
  .question-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--spacing-12);
    background: rgba(16, 9, 4, 0.4);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards);
    gap: var(--spacing-14);
  }
  .question-item.resolved {
    opacity: 0.6;
  }
  .q-text {
    font-size: 15px;
    color: var(--color-warm-cream);
  }
  .q-meta {
    font-size: 10px;
    color: var(--color-driftwood);
  }
  .q-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .upvote-btn {
    padding: 4px 12px;
  }

  @media (max-width: 600px) {
    .qa-submit-form {
      flex-direction: column;
      align-items: stretch;
    }
    .qa-submit-form button {
      width: 100%;
    }
  }
</style>
