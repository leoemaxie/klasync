<script lang="ts">
  import { triggerHaptic } from '$lib/native/haptics';
  import { RotateCw, CheckCircle2 } from '@lucide/svelte';
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
    aria-label="Card {currentIndex + 1}. {isFlipped ? 'Showing answer' : 'Showing question'}."
  >
    <div class="card-face front" aria-hidden={isFlipped}>
      <div class="face-top">
        <span class="tag">{card.topic_tag || 'QUESTION'}</span>
        <span class="diff-badge diff-{card.difficulty || 'medium'}">{card.difficulty || 'medium'}</span>
      </div>
      <h3 class="card-prompt">{card.prompt}</h3>
      <div class="face-bottom">
        <span class="flip-hint"><RotateCw size={12} /> Tap to flip</span>
        {#if card.mastered}<span class="mastered-badge"><CheckCircle2 size={12} /> Mastered</span>{/if}
      </div>
    </div>
    <div class="card-face back" aria-hidden={!isFlipped}>
      <div class="face-top">
        <span class="tag">EXPLANATION / ANSWER</span>
        {#if card.mastered}<span class="mastered-badge"><CheckCircle2 size={12} /> Mastered</span>{/if}
      </div>
      <p class="card-answer">{card.answer}</p>
      <div class="face-bottom">
        <span class="flip-hint"><RotateCw size={12} /> Tap to see question</span>
      </div>
    </div>
  </button>
  {#if onToggleMastery}
    <div class="mastery-bar">
      <button
        type="button"
        class="mastery-toggle"
        class:is-mastered={card.mastered}
        onclick={() => onToggleMastery(card.id)}
      >
        <CheckCircle2 size={13} />
        {card.mastered ? 'Mastered · Click to review again' : 'Mark as Mastered'}
      </button>
    </div>
  {/if}
</div>

<style>
  .card-viewport { perspective: 1000px; display: flex; flex-direction: column; gap: var(--spacing-6); }
  .flashcard-flipper { position: relative; width: 100%; min-height: 185px; background: rgba(16, 9, 4, 0.65); border: 1px solid var(--color-cork-border); border-radius: var(--radius-cards, 8px); cursor: pointer; text-align: left; padding: var(--spacing-14); transition: transform 0.4s cubic-bezier(0.4, 0, 0.2, 1), border-color 0.2s; transform-style: preserve-3d; user-select: none; }
  .flashcard-flipper:hover { border-color: var(--color-warm-cream); }
  .flashcard-flipper.flipped { transform: rotateY(180deg); }
  .card-face { display: flex; flex-direction: column; justify-content: space-between; min-height: 155px; backface-visibility: hidden; }
  .card-face.back { position: absolute; inset: var(--spacing-14); transform: rotateY(180deg); }
  .face-top, .face-bottom { display: flex; justify-content: space-between; align-items: center; }
  .tag { font-size: 10px; font-family: var(--font-mono, monospace); letter-spacing: 0.1em; color: var(--color-driftwood); text-transform: uppercase; }
  .diff-badge { font-size: 9px; text-transform: uppercase; padding: 2px 6px; border-radius: 4px; border: 1px solid var(--color-cork-border); color: var(--color-driftwood); }
  .diff-hard { border-color: var(--color-ember-accent); color: var(--color-ember-accent); }
  .card-prompt { font-size: 15px; color: var(--color-warm-cream); line-height: 1.4; margin: var(--spacing-6) 0; font-weight: 500; }
  .card-answer { font-size: 13px; color: var(--color-warm-cream); line-height: 1.55; margin: var(--spacing-6) 0; }
  .flip-hint { display: inline-flex; align-items: center; gap: 4px; font-size: 11px; color: var(--color-driftwood); }
  .mastered-badge { display: inline-flex; align-items: center; gap: 4px; font-size: 11px; color: #a4c952; }
  .mastery-bar { display: flex; justify-content: flex-end; }
  .mastery-toggle { font-size: 11px; background: transparent; border: 1px dashed var(--color-cork-border); color: var(--color-driftwood); padding: 4px 10px; border-radius: 4px; cursor: pointer; display: inline-flex; align-items: center; gap: 6px; }
  .mastery-toggle.is-mastered { border-color: #a4c952; color: #a4c952; background: rgba(164, 201, 82, 0.08); }
</style>
