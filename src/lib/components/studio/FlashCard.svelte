<script lang="ts">
  import { untrack } from 'svelte';
  import { RotateCw, CircleCheck } from '@lucide/svelte';
  import type { FlashcardItem } from '$lib/utils/flashcards';

  let {
    card,
    flipped = false,
    position,
    onFlip,
    onSwipe,
  }: {
    card: FlashcardItem;
    flipped?: boolean;
    /** e.g. "3 of 12", used in the accessible name. */
    position: string;
    onFlip: () => void;
    /** -1 = previous (swipe right), +1 = next (swipe left). */
    onSwipe?: (direction: -1 | 1) => void;
  } = $props();

  // Swapping to another card must not animate (it would flash the next
  // answer while the old card turns back), so it jumps instantly. The element
  // is kept, not re-created, so keyboard focus stays on the flip control.
  let instant = $state(false);
  let lastId = untrack(() => card.id);
  $effect.pre(() => {
    if (card.id === lastId) return;
    lastId = card.id;
    instant = true;
    requestAnimationFrame(() => requestAnimationFrame(() => (instant = false)));
  });

  let startX = 0;
  let startY = 0;

  function onTouchStart(event: TouchEvent) {
    startX = event.touches[0].clientX;
    startY = event.touches[0].clientY;
  }
  function onTouchEnd(event: TouchEvent) {
    const dx = event.changedTouches[0].clientX - startX;
    const dy = event.changedTouches[0].clientY - startY;
    if (Math.abs(dx) > 56 && Math.abs(dx) > Math.abs(dy) * 1.5)
      onSwipe?.(dx < 0 ? 1 : -1);
  }
</script>

<div
  class="card"
  class:flipped
  class:instant
  role="group"
  aria-label="Flashcard {position}"
  ontouchstart={onTouchStart}
  ontouchend={onTouchEnd}
>
  <div class="inner">
    <!-- The hidden face is inert, so it is neither focusable nor announced. -->
    <div class="face front" inert={flipped}>
      <div class="top">
        <span class="tag">{card.topic_tag || 'Question'}</span>
        <span class="diff" data-level={card.difficulty ?? 'medium'}
          >{card.difficulty ?? 'medium'}</span
        >
      </div>
      <p class="body-text prompt">{card.prompt}</p>
      <div class="bottom">
        <span class="flip-hint"
          ><RotateCw size={13} /> Flip to see the answer</span
        >
        {#if card.mastered}<span class="done"
            ><CircleCheck size={14} /> Mastered</span
          >{/if}
      </div>
    </div>
    <div class="face back" inert={!flipped}>
      <div class="top">
        <span class="tag answer">Answer</span>
        {#if card.mastered}<span class="done"
            ><CircleCheck size={14} /> Mastered</span
          >{/if}
      </div>
      <p class="body-text answer-text">{card.answer}</p>
      <div class="bottom">
        <span class="flip-hint"><RotateCw size={13} /> Flip back</span>
      </div>
    </div>
  </div>
  <!-- A sibling (not an ancestor) of the face content, so nothing interactive
       is nested inside a button. -->
  <button
    type="button"
    class="flip-hit"
    onclick={onFlip}
    aria-label={flipped
      ? 'Showing the answer. Press to show the question.'
      : 'Showing the question. Press to show the answer.'}
  ></button>
</div>

<style>
  .card {
    position: relative;
    flex: 1;
    min-height: 200px;
    flex-shrink: 0;
    perspective: 1400px;
  }
  .inner {
    position: absolute;
    inset: 0;
    transform-style: preserve-3d;
    transition: transform 520ms var(--ease-out);
  }
  .instant .inner {
    transition: none;
  }
  .flipped .inner {
    transform: rotateY(180deg);
  }
  .face {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 18px 20px;
    background: linear-gradient(160deg, var(--surface-3), var(--surface-2) 70%);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-xl);
    box-shadow: var(--elev-2);
    backface-visibility: hidden;
    -webkit-backface-visibility: hidden;
    overflow: hidden;
  }
  .back {
    transform: rotateY(180deg);
    border-color: var(--border-accent);
    background: linear-gradient(
      160deg,
      rgba(90, 40, 12, 0.85),
      var(--surface-2) 75%
    );
  }
  .top,
  .bottom {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    flex: none;
  }
  .tag {
    max-width: 70%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 3px 10px;
    border: 1px solid var(--border-default);
    border-radius: 999px;
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 11px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }
  .tag.answer {
    border-color: var(--border-accent);
    color: var(--text-accent);
  }
  .diff {
    font-size: 11.5px;
    text-transform: capitalize;
    color: var(--text-secondary);
  }
  .diff::before {
    content: '';
    display: inline-block;
    width: 7px;
    height: 7px;
    margin-right: 6px;
    border-radius: 50%;
    background: var(--text-tertiary);
  }
  .diff[data-level='easy']::before {
    background: #7ddc9f;
  }
  .diff[data-level='medium']::before {
    background: #f0c060;
  }
  .diff[data-level='hard']::before {
    background: #ff8c4a;
  }
  .body-text {
    flex: 1;
    min-height: 0;
    margin: 0;
    overflow-y: auto;
    display: flex;
    align-items: center;
    color: var(--text-primary);
    overflow-wrap: anywhere;
    text-wrap: pretty;
  }
  .prompt {
    font-family: var(--font-display);
    font-size: clamp(20px, 2.4vw, 26px);
    line-height: 1.3;
  }
  .answer-text {
    font-size: clamp(16px, 1.8vw, 19px);
    line-height: 1.6;
  }
  .flip-hint,
  .done {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
    color: var(--text-secondary);
  }
  .done {
    color: #7ddc9f;
  }
  .flip-hit {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    min-height: 0;
    min-width: 0;
    padding: 0;
    border: 0;
    border-radius: var(--radius-xl);
    background: transparent;
    cursor: pointer;
  }
  .flip-hit:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }

  @media (prefers-reduced-motion: reduce) {
    .inner {
      transition: none;
      transform: none !important;
    }
    .face {
      backface-visibility: visible;
      transform: none;
      transition: opacity 150ms linear;
    }
    .front {
      opacity: 1;
    }
    .back {
      opacity: 0;
    }
    .flipped .front {
      opacity: 0;
    }
    .flipped .back {
      opacity: 1;
    }
  }
</style>
