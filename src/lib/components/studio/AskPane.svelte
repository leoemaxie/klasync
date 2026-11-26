<script lang="ts">
  import { tick } from 'svelte';
  import { Send, Play, Sparkles } from '@lucide/svelte';
  import { askLecture, type AskResponse } from '$lib/api/aiStudy';
  import { useStudio } from '$lib/studio/lectureStudio.svelte';
  import { clock } from '$lib/studio/time';

  const studio = useStudio();

  type Turn = {
    id: number;
    question: string;
    result?: AskResponse;
    error?: string;
  };
  let turns = $state<Turn[]>([]);
  let question = $state('');
  let asking = $state(false);
  let input: HTMLTextAreaElement | undefined = $state();
  let nextId = 0;

  const suggestions = $derived(
    [
      studio.activeChapter
        ? `Explain “${studio.activeChapter.title}” simply`
        : 'What were the main points of this lecture?',
      'What should I revise for the exam?',
      'Define the key terms from this lecture',
    ].slice(0, 3)
  );

  // A draft handed over by "Ask about this" in the transcript.
  $effect(() => {
    if (!studio.askDraft) return;
    question = `Explain this from the lecture: “${studio.askDraft}”`;
    studio.askDraft = '';
    void tick().then(() => {
      input?.focus();
      input?.setSelectionRange(question.length, question.length);
    });
  });

  $effect(() => {
    if (!studio.focusAsk) return;
    studio.focusAsk = false;
    void tick().then(() => input?.focus());
  });

  async function submit(text = question) {
    const q = text.trim();
    if (q.length < 3 || asking) return;
    question = '';
    asking = true;
    const id = nextId++;
    turns = [{ id, question: q }, ...turns];
    // Update through the reactive array (not the object we pushed), so the
    // answer renders as soon as it arrives.
    const patch = (change: Partial<Turn>) => {
      turns = turns.map((t) => (t.id === id ? { ...t, ...change } : t));
    };
    try {
      patch({ result: await askLecture(studio.sessionKey, q) });
    } catch (error) {
      patch({
        error:
          error instanceof Error
            ? error.message
            : 'Unable to answer right now.',
      });
    } finally {
      asking = false;
    }
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      void submit();
    }
  }
</script>

<div class="ask">
  <div class="log" aria-live="polite" aria-relevant="additions">
    {#if turns.length === 0}
      <div class="intro">
        <span class="glyph"><Sparkles size={18} /></span>
        <h3>Ask this lecture</h3>
        <p>
          Answers come only from what was said in this lecture. Each answer
          cites the moments so you can replay them.
        </p>
        <div class="chips">
          {#each suggestions as s}
            <button type="button" class="chip" onclick={() => submit(s)}
              >{s}</button
            >
          {/each}
        </div>
      </div>
    {/if}

    {#each turns as turn (turn.id)}
      <article class="turn">
        <p class="q">{turn.question}</p>
        {#if turn.error}
          <p class="err" role="alert">{turn.error}</p>
        {:else if turn.result}
          <p class="a">{turn.result.answer}</p>
          {#if turn.result.sources.length}
            <div
              class="cites"
              role="group"
              aria-label="Moments cited in this answer"
            >
              <span class="cites-label">Cited moments</span>
              {#each turn.result.sources as source}
                <button
                  type="button"
                  class="cite"
                  title={source.text}
                  onclick={() =>
                    studio.seek(source.start_sec, { play: studio.hasAudio })}
                >
                  <Play size={11} fill="currentColor" />
                  <span class="when">{clock(source.start_sec)}</span>
                  <span class="snippet">{source.text}</span>
                </button>
              {/each}
            </div>
          {/if}
        {:else}
          <p class="thinking" role="status">
            <span class="dots" aria-hidden="true"><i></i><i></i><i></i></span> Searching
            the lecture…
          </p>
        {/if}
      </article>
    {/each}
  </div>

  <form
    class="composer"
    onsubmit={(e) => {
      e.preventDefault();
      void submit();
    }}
  >
    <textarea
      bind:this={input}
      bind:value={question}
      rows="1"
      maxlength="500"
      placeholder="Ask about this lecture…"
      aria-label="Ask a question about this lecture"
      onkeydown={onKeydown}></textarea>
    <button
      type="submit"
      class="send"
      aria-label={asking ? 'Thinking' : 'Send question'}
      disabled={asking || question.trim().length < 3}
    >
      <Send size={16} />
    </button>
  </form>
</div>

<style>
  .ask {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    gap: 10px;
  }
  .log {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 2px 2px 8px;
    scrollbar-width: thin;
    scrollbar-color: var(--color-cork-border) transparent;
  }
  .intro {
    margin: auto;
    max-width: 380px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    text-align: center;
    padding: 12px 4px;
  }
  .glyph {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: var(--surface-accent);
    border: 1px solid var(--border-accent);
    color: var(--text-accent);
  }
  .intro h3 {
    margin: 4px 0 0;
    font-size: 20px;
  }
  .intro p {
    margin: 0;
    font-size: 13.5px;
    line-height: 1.55;
    color: var(--text-secondary);
  }
  .chips {
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 100%;
    margin-top: 8px;
  }
  .chip {
    min-height: var(--control-h);
    padding: 8px 14px;
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-secondary);
    font-family: var(--font-body);
    font-size: 13px;
    text-align: left;
    letter-spacing: 0;
    text-transform: none;
    white-space: normal;
    transition:
      background var(--motion-fast) var(--ease-out),
      border-color var(--motion-fast) var(--ease-out),
      color var(--motion-fast) var(--ease-out);
  }
  .chip:hover {
    background: var(--surface-hover);
    border-color: var(--border-strong);
    color: var(--text-primary);
  }
  .turn {
    display: flex;
    flex-direction: column;
    gap: 10px;
    animation: rise var(--motion-slow) var(--ease-out);
  }
  .q {
    align-self: flex-end;
    max-width: 88%;
    margin: 0;
    padding: 9px 14px;
    border-radius: var(--radius-lg) var(--radius-lg) 4px var(--radius-lg);
    background: var(--surface-3);
    color: var(--text-primary);
    font-size: 13.5px;
    line-height: 1.5;
    word-break: break-word;
  }
  .a {
    margin: 0;
    padding: 2px 4px;
    font-size: 14.5px;
    line-height: 1.65;
    color: var(--text-primary);
    white-space: pre-wrap;
    text-wrap: pretty;
  }
  .err {
    margin: 0;
    padding: 2px 4px;
    font-size: 13px;
    color: var(--color-error);
  }
  .cites {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 0 2px;
  }
  .cites-label {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--text-tertiary);
  }
  .cite {
    display: grid;
    grid-template-columns: auto auto minmax(0, 1fr);
    align-items: center;
    gap: 8px;
    min-height: 38px;
    padding: 7px 12px;
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
    background: var(--surface-1);
    color: var(--text-secondary);
    font-family: var(--font-body);
    font-size: 12.5px;
    letter-spacing: 0;
    text-transform: none;
    text-align: left;
    white-space: normal;
    transition:
      border-color var(--motion-fast) var(--ease-out),
      background var(--motion-fast) var(--ease-out);
  }
  .cite:hover {
    border-color: var(--border-accent);
    background: var(--surface-accent);
    color: var(--text-primary);
  }
  .cite :global(svg) {
    color: var(--text-accent);
  }
  .when {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-variant-numeric: tabular-nums;
    color: var(--text-accent);
  }
  .snippet {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    line-height: 1.45;
  }
  .thinking {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0;
    padding: 2px 4px;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .dots {
    display: inline-flex;
    gap: 3px;
  }
  .dots i {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--text-accent);
    animation: bounce 1.1s var(--ease-in-out) infinite;
  }
  .dots i:nth-child(2) {
    animation-delay: 0.15s;
  }
  .dots i:nth-child(3) {
    animation-delay: 0.3s;
  }
  @keyframes bounce {
    0%,
    80%,
    100% {
      opacity: 0.3;
      transform: translateY(0);
    }
    40% {
      opacity: 1;
      transform: translateY(-3px);
    }
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }
  .composer {
    display: flex;
    align-items: flex-end;
    gap: 8px;
    padding: 6px 6px 6px 14px;
    background: var(--surface-inset);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-xl);
    transition:
      border-color var(--motion-fast) var(--ease-out),
      box-shadow var(--motion-fast) var(--ease-out);
  }
  .composer:focus-within {
    border-color: var(--border-strong);
  }
  textarea {
    flex: 1;
    min-width: 0;
    max-height: 120px;
    min-height: 34px;
    margin: 0;
    padding: 7px 0;
    border: 0;
    background: transparent;
    color: var(--text-primary);
    font-family: var(--font-body);
    font-size: 14px;
    line-height: 1.5;
    letter-spacing: 0;
    text-transform: none;
    resize: none;
    field-sizing: content;
  }
  textarea::placeholder {
    color: var(--text-tertiary);
    font-style: normal;
  }
  textarea:focus,
  textarea:focus-visible {
    outline: none;
    box-shadow: none;
  }
  .send {
    display: grid;
    place-items: center;
    flex: none;
    width: 38px;
    min-width: 38px;
    height: 38px;
    min-height: 38px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: var(--fill-accent);
    color: var(--text-on-accent);
    transition: background var(--motion-fast) var(--ease-out);
  }
  .send:hover:not(:disabled) {
    background: var(--fill-accent-hover);
  }
  .send:focus-visible,
  .cite:focus-visible,
  .chip:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }
  @media (prefers-reduced-motion: reduce) {
    .dots i {
      animation-duration: 1.8s !important;
      animation-iteration-count: infinite !important;
    }
  }
</style>
