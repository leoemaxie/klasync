<script lang="ts">
  import FlashcardCard from './FlashcardCard.svelte';
  import FlashcardGenerator from './FlashcardGenerator.svelte';
  import { triggerHaptic } from '$lib/native/haptics';

  let { cards = [] }: { cards?: { prompt: string; answer: string }[] } =
    $props();

  const defaultCards = [
    {
      prompt: 'What is the core principle covered in this lecture?',
      answer:
        'The lecture highlights key domain concepts, structured equations, and problem-solving methodologies.',
    },
    {
      prompt: 'How are key definitions verified during assessments?',
      answer:
        'Assessments focus on active recall, formula applications, and clear analytical reasoning.',
    },
    {
      prompt: 'What is the primary take-home objective for revision?',
      answer:
        'Review lecture transcript chapters, practice flashcards, and solidify core definitions.',
    },
  ];

  let customCards = $state<{ prompt: string; answer: string }[]>([]);
  let currentIndex = $state(0);

  let displayCards = $derived([
    ...customCards,
    ...(cards.length > 0 ? cards : defaultCards),
  ]);

  function nextCard() {
    if (!displayCards.length) return;
    triggerHaptic('light');
    currentIndex = (currentIndex + 1) % displayCards.length;
  }

  function prevCard() {
    if (!displayCards.length) return;
    triggerHaptic('light');
    currentIndex =
      (currentIndex - 1 + displayCards.length) % displayCards.length;
  }

  function handleAddCard(topic: string) {
    const newCard = {
      prompt: `Key elements of "${topic}"?`,
      answer: `Summary for "${topic}": Based on lecture material, "${topic}" involves core definitions and practical applications.`,
    };
    customCards = [newCard, ...customCards];
    currentIndex = 0;
  }
</script>

<div class="panel flashcard-deck">
  <div class="deck-header">
    <div>
      <p class="eyebrow">REVISION FLASHCARDS</p>
      <span class="hint">Tap to flip · Swipe left/right on mobile</span>
    </div>
    {#if displayCards.length > 0}
      <span class="card-counter"
        >{currentIndex + 1} OF {displayCards.length}</span
      >
    {/if}
  </div>

  <FlashcardGenerator onGenerate={handleAddCard} />

  {#if displayCards.length > 0}
    <FlashcardCard
      card={displayCards[currentIndex]}
      {currentIndex}
      totalCards={displayCards.length}
      onSwipeLeft={nextCard}
      onSwipeRight={prevCard}
    />

    <div class="deck-actions">
      <button
        type="button"
        class="outline"
        onclick={prevCard}
        disabled={displayCards.length <= 1}>Previous</button
      >
      <button
        type="button"
        class="primary"
        onclick={nextCard}
        disabled={displayCards.length <= 1}>Next Card</button
      >
    </div>
  {:else}
    <p class="hint">No flashcards available.</p>
  {/if}
</div>

<style>
  .flashcard-deck {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-10);
    padding: var(--spacing-14);
    background: rgba(16, 9, 4, 0.4);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards);
  }
  .deck-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .card-counter {
    font-size: 10px;
    letter-spacing: 0.1em;
    color: var(--color-driftwood);
    font-family: var(--font-mono, monospace);
  }
  .deck-actions {
    display: flex;
    justify-content: space-between;
    gap: var(--spacing-8);
  }
  .deck-actions button {
    flex: 1;
    padding: 8px 12px;
    font-size: 11px;
    text-transform: uppercase;
  }
</style>
