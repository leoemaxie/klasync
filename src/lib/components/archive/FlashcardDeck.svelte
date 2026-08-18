<script lang="ts">
  import { onMount } from 'svelte';
  import FlashcardCard from './FlashcardCard.svelte';
  import FlashcardGenerator from './FlashcardGenerator.svelte';
  import { triggerHaptic } from '$lib/native/haptics';
  import { fetchSessionFlashcards } from '$lib/api/aiStudy';
  import {
    extractDynamicFlashcards,
    getStoredDeck,
    saveStoredDeck,
    type FlashcardItem,
  } from '$lib/utils/flashcards';
  import { Shuffle, RotateCcw } from '@lucide/svelte';

  let {
    sessionId = 'demo-session',
    transcript = '',
    cards = [],
  }: {
    sessionId?: string;
    transcript?: string;
    cards?: FlashcardItem[];
  } = $props();

  let deck = $state<FlashcardItem[]>([]);
  let currentIndex = $state(0);
  let masteredCount = $derived(deck.filter((c) => c.mastered).length);

  onMount(async () => {
    const stored = getStoredDeck(sessionId);
    if (stored?.length) return (deck = stored);
    if (cards.length > 0) return (deck = [...cards]);
    const apiCards = await fetchSessionFlashcards(sessionId);
    deck = apiCards?.length
      ? apiCards.map((c) => ({ ...c, mastered: false }))
      : extractDynamicFlashcards(transcript);
    saveStoredDeck(sessionId, deck);
  });

  function nextCard() {
    if (!deck.length) return;
    triggerHaptic('light');
    currentIndex = (currentIndex + 1) % deck.length;
  }
  function prevCard() {
    if (!deck.length) return;
    triggerHaptic('light');
    currentIndex = (currentIndex - 1 + deck.length) % deck.length;
  }
  function toggleMastery(id: string) {
    deck = deck.map((c) => (c.id === id ? { ...c, mastered: !c.mastered } : c));
    saveStoredDeck(sessionId, deck);
    triggerHaptic('selection');
  }
  function handleGenerate(topic: string) {
    deck = [...extractDynamicFlashcards(transcript, topic), ...deck];
    saveStoredDeck(sessionId, deck);
    currentIndex = 0;
  }
  function shuffleDeck() {
    triggerHaptic('medium');
    deck = [...deck].sort(() => Math.random() - 0.5);
    currentIndex = 0;
  }
  function resetDeck() {
    triggerHaptic('medium');
    deck = extractDynamicFlashcards(transcript);
    saveStoredDeck(sessionId, deck);
    currentIndex = 0;
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.target instanceof HTMLInputElement) return;
    if (e.key === 'ArrowRight') nextCard();
    if (e.key === 'ArrowLeft') prevCard();
  }}
/>

<div class="panel flashcard-deck">
  <div class="deck-header">
    <div>
      <p class="eyebrow">FLASHCARDS</p>
      <span class="hint">{masteredCount} of {deck.length} mastered</span>
    </div>
    <div class="header-tools">
      <button
        type="button"
        class="tool-btn"
        title="Shuffle"
        onclick={shuffleDeck}><Shuffle size={12} /></button
      >
      <button type="button" class="tool-btn" title="Reset" onclick={resetDeck}
        ><RotateCcw size={12} /></button
      >
      {#if deck.length}<span class="card-counter"
          >{currentIndex + 1} / {deck.length}</span
        >{/if}
    </div>
  </div>

  <FlashcardGenerator onGenerate={handleGenerate} />

  {#if deck.length > 0}
    <div class="deck-progress">
      <div
        class="progress-fill"
        style="width: {((currentIndex + 1) / deck.length) * 100}%"
      ></div>
    </div>
    <FlashcardCard
      card={deck[currentIndex]}
      {currentIndex}
      totalCards={deck.length}
      onToggleMastery={toggleMastery}
      onSwipeLeft={nextCard}
      onSwipeRight={prevCard}
    />
    <div class="deck-actions">
      <button
        type="button"
        class="outline"
        onclick={prevCard}
        disabled={deck.length <= 1}>Previous</button
      >
      <button
        type="button"
        class="primary"
        onclick={nextCard}
        disabled={deck.length <= 1}>Next</button
      >
    </div>
  {:else}
    <p class="hint">No flashcards available. Generate above.</p>
  {/if}
</div>

<style>
  .flashcard-deck {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-8);
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
  .header-tools {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .tool-btn {
    background: transparent;
    border: 1px solid var(--color-cork-border);
    color: var(--color-driftwood);
    padding: 4px;
    border-radius: 4px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
  }
  .tool-btn:hover {
    border-color: var(--color-warm-cream);
    color: var(--color-warm-cream);
  }
  .card-counter {
    font-size: 10px;
    letter-spacing: 0.1em;
    color: var(--color-driftwood);
    font-family: var(--font-mono, monospace);
  }
  .deck-progress {
    height: 2px;
    background: rgba(255, 237, 215, 0.1);
    border-radius: 2px;
    overflow: hidden;
  }
  .progress-fill {
    height: 100%;
    background: var(--color-warm-cream);
    transition: width 0.3s ease;
  }
  .deck-actions {
    display: flex;
    justify-content: space-between;
    gap: var(--spacing-8);
    margin-top: var(--spacing-2);
  }
  .deck-actions button {
    flex: 1;
    padding: 8px 12px;
    font-size: 11px;
    text-transform: uppercase;
  }
</style>
