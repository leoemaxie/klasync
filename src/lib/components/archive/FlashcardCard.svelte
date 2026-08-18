<script lang="ts">
  import { triggerHaptic } from '$lib/native/haptics';
  import { RotateCw, CheckCircle2, Circle } from '@lucide/svelte';
  import type { FlashcardItem } from '$lib/utils/flashcards';

  let {
    card,
    currentIndex = 0,
    totalCards = 1,
    onToggleMastery,
    onSwipeLeft,
    onSwipeRight,
  }: {
    card: FlashcardItem;
    currentIndex?: number;
    totalCards?: number;
    onToggleMastery?: (id: string) => void;
    onSwipeLeft?: () => void;
    onSwipeRight?: () => void;
  } = $props();

  let isFlipped = $state(false);
  let touchStartX = 0;
  let touchDeltaX = $state(0);

  $effect(() => {
    card.id;
    isFlipped = false;
  });

  function toggleFlip() {
    triggerHaptic('light');
    isFlipped = !isFlipped;
  }

  function handleTouchStart(e: TouchEvent) {
    touchStartX = e.touches[0].clientX;
    touchDeltaX = 0;
  }
  function handleTouchMove(e: TouchEvent) {
    touchDeltaX = e.touches[0].clientX - touchStartX;
  }
  function handleTouchEnd() {
    if (touchDeltaX < -50) {
      triggerHaptic('selection');
      onSwipeLeft?.();
    } else if (touchDeltaX > 50) {
      triggerHaptic('selection');
      onSwipeRight?.();
    }
    touchDeltaX = 0;
  }
</script>

<div
  class="card-viewport"
  role="region"
  aria-label="Flashcard {currentIndex + 1} of {totalCards}"
  ontouchstart={handleTouchStart}
  ontouchmove={handleTouchMove}
  ontouchend={handleTouchEnd}
>
  <button
    type="button"
    class="flashcard-flipper"
    class:flipped={isFlipped}
    onclick={toggleFlip}
    aria-label="Card {currentIndex + 1}. {isFlipped
      ? 'Showing answer'
      : 'Showing question'}."
  >
    <div class="card-face front" aria-hidden={isFlipped}>
      <div class="face-top">
        <span class="topic-tag">{card.topic_tag || 'QUESTION'}</span>
        <div class="top-meta-right">
          <span class="diff-badge diff-{card.difficulty || 'medium'}"
            >{card.difficulty || 'medium'}</span
          >
          {#if onToggleMastery}
            <button
              type="button"
              class="card-master-btn"
              class:is-mastered={card.mastered}
              onclick={(e) => {
                e.stopPropagation();
                onToggleMastery(card.id);
              }}
              title={card.mastered ? 'Marked as mastered' : 'Mark as mastered'}
            >
              {#if card.mastered}
                <CheckCircle2 size={13} />
                <span>Mastered</span>
              {:else}
                <Circle size={13} />
                <span>Mark Mastered</span>
              {/if}
            </button>
          {/if}
        </div>
      </div>

      <div class="face-content">
        <h3 class="card-prompt">{card.prompt}</h3>
      </div>

      <div class="face-bottom">
        <span class="flip-hint"
          ><RotateCw size={13} /> Tap anywhere to flip</span
        >
        <span class="card-num-indicator">{currentIndex + 1} / {totalCards}</span
        >
      </div>
    </div>

    <div class="card-face back" aria-hidden={!isFlipped}>
      <div class="face-top">
        <span class="topic-tag answer-tag">ANSWER</span>
        <div class="top-meta-right">
          {#if onToggleMastery}
            <button
              type="button"
              class="card-master-btn"
              class:is-mastered={card.mastered}
              onclick={(e) => {
                e.stopPropagation();
                onToggleMastery(card.id);
              }}
            >
              {#if card.mastered}
                <CheckCircle2 size={13} />
                <span>Mastered</span>
              {:else}
                <Circle size={13} />
                <span>Mark Mastered</span>
              {/if}
            </button>
          {/if}
        </div>
      </div>

      <div class="face-content">
        <p class="card-answer">{card.answer}</p>
      </div>

      <div class="face-bottom">
        <span class="flip-hint"><RotateCw size={13} /> Tap to flip back</span>
        <span class="card-num-indicator">{currentIndex + 1} / {totalCards}</span
        >
      </div>
    </div>
  </button>
</div>

<style>
  .card-viewport {
    perspective: 1200px;
    display: flex;
    flex-direction: column;
    width: 100%;
  }
  .flashcard-flipper {
    position: relative;
    width: 100%;
    min-height: 220px;
    background: rgba(22, 13, 7, 0.7);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards, 10px);
    cursor: pointer;
    text-align: left;
    padding: var(--spacing-16);
    transition:
      transform 0.4s cubic-bezier(0.4, 0, 0.2, 1),
      border-color 0.2s,
      box-shadow 0.2s;
    transform-style: preserve-3d;
    user-select: none;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.25);
  }
  .flashcard-flipper:hover {
    border-color: rgba(255, 237, 215, 0.25);
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.35);
  }
  .flashcard-flipper.flipped {
    transform: rotateY(180deg);
  }
  .card-face {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    min-height: 188px;
    backface-visibility: hidden;
  }
  .card-face.back {
    position: absolute;
    inset: var(--spacing-16);
    transform: rotateY(180deg);
  }
  .face-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }
  .top-meta-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .face-content {
    flex: 1;
    display: flex;
    align-items: center;
    padding: var(--spacing-12) 0;
  }
  .face-bottom {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-top: 1px solid rgba(255, 237, 215, 0.06);
    padding-top: var(--spacing-10);
  }
  .topic-tag {
    font-size: 11px;
    font-family: var(--font-mono, monospace);
    letter-spacing: 0.08em;
    color: var(--color-ember-accent);
    background: rgba(220, 80, 0, 0.08);
    border: 1px solid rgba(220, 80, 0, 0.2);
    padding: 3px 8px;
    border-radius: 4px;
    text-transform: uppercase;
    font-weight: 700;
  }
  .answer-tag {
    color: #4ade80;
    background: rgba(74, 222, 128, 0.08);
    border-color: rgba(74, 222, 128, 0.2);
  }
  .diff-badge {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    padding: 3px 8px;
    border-radius: 4px;
    border: 1px solid var(--color-cork-border);
    color: var(--color-warm-cream-dim);
    letter-spacing: 0.04em;
  }
  .diff-hard {
    border-color: var(--color-ember-accent);
    color: var(--color-ember-accent);
    background: rgba(220, 80, 0, 0.08);
  }
  .card-master-btn {
    font-size: 11px;
    font-weight: 600;
    background: rgba(255, 237, 215, 0.04);
    border: 1px solid var(--color-cork-border);
    color: var(--color-warm-cream-dim);
    padding: 3px 9px;
    border-radius: 4px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    min-height: 26px;
    transition:
      background 0.15s ease,
      border-color 0.15s ease,
      color 0.15s ease;
  }
  .card-master-btn:hover {
    border-color: var(--color-warm-cream);
    color: var(--color-warm-cream);
  }
  .card-master-btn.is-mastered {
    border-color: #4ade80;
    color: #4ade80;
    background: rgba(74, 222, 128, 0.12);
  }
  .card-prompt {
    font-family: var(--font-display);
    font-size: 19px;
    color: var(--color-warm-cream);
    line-height: 1.45;
    margin: 0;
    font-weight: 600;
    word-break: break-word;
  }
  .card-answer {
    font-size: 15px;
    color: var(--color-warm-cream);
    line-height: 1.6;
    margin: 0;
    word-break: break-word;
    font-weight: 400;
  }
  .flip-hint {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--color-warm-cream-dim);
  }
  .card-num-indicator {
    font-size: 11px;
    font-family: var(--font-mono, monospace);
    color: var(--color-warm-cream-dim);
  }
</style>
