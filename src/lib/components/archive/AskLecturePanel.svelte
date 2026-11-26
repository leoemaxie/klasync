<script lang="ts">
  import { askLecture, type AskResponse } from '$lib/api/aiStudy';
  import { Send } from '@lucide/svelte';

  let { sessionId = '' }: { sessionId?: string } = $props();

  type Turn = { question: string; result?: AskResponse; error?: string };
  let turns = $state<Turn[]>([]);
  let question = $state('');
  let isAsking = $state(false);

  const clock = (seconds: number) =>
    `${String(Math.floor(seconds / 60)).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}`;

  async function ask(event: SubmitEvent) {
    event.preventDefault();
    const text = question.trim();
    if (text.length < 3 || isAsking) return;
    question = '';
    isAsking = true;
    const turn: Turn = { question: text };
    turns = [turn, ...turns];
    try {
      turn.result = await askLecture(sessionId, text);
    } catch (error) {
      turn.error =
        error instanceof Error ? error.message : 'Unable to answer right now.';
    } finally {
      turns = [...turns];
      isAsking = false;
    }
  }
</script>

<div class="ask-panel">
  <div class="ask-head">
    <h2 class="section-title">Ask this lecture</h2>
    <p class="hint">
      Answers come only from what was said in this lecture, with the times to
      replay.
    </p>
  </div>

  <form class="ask-form" onsubmit={ask}>
    <input
      bind:value={question}
      placeholder="e.g. What did the lecturer say about the Calvin cycle?"
      aria-label="Ask a question about this lecture"
      maxlength="500"
    />
    <button
      type="submit"
      class="primary"
      disabled={isAsking || question.trim().length < 3}
    >
      <Send size={14} /><span>{isAsking ? 'Thinking…' : 'Ask'}</span>
    </button>
  </form>

  {#each turns as turn}
    <article class="turn">
      <p class="q">{turn.question}</p>
      {#if turn.error}
        <p class="error" role="alert">{turn.error}</p>
      {:else if turn.result}
        <p class="a">{turn.result.answer}</p>
        {#if turn.result.sources.length}
          <details class="sources">
            <summary>From the lecture ({turn.result.sources.length})</summary>
            <ul>
              {#each turn.result.sources as source}
                <li>
                  <span class="stamp">{clock(source.start_sec)}</span>
                  {source.text}
                </li>
              {/each}
            </ul>
          </details>
        {/if}
      {:else}
        <p class="hint" role="status">Searching the lecture…</p>
      {/if}
    </article>
  {/each}
</div>

<style>
  .ask-panel {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-10);
    width: 100%;
  }
  .section-title {
    font-size: 15px;
    font-weight: 700;
    color: var(--color-warm-cream);
    margin: 0;
  }
  .hint {
    margin: 2px 0 0;
    font-size: 12px;
    color: var(--color-warm-cream-dim);
  }
  .ask-form {
    display: flex;
    gap: var(--spacing-8);
  }
  .ask-form input {
    flex: 1;
    min-width: 0;
    height: 38px;
    padding: 0 14px;
    font-size: 13px;
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    color: var(--color-warm-cream);
    border-radius: var(--radius-controls, 4px);
    box-sizing: border-box;
  }
  .ask-form button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 38px;
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .turn {
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-controls, 6px);
    padding: var(--spacing-12);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }
  .q {
    margin: 0;
    font-weight: 700;
    font-size: 13px;
    color: var(--color-warm-cream);
  }
  .a {
    margin: 0;
    font-size: 13px;
    line-height: 1.55;
    color: var(--color-warm-cream);
    white-space: pre-wrap;
  }
  .error {
    margin: 0;
    font-size: 12px;
    color: var(--color-error);
  }
  .sources {
    font-size: 12px;
    color: var(--color-warm-cream-dim);
  }
  .sources ul {
    margin: 6px 0 0;
    padding-left: 16px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .stamp {
    font-family: var(--font-mono, monospace);
    color: var(--color-ember-accent);
    margin-right: 6px;
  }
  @media (max-width: 520px) {
    .ask-form {
      flex-direction: column;
    }
  }
</style>
