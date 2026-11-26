import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('$lib/api/aiStudy', () => ({
  fetchSessionFlashcards: vi.fn(),
  generateSessionFlashcards: vi.fn(),
  runStudyJob: vi.fn(),
}));

import * as api from '$lib/api/aiStudy';
import { FlashcardDeckState, shuffled } from './flashcardDeck.svelte';

const card = (id: string, n = 1) => ({
  id,
  prompt: `Q${id}`,
  answer: `A${id}`,
  topic_tag: 't',
  difficulty: 'easy' as const,
  ...(n ? {} : {}),
});
const three = [card('1'), card('2'), card('3')];

async function ready(cards = three) {
  vi.mocked(api.fetchSessionFlashcards).mockResolvedValue(cards);
  const deck = new FlashcardDeckState('sess');
  await deck.load();
  return deck;
}

beforeEach(() => {
  localStorage.clear();
  vi.clearAllMocks();
});

describe('shuffled', () => {
  it('returns a permutation without mutating the input', () => {
    const input = [1, 2, 3, 4, 5, 6];
    const out = shuffled(input);
    expect(input).toEqual([1, 2, 3, 4, 5, 6]);
    expect([...out].sort()).toEqual(input);
  });
  it('uses the supplied random source (deterministic)', () => {
    const seq = [0.1, 0.9, 0.5, 0.3];
    let i = 0;
    const rnd = () => seq[i++ % seq.length];
    const a = shuffled([1, 2, 3, 4, 5], rnd);
    i = 0;
    const b = shuffled([1, 2, 3, 4, 5], rnd);
    expect(a).toEqual(b);
  });
  it('can produce every ordering of three items (not biased to few)', () => {
    const seen = new Set<string>();
    for (let a = 0; a < 3; a++)
      for (let b = 0; b < 2; b++) {
        const vals = [a / 3 + 0.01, b / 2 + 0.01];
        let i = 0;
        seen.add(shuffled([1, 2, 3], () => vals[i++]).join(''));
      }
    expect(seen.size).toBe(6);
  });
  it('handles empty and single arrays', () => {
    expect(shuffled([])).toEqual([]);
    expect(shuffled([7])).toEqual([7]);
  });
});

describe('load', () => {
  it('loads cards and persists them', async () => {
    const deck = await ready();
    expect(deck.status).toBe('ready');
    expect(deck.deck.map((c) => c.id)).toEqual(['1', '2', '3']);
    expect(localStorage.getItem('klasync_flashcards_sess')).toContain(
      '"id":"1"'
    );
  });

  it('keeps mastery for cards that are loaded again', async () => {
    const first = await ready();
    first.rate(true);
    const second = new FlashcardDeckState('sess');
    await second.load();
    expect(second.deck.find((c) => c.id === '1')?.mastered).toBe(true);
  });

  it('falls back to the stored deck when the API fails, and says why', async () => {
    const first = await ready();
    first.rate(true);
    vi.mocked(api.fetchSessionFlashcards).mockRejectedValue(
      new Error('offline')
    );
    const again = new FlashcardDeckState('sess');
    await again.load();
    expect(again.error).toBe('offline');
    expect(again.deck.length).toBe(3);
    expect(again.status).toBe('ready');
  });

  it('is empty with no session id', async () => {
    const d = new FlashcardDeckState('');
    await d.load();
    expect(d.deck).toEqual([]);
    expect(api.fetchSessionFlashcards).not.toHaveBeenCalled();
  });
});

describe('studying', () => {
  it('flips only when there is a current card', async () => {
    const empty = await ready([]);
    empty.flip();
    expect(empty.flipped).toBe(false);
    const deck = await ready();
    deck.flip();
    expect(deck.flipped).toBe(true);
  });

  it('moves with wraparound and resets the flip', async () => {
    const deck = await ready();
    deck.flip();
    deck.go(1);
    expect(deck.index).toBe(1);
    expect(deck.flipped).toBe(false);
    deck.go(-2);
    expect(deck.index).toBe(2);
    deck.go(1);
    expect(deck.index).toBe(0);
  });

  it('"Got it" marks mastery and advances', async () => {
    const deck = await ready();
    deck.rate(true);
    expect(deck.deck[0].mastered).toBe(true);
    expect(deck.index).toBe(1);
    expect(deck.masteredCount).toBe(1);
  });

  it('"Still learning" clears mastery and advances', async () => {
    const deck = await ready();
    deck.rate(true);
    deck.go(-1);
    deck.rate(false);
    expect(deck.deck[0].mastered).toBe(false);
  });

  it('in the To-learn filter a mastered card leaves the list without skipping the next', async () => {
    const deck = await ready();
    deck.setFilter('learning');
    expect(deck.visible.map((c) => c.id)).toEqual(['1', '2', '3']);
    deck.rate(true); // removes card 1; card 2 slides into index 0
    expect(deck.visible.map((c) => c.id)).toEqual(['2', '3']);
    expect(deck.index).toBe(0);
    expect(deck.current?.id).toBe('2');
  });

  it('reaches "all mastered" and exposes no current card in the To-learn filter', async () => {
    const deck = await ready();
    deck.setFilter('learning');
    deck.rate(true);
    deck.rate(true);
    deck.rate(true);
    expect(deck.allMastered).toBe(true);
    expect(deck.current).toBeNull();
    deck.setFilter('all');
    expect(deck.current).not.toBeNull();
  });

  it('keeps the index in range when the last visible card is mastered', async () => {
    const deck = await ready();
    deck.setFilter('learning');
    deck.go(2);
    expect(deck.index).toBe(2);
    deck.rate(true);
    expect(deck.index).toBe(1);
    expect(deck.current?.id).toBe('2');
  });

  it('resets progress and the filter', async () => {
    const deck = await ready();
    deck.rate(true);
    deck.setFilter('learning');
    deck.resetProgress();
    expect(deck.masteredCount).toBe(0);
    expect(deck.filter).toBe('all');
    expect(deck.index).toBe(0);
  });

  it('shuffles without losing cards', async () => {
    const deck = await ready();
    deck.shuffle();
    expect(deck.deck.map((c) => c.id).sort()).toEqual(['1', '2', '3']);
    expect(deck.index).toBe(0);
  });
});

describe('generate', () => {
  it('runs the job, replaces the deck and jumps to the first new card', async () => {
    const deck = await ready([card('1')]);
    vi.mocked(api.runStudyJob).mockImplementation(async (start) => {
      await start();
      return undefined as never;
    });
    vi.mocked(api.generateSessionFlashcards).mockResolvedValue({} as never);
    vi.mocked(api.fetchSessionFlashcards).mockResolvedValue([
      card('1'),
      card('2'),
      card('3'),
    ]);
    await deck.generate('Calvin');
    expect(api.generateSessionFlashcards).toHaveBeenCalledWith(
      'sess',
      'Calvin'
    );
    expect(deck.deck.length).toBe(3);
    expect(deck.index).toBe(1);
    expect(deck.generating).toBe(false);
    expect(deck.error).toBe('');
  });

  it('reports a failed job and leaves the deck alone', async () => {
    const deck = await ready();
    vi.mocked(api.runStudyJob).mockRejectedValue(new Error('model busy'));
    await deck.generate();
    expect(deck.error).toBe('model busy');
    expect(deck.deck.length).toBe(3);
    expect(deck.generating).toBe(false);
  });

  it('reports when nothing was produced', async () => {
    const deck = await ready();
    vi.mocked(api.runStudyJob).mockResolvedValue(undefined as never);
    vi.mocked(api.fetchSessionFlashcards).mockResolvedValue([]);
    await deck.generate();
    expect(deck.error).toMatch(/No flashcards were produced/);
  });

  it('ignores a second request while one is running', async () => {
    const deck = await ready();
    let release!: () => void;
    vi.mocked(api.runStudyJob).mockImplementation(
      () => new Promise<never>((r) => (release = () => r(undefined as never)))
    );
    const first = deck.generate();
    await deck.generate();
    expect(api.runStudyJob).toHaveBeenCalledTimes(1);
    release();
    await first;
  });
});
