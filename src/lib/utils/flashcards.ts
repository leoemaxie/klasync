export interface FlashcardItem {
  id: string;
  prompt: string;
  answer: string;
  topic_tag?: string;
  difficulty?: 'easy' | 'medium' | 'hard';
  mastered?: boolean;
}

export function getMasteredIds(sessionId: string): Set<string> {
  if (typeof localStorage === 'undefined' || !sessionId) return new Set();
  try {
    const raw = localStorage.getItem(`klasync_flashcards_mastered_${sessionId}`);
    const parsed = raw ? JSON.parse(raw) : [];
    return new Set(Array.isArray(parsed) ? parsed : []);
  } catch {
    return new Set();
  }
}

export function saveMasteredIds(sessionId: string, ids: string[]): void {
  if (typeof localStorage === 'undefined' || !sessionId) return;
  try {
    localStorage.setItem(
      `klasync_flashcards_mastered_${sessionId}`,
      JSON.stringify(ids)
    );
  } catch {}
}

export function clearLegacyCachedFlashcards(sessionId: string): void {
  if (typeof localStorage === 'undefined' || !sessionId) return;
  try {
    localStorage.removeItem(`klasync_flashcards_${sessionId}`);
  } catch {}
}
