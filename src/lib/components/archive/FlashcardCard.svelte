<script lang="ts">
  import { triggerHaptic } from '$lib/native/haptics';

  let {
    card,
    currentIndex = 0,
    totalCards = 1,
    onSwipeLeft,
    onSwipeRight,
  }: {
    card: { prompt: string; answer: string };
    currentIndex?: number;
    totalCards?: number;
    onSwipeLeft?: () => void;
    onSwipeRight?: () => void;
  } = $props();

  let isFlipped = $state(false);
  let touchStartX = 0;
  let touchDeltaX = $state(0);

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
    if (touchDeltaX < -60) {
      triggerHaptic('selection');
      isFlipped = false;
      onSwipeLeft?.();
    } else if (touchDeltaX > 60) {
      triggerHaptic('selection');
      isFlipped = false;
      onSwipeRight?.();
    }
    touchDeltaX = 0;
  }
</script>

<div
  class="flashcard-surface"
  class:flipped={isFlipped}
  onclick={toggleFlip}
  ontouchstart={handleTouchStart}
  ontouchmove={handleTouchMove}
  ontouchend={handleTouchEnd}
  role="button"
  tabindex="0"
  aria-label="Flashcard {currentIndex +
    1} of {totalCards}. Tap to flip, swipe left or right."
  onkeydown={(e) => (e.key === ' ' || e.key === 'Enter') && toggleFlip()}
>
  <div class="card-face front" aria-hidden={isFlipped}>
    <span class="card-label">QUESTION / PROMPT</span>
    <h3>{card.prompt}</h3>
    <p class="hint">Tap to flip · Swipe for next card</p>
  </div>
  <div class="card-face back" aria-hidden={!isFlipped}>
    <span class="card-label">EXPLANATION / ANSWER</span>
    <p class="answer-text">{card.answer}</p>
  </div>
</div>

<style>
  .flashcard-surface {
    min-height: 180px;
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: 8px;
    padding: var(--spacing-16);
    cursor: pointer;
    display: flex;
    flex-direction: column;
    justify-content: center;
    position: relative;
    user-select: none;
    touch-action: pan-y;
  }
  .flashcard-surface:hover {
    border-color: var(--color-warm-cream);
  }
  .card-label {
    font-size: 10px;
    letter-spacing: 0.12em;
    color: var(--color-ember-accent);
    margin-bottom: 6px;
    display: block;
  }
  .card-face.front h3 {
    font-size: 16px;
    color: var(--color-warm-cream);
    margin-bottom: 6px;
    line-height: 1.4;
  }
  .answer-text {
    font-size: 14px;
    line-height: 1.5;
    color: var(--color-warm-cream);
  }
  .card-face.back {
    display: none;
  }
  .flashcard-surface.flipped .card-face.front {
    display: none;
  }
  .flashcard-surface.flipped .card-face.back {
    display: block;
  }
</style>
