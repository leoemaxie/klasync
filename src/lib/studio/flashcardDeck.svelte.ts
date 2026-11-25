import {
  fetchSessionFlashcards,
  generateSessionFlashcards,
  runStudyJob,
  type SessionFlashcard,
} from '$lib/api/aiStudy';
import {
  getStoredDeck,
  saveStoredDeck,
  type FlashcardItem,
} from '$lib/utils/flashcards';

export type DeckFilter = 'all' | 'learning';

/** Unbiased in-place-free shuffle (Fisher–Yates). */
export function shuffled<T>(items: T[], random: () => number = Math.random) {
  const out = [...items];
  for (let i = out.length - 1; i > 0; i--) {
    const j = Math.floor(random() * (i + 1));
    [out[i], out[j]] = [out[j], out[i]];
  }
  return out;
}

/**
 * Study state for one lecture's flashcards: load (API, falling back to the
 * locally stored deck), generate, shuffle, filter, and rate cards. Mastery is
 * kept per card id and persisted so it survives regenerating the deck.
 */
export class FlashcardDeckState {
  readonly sessionId: string;

  deck = $state<FlashcardItem[]>([]);
  filter = $state<DeckFilter>('all');
  index = $state(0);
  flipped = $state(false);
  status = $state<'loading' | 'ready'>('loading');
  generating = $state(false);
  error = $state('');

  constructor(sessionId: string) {
    this.sessionId = sessionId;
  }

  visible = $derived(
    this.filter === 'learning'
      ? this.deck.filter((c) => !c.mastered)
      : this.deck
  );
  current = $derived(this.visible[this.index] ?? null);
  masteredCount = $derived(this.deck.filter((c) => c.mastered).length);
  allMastered = $derived(
    this.deck.length > 0 && this.masteredCount === this.deck.length
  );

  #toItems(cards: SessionFlashcard[]): FlashcardItem[] {
    const stored = getStoredDeck(this.sessionId) ?? [];
    const mastered = new Map(stored.map((c) => [c.id, !!c.mastered]));
    return cards.map((c) => ({
      id: c.id,
      prompt: c.prompt,
      answer: c.answer,
      topic_tag: c.topic_tag,
      difficulty: c.difficulty,
      mastered: mastered.get(c.id) ?? false,
    }));
  }

  #persist() {
    saveStoredDeck(this.sessionId, this.deck);
  }

  async load() {
    this.status = 'loading';
    this.error = '';
    try {
      if (!this.sessionId) {
        this.deck = [];
        return;
      }
      try {
        const cards = await fetchSessionFlashcards(this.sessionId);
        if (cards.length) {
          this.deck = this.#toItems(cards);
          this.#persist();
          return;
        }
      } catch (error) {
        this.error =
          error instanceof Error ? error.message : 'Unable to load flashcards.';
      }
      this.deck = getStoredDeck(this.sessionId) ?? [];
    } finally {
      this.index = 0;
      this.flipped = false;
      this.status = 'ready';
    }
  }

  async generate(topic?: string) {
    if (this.generating) return;
    this.generating = true;
    this.error = '';
    const before = this.deck.length;
    try {
      // Waits for the job and surfaces the real failure reason (model busy,
      // no transcript yet, ...) rather than silently doing nothing.
      await runStudyJob(() => generateSessionFlashcards(this.sessionId, topic));
      const cards = await fetchSessionFlashcards(this.sessionId);
      if (cards.length === 0) {
        this.error = 'No flashcards were produced. Please try again.';
        return;
      }
      this.deck = this.#toItems(cards);
      this.#persist();
      this.filter = 'all';
      // Jump to the first new card.
      this.index = Math.min(before, Math.max(this.deck.length - 1, 0));
      this.flipped = false;
    } catch (error) {
      this.error =
        error instanceof Error ? error.message : 'Flashcard generation failed.';
    } finally {
      this.generating = false;
    }
  }

  #clampIndex() {
    this.index = Math.min(this.index, Math.max(this.visible.length - 1, 0));
  }

  setFilter(filter: DeckFilter) {
    this.filter = filter;
    this.index = 0;
    this.flipped = false;
  }

  flip() {
    if (this.current) this.flipped = !this.flipped;
  }

  go(delta: number) {
    const n = this.visible.length;
    if (!n) return;
    this.index = (this.index + delta + n) % n;
    this.flipped = false;
  }

  /** Marks the current card and moves on ("Got it" / "Still learning"). */
  rate(known: boolean) {
    const card = this.current;
    if (!card) return;
    this.deck = this.deck.map((c) =>
      c.id === card.id ? { ...c, mastered: known } : c
    );
    this.#persist();
    this.flipped = false;
    if (this.filter === 'learning' && known) {
      // The card leaves the "to learn" list, so the next one slides into place.
      this.#clampIndex();
    } else {
      this.go(1);
    }
  }

  shuffle() {
    this.deck = shuffled(this.deck);
    this.#persist();
    this.index = 0;
    this.flipped = false;
  }

  resetProgress() {
    this.deck = this.deck.map((c) => ({ ...c, mastered: false }));
    this.#persist();
    this.filter = 'all';
    this.index = 0;
    this.flipped = false;
  }
}
