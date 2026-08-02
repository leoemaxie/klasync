<script lang="ts">
  let {
    cards = []
  }: {
    cards?: { prompt: string; answer: string }[];
  } = $props();

  let currentIndex = $state(0);
  let isFlipped = $state(false);

  function nextCard() {
    if (!cards.length) return;
    isFlipped = false;
    currentIndex = (currentIndex + 1) % cards.length;
  }

  function prevCard() {
    if (!cards.length) return;
    isFlipped = false;
    currentIndex = (currentIndex - 1 + cards.length) % cards.length;
  }
</script>

<div class="panel flashcard-deck">
  <div class="deck-header">
    <p class="eyebrow">REVISION FLASHCARDS</p>
    {#if cards.length > 0}
      <span class="card-counter">CARD {currentIndex + 1} OF {cards.length}</span>
    {/if}
  </div>

  {#if cards.length > 0}
    <div
      class="flashcard-surface"
      class:flipped={isFlipped}
      onclick={() => (isFlipped = !isFlipped)}
      role="button"
      tabindex="0"
      onkeydown={(e) => e.key === " " && (isFlipped = !isFlipped)}
    >
      <div class="card-face front">
        <span class="card-label">QUESTION / PROMPT</span>
        <h3>{cards[currentIndex].prompt}</h3>
        <p class="hint">Click or press space to reveal answer</p>
      </div>
      <div class="card-face back">
        <span class="card-label">EXPLANATION / ANSWER</span>
        <p class="answer-text">{cards[currentIndex].answer}</p>
      </div>
    </div>

    <div class="deck-actions">
      <button class="outline" onclick={prevCard} disabled={cards.length <= 1}>Previous Card</button>
      <button class="primary" onclick={nextCard} disabled={cards.length <= 1}>Next Card</button>
    </div>
  {:else}
    <div class="empty-flashcard-box">
      <p class="empty-text">No revision flashcards generated for this lecture yet.</p>
    </div>
  {/if}
</div>

<style>
  .flashcard-deck { display: flex; flex-direction: column; gap: var(--spacing-14); }
  .deck-header { display: flex; justify-content: space-between; align-items: center; }
  .card-counter { font-size: 10px; letter-spacing: 0.12em; color: var(--color-driftwood); }
  .flashcard-surface { min-height: 160px; background: rgba(16, 9, 4, 0.6); border: 1px solid var(--color-cork-border); border-radius: 8px; padding: var(--spacing-24); cursor: pointer; display: flex; flex-direction: column; justify-content: center; position: relative; transition: transform 0.3s ease; }
  .flashcard-surface:hover { border-color: var(--color-warm-cream); }
  .card-label { font-size: 9px; letter-spacing: 0.14em; color: var(--color-ember-accent); margin-bottom: 8px; display: block; }
  .answer-text { font-size: 15px; line-height: 1.6; color: var(--color-warm-cream); }
  .deck-actions { display: flex; justify-content: space-between; gap: var(--spacing-12); }
  .empty-flashcard-box { padding: var(--spacing-18); text-align: center; border: 1px dashed var(--color-cork-border); border-radius: 6px; }
  .empty-text { font-size: 13px; color: var(--color-driftwood); }
</style>

