<script lang="ts">
  import { onMount } from 'svelte';
  import FlashcardCard from './FlashcardCard.svelte';
  import FlashcardGenerator from './FlashcardGenerator.svelte';
  import SkeletonCard from '$lib/components/shared/SkeletonCard.svelte';
  import { triggerHaptic } from '$lib/native/haptics';
  import {
    fetchSessionFlashcards,
    generateSessionFlashcards,
    runStudyJob,
  } from '$lib/api/aiStudy';
  import {
    getStoredDeck,
    saveStoredDeck,
    type FlashcardItem,
  } from '$lib/utils/flashcards';
  import { Shuffle, RotateCcw, Layers } from '@lucide/svelte';

  let {
    sessionId = '',
    cards = [],
    seedTopic = '',
  }: {
    sessionId?: string;
    cards?: FlashcardItem[];
    /** Pre-fills the generator, e.g. from text selected in the transcript. */
    seedTopic?: string;
  } = $props();

  let deck = $state<FlashcardItem[]>([]);
  let currentIndex = $state(0);
  let isLoading = $state(true);
  let isGenerating = $state(false);
  let errorMessage = $state('');
  let masteredCount = $derived(deck.filter((c) => c.mastered).length);

  async function loadFlashcards() {
    isLoading = true;
    errorMessage = '';
    try {
      if (cards.length > 0) {
        deck = [...cards];
        return;
      }

      if (!sessionId) {
        deck = [];
        return;
      }

      const apiCards = await fetchSessionFlashcards(sessionId).catch(
        (error) => {
          errorMessage =
            error instanceof Error
              ? error.message
              : 'Unable to load flashcards.';
          return null;
        }
      );
      if (apiCards && apiCards.length > 0) {
        const stored = getStoredDeck(sessionId) || [];
        const masteredMap = new Map(stored.map((c) => [c.id, c.mastered]));
        deck = apiCards.map((c) => ({
          id: c.id,
          prompt: c.prompt,
          answer: c.answer,
          topic_tag: c.topic_tag,
          difficulty: c.difficulty,
          mastered: !!masteredMap.get(c.id),
        }));
        saveStoredDeck(sessionId, deck);
        return;
      }

      const stored = getStoredDeck(sessionId);
      if (stored && stored.length > 0) {
        deck = stored;
        return;
      }

      deck = [];
    } finally {
      isLoading = false;
    }
  }

  onMount(() => {
    void loadFlashcards();
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

  function applyCards(
    updated: Awaited<ReturnType<typeof fetchSessionFlashcards>>
  ) {
    const stored = getStoredDeck(sessionId) || [];
    const masteredMap = new Map(stored.map((c) => [c.id, c.mastered]));
    deck = updated.map((c) => ({
      id: c.id,
      prompt: c.prompt,
      answer: c.answer,
      topic_tag: c.topic_tag,
      difficulty: c.difficulty,
      mastered: !!masteredMap.get(c.id),
    }));
    saveStoredDeck(sessionId, deck);
  }

  async function handleGenerate(topic?: string) {
    isGenerating = true;
    errorMessage = '';
    const before = deck.length;
    try {
      // Waits for the job and surfaces the real failure reason (model busy, no
      // transcript yet, ...) rather than silently doing nothing.
      await runStudyJob(() => generateSessionFlashcards(sessionId, topic));
      const updated = await fetchSessionFlashcards(sessionId);
      if (updated.length === 0) {
        errorMessage = 'No flashcards were produced. Please try again.';
        return;
      }
      applyCards(updated);
      currentIndex = Math.min(before, Math.max(updated.length - 1, 0));
    } catch (error) {
      errorMessage =
        error instanceof Error ? error.message : 'Flashcard generation failed.';
    } finally {
      isGenerating = false;
    }
  }

  function shuffleDeck() {
    triggerHaptic('medium');
    deck = [...deck].sort(() => Math.random() - 0.5);
    currentIndex = 0;
  }

  function resetDeck() {
    triggerHaptic('medium');
    void loadFlashcards();
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

<div class="flashcard-deck">
  <div class="deck-header">
    <div class="deck-title-group">
      <h2 class="deck-section-title">Study Flashcards</h2>
      <span class="mastered-pill"
        >{masteredCount} of {deck.length} Mastered</span
      >
    </div>
    <div class="header-tools">
      <button
        type="button"
        class="tool-btn"
        title="Shuffle flashcards"
        onclick={shuffleDeck}
        disabled={deck.length <= 1}
      >
        <Shuffle size={15} />
      </button>
      <button
        type="button"
        class="tool-btn"
        title="Reset order"
        onclick={resetDeck}
        disabled={!deck.length}
      >
        <RotateCcw size={15} />
      </button>
      {#if deck.length}
        <span class="card-counter">{currentIndex + 1} / {deck.length}</span>
      {/if}
    </div>
  </div>

  <FlashcardGenerator onGenerate={handleGenerate} {seedTopic} />

  {#if isGenerating}
    <p class="gen-status" role="status">
      Generating flashcards from the lecture. This can take up to a minute…
    </p>
  {/if}
  {#if errorMessage}
    <p class="gen-error" role="alert">{errorMessage}</p>
  {/if}

  {#if isLoading && deck.length === 0}
    <SkeletonCard lines={3} label="Loading flashcards..." />
  {:else if deck.length > 0}
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
        class="outline deck-nav-btn"
        onclick={prevCard}
        disabled={deck.length <= 1}
      >
        Previous Card
      </button>
      <button
        type="button"
        class="primary deck-nav-btn"
        onclick={nextCard}
        disabled={deck.length <= 1}
      >
        Next Card
      </button>
    </div>
  {:else}
    <div class="empty-flashcards-state">
      <div class="empty-icon-wrap">
        <Layers size={24} color="var(--color-driftwood)" />
      </div>
      <h3 class="empty-title">No Flashcards Yet</h3>
      <p class="empty-desc">
        Generate a deck from the whole lecture, or enter a topic above to focus
        it.
      </p>
      <button
        type="button"
        class="primary"
        onclick={() => handleGenerate()}
        disabled={isGenerating}
      >
        {isGenerating ? 'Generating…' : 'Generate flashcards'}
      </button>
    </div>
  {/if}
</div>

<style>
  .gen-status {
    margin: 0;
    font-size: 12px;
    color: var(--color-warm-cream-dim);
  }
  .gen-error {
    margin: 0;
    font-size: 12px;
    color: var(--color-error);
  }
  .flashcard-deck {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-10);
    width: 100%;
  }
  .deck-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--spacing-8);
    flex-wrap: wrap;
  }
  .deck-title-group {
    display: flex;
    align-items: center;
    gap: var(--spacing-8);
  }
  .deck-section-title {
    font-size: 15px;
    font-weight: 700;
    color: var(--color-warm-cream);
    margin: 0;
    letter-spacing: -0.01em;
  }
  .mastered-pill {
    font-size: 11px;
    font-weight: 600;
    color: #4ade80;
    background: rgba(74, 222, 128, 0.1);
    border: 1px solid rgba(74, 222, 128, 0.2);
    padding: 2px 8px;
    border-radius: 999px;
  }
  .header-tools {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .tool-btn {
    background: rgba(255, 237, 215, 0.04);
    border: 1px solid var(--color-cork-border);
    color: var(--color-warm-cream-dim);
    width: 32px;
    height: 32px;
    min-width: 32px;
    min-height: 32px;
    padding: 0;
    margin: 0;
    border-radius: var(--radius-controls, 4px);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    box-sizing: border-box;
    transition:
      border-color 0.15s ease,
      color 0.15s ease,
      background 0.15s ease;
  }
  .tool-btn:hover {
    border-color: var(--color-warm-cream);
    color: var(--color-warm-cream);
    background: rgba(255, 237, 215, 0.1);
  }
  .card-counter {
    font-size: 11px;
    letter-spacing: 0.05em;
    color: var(--color-warm-cream-dim);
    font-family: var(--font-mono, monospace);
    padding-left: 4px;
    font-weight: 600;
  }
  .deck-progress {
    height: 3px;
    background: rgba(255, 237, 215, 0.08);
    border-radius: 999px;
    overflow: hidden;
  }
  .progress-fill {
    height: 100%;
    background: var(--color-ember-accent);
    transition: width 0.3s ease;
  }
  .deck-actions {
    display: flex;
    justify-content: space-between;
    gap: var(--spacing-10);
    margin-top: var(--spacing-4);
  }
  .deck-nav-btn {
    flex: 1;
    height: 42px;
    min-height: 42px;
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    border-radius: var(--radius-controls, 4px);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    box-sizing: border-box;
  }
  .empty-flashcards-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: var(--spacing-24) var(--spacing-16);
    background: rgba(16, 9, 4, 0.3);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards, 8px);
    gap: var(--spacing-8);
  }
  .empty-icon-wrap {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    background: rgba(255, 237, 215, 0.04);
    border: 1px solid var(--color-cork-border);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .empty-title {
    font-family: var(--font-display);
    font-size: 18px;
    color: var(--color-warm-cream);
    margin: 0;
  }
  .empty-desc {
    font-size: 13px;
    color: var(--color-warm-cream-dim);
    max-width: 380px;
    line-height: 1.5;
    margin: 0;
  }
  @media (max-width: 640px) {
    .deck-header {
      gap: 6px;
    }
    .deck-section-title {
      font-size: 14px;
    }
    .mastered-pill {
      font-size: 10px;
      padding: 2px 6px;
    }
    .deck-actions {
      gap: 8px;
    }
    .deck-nav-btn {
      height: 38px;
      min-height: 38px;
      font-size: 11px;
    }
  }
</style>
