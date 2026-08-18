export interface FlashcardItem {
  id: string;
  prompt: string;
  answer: string;
  topic_tag?: string;
  difficulty?: 'easy' | 'medium' | 'hard';
  mastered?: boolean;
}

export function getStoredDeck(sessionId: string): FlashcardItem[] | null {
  if (typeof localStorage === 'undefined' || !sessionId) return null;
  try {
    const raw = localStorage.getItem(`klasync_flashcards_${sessionId}`);
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}

export function saveStoredDeck(sessionId: string, deck: FlashcardItem[]): void {
  if (typeof localStorage === 'undefined' || !sessionId) return;
  try {
    localStorage.setItem(
      `klasync_flashcards_${sessionId}`,
      JSON.stringify(deck)
    );
  } catch {}
}
