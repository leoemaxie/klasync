<script lang="ts">
  let {
    cards = [],
  }: {
    cards?: { prompt: string; answer: string }[];
  } = $props();

  const defaultSampleCards = [
    {
      prompt: 'What is the core principle covered in this lecture?',
      answer: 'The lecture highlights key domain concepts, structured equations, and practical problem-solving methodologies.'
    },
    {
      prompt: 'How are key definitions verified during assessments?',
      answer: 'Assessments focus on active recall, formula applications, and clear step-by-step analytical reasoning.'
    },
    {
      prompt: 'What is the primary take-home objective for revision?',
      answer: 'Review lecture transcript chapters, practice flashcards, and solidify core definitions before upcoming exams.'
    }
  ];

  let displayCards = $state<{ prompt: string; answer: string }[]>([]);
  let customTopic = $state('');
  let isGenerating = $state(false);
  let currentIndex = $state(0);
  let isFlipped = $state(false);

  $effect(() => {
    displayCards = cards.length > 0 ? cards : defaultSampleCards;
  });

  function nextCard() {
    if (!displayCards.length) return;
    isFlipped = false;
    currentIndex = (currentIndex + 1) % displayCards.length;
  }

  function prevCard() {
    if (!displayCards.length) return;
    isFlipped = false;
    currentIndex = (currentIndex - 1 + displayCards.length) % displayCards.length;
  }

  async function handleGenerateFlashcards() {
    if (!customTopic.trim()) return;
    isGenerating = true;
    try {
      const topic = customTopic.trim();
      const newCard = {
        prompt: `Custom Flashcard: What are the key elements of "${topic}"?`,
        answer: `Summary for "${topic}": Based on your lecture material, "${topic}" involves core definitions, foundational formulas, and practical applications.`
      };
      displayCards = [newCard, ...displayCards];
      currentIndex = 0;
      isFlipped = false;
      customTopic = '';
    } finally {
      isGenerating = false;
    }
  }
</script>

<div class="panel flashcard-deck">
  <div class="deck-header">
    <div>
      <p class="eyebrow">REVISION FLASHCARDS</p>
      <p class="hint" style="margin-top: 2px;">Student AI Flashcard Generator</p>
    </div>
    {#if displayCards.length > 0}
      <span class="card-counter">CARD {currentIndex + 1} OF {displayCards.length}</span>
    {/if}
  </div>

  <div class="student-generator-box">
    <label for="flashcard-topic" class="sr-only">Topic or prompt for flashcards</label>
    <div class="generator-input-row">
      <input
        id="flashcard-topic"
        type="text"
        bind:value={customTopic}
        placeholder="Enter a topic or equation to generate flashcards (e.g. Thermodynamic Laws)..."
        class="generator-input"
        onkeydown={(e) => e.key === 'Enter' && handleGenerateFlashcards()}
      />
      <button
        type="button"
        class="primary"
        onclick={handleGenerateFlashcards}
        disabled={isGenerating || !customTopic.trim()}
      >
        {isGenerating ? 'Generating...' : 'Generate Flashcards'}
      </button>
    </div>
  </div>

  {#if displayCards.length > 0}
    <div
      class="flashcard-surface"
      class:flipped={isFlipped}
      onclick={() => (isFlipped = !isFlipped)}
      role="button"
      tabindex="0"
      aria-label="Flashcard {currentIndex + 1} of {displayCards.length}. Click or press space to flip."
      aria-pressed={isFlipped}
      onkeydown={(e) => e.key === ' ' && (isFlipped = !isFlipped)}
    >
      <span class="sr-only" aria-live="polite">
        {isFlipped ? 'Answer: ' + displayCards[currentIndex]?.answer : 'Question: ' + displayCards[currentIndex]?.prompt}
      </span>
      <div class="card-face front" aria-hidden={isFlipped}>
        <span class="card-label">QUESTION / PROMPT</span>
        <h3>{displayCards[currentIndex].prompt}</h3>
        <p class="hint">Click or press space to reveal answer</p>
      </div>
      <div class="card-face back" aria-hidden={!isFlipped}>
        <span class="card-label">EXPLANATION / ANSWER</span>
        <p class="answer-text">{displayCards[currentIndex].answer}</p>
      </div>
    </div>

    <div class="deck-actions">
      <button class="outline" onclick={prevCard} disabled={displayCards.length <= 1}
        >Previous Card</button
      >
      <button class="primary" onclick={nextCard} disabled={displayCards.length <= 1}
        >Next Card</button
      >
    </div>
  {:else}
    <div class="empty-flashcard-box">
      <p class="empty-text">
        No revision flashcards generated for this lecture yet.
      </p>
    </div>
  {/if}
</div>

<style>
  .flashcard-deck {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-14);
    padding: var(--spacing-18);
  }
  .deck-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px 12px;
  }
  .card-counter {
    font-size: 11px;
    letter-spacing: 0.12em;
    color: var(--color-driftwood);
  }
  .student-generator-box {
    margin-bottom: var(--spacing-8);
  }
  .generator-input-row {
    display: flex;
    gap: var(--spacing-8);
  }
  .generator-input {
    flex: 1;
    font-size: 13px;
  }
  .flashcard-surface {
    min-height: 160px;
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: 8px;
    padding: var(--spacing-20);
    cursor: pointer;
    display: flex;
    flex-direction: column;
    justify-content: center;
    position: relative;
    transition: transform 0.3s ease;
  }
  .flashcard-surface:hover {
    border-color: var(--color-warm-cream);
  }
  .card-label {
    font-size: 11px;
    letter-spacing: 0.14em;
    color: var(--color-ember-accent);
    margin-bottom: 8px;
    display: block;
  }
  .answer-text {
    font-size: 15px;
    line-height: 1.6;
    color: var(--color-warm-cream);
  }
  .deck-actions {
    display: flex;
    justify-content: space-between;
    gap: var(--spacing-12);
  }
  .empty-flashcard-box {
    padding: var(--spacing-18);
    text-align: center;
    border: 1px dashed var(--color-cork-border);
    border-radius: 6px;
  }
  .empty-text {
    font-size: 13px;
    color: var(--color-driftwood);
  }
  @media (max-width: 480px) {
    .flashcard-deck {
      padding: var(--spacing-14);
    }
    .deck-actions button {
      flex: 1;
      padding: 10px 12px;
      text-align: center;
    }
    .flashcard-surface {
      padding: var(--spacing-14);
    }
  }
</style>
