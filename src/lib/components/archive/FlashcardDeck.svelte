<script lang="ts">
  let cards = $state([
    { prompt: "What is a Feedback Loop in HCI?", answer: "A mechanism that captures output signal, compares it against a desired state, and adjusts future system inputs." },
    { prompt: "Why is audio the primary source of truth in Klasync?", answer: "Because lightweight wireless mic streaming makes auditorium lectures accessible without complex video infrastructure." },
    { prompt: "What defines Cognitive Accessibility in higher education?", answer: "Transforming dense, continuous speech into structured summaries, key concept cards, and digestible transcripts." }
  ]);

  let currentIndex = $state(0);
  let isFlipped = $state(false);

  function nextCard() {
    isFlipped = false;
    currentIndex = (currentIndex + 1) % cards.length;
  }

  function prevCard() {
    isFlipped = false;
    currentIndex = (currentIndex - 1 + cards.length) % cards.length;
  }
</script>

<div class="panel flashcard-deck">
  <div class="deck-header">
    <p class="eyebrow">REVISION FLASHCARDS</p>
    <span class="card-counter">CARD {currentIndex + 1} OF {cards.length}</span>
  </div>

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
    <button class="outline" onclick={prevCard}>Previous Card</button>
    <button class="primary" onclick={nextCard}>Next Card</button>
  </div>
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
</style>
