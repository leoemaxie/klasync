export interface FlashcardItem {
  id: string;
  prompt: string;
  answer: string;
  topic_tag?: string;
  difficulty?: 'easy' | 'medium' | 'hard';
  mastered?: boolean;
}

export function extractDynamicFlashcards(
  transcript?: string,
  topic?: string
): FlashcardItem[] {
  if (topic && topic.trim()) {
    const t = topic.trim();
    const matching = transcript
      ?.split('\n')
      .find((l) => l.toLowerCase().includes(t.toLowerCase()))
      ?.replace(/^\[.*?\]\s*/, '');

    return [
      {
        id: `gen-${Date.now()}-1`,
        prompt: `What is the core definition of "${t}"?`,
        answer: matching
          ? `From lecture transcript: "${matching}"`
          : `"${t}" represents a key principle discussed in this session with direct application to system architecture and assessments.`,
        topic_tag: t,
        difficulty: 'medium',
        mastered: false,
      },
      {
        id: `gen-${Date.now()}-2`,
        prompt: `How is "${t}" verified and assessed?`,
        answer: `Assessment of "${t}" relies on analytical recall, active problem-solving, and accurate contextual application.`,
        topic_tag: t,
        difficulty: 'hard',
        mastered: false,
      },
    ];
  }

  const lines = (transcript || '')
    .split('\n')
    .map((l) => l.replace(/^\[\d+:\d+(?::\d+)?\]\s*/, '').trim())
    .filter((l) => l.length > 20);

  if (lines.length > 0) {
    return lines.slice(0, 8).map((line, idx) => {
      const words = line.split(' ');
      const subject = words.slice(0, 4).join(' ');
      return {
        id: `dyn-${idx}-${Date.now()}`,
        prompt: `Key Concept: What was highlighted regarding "${subject}..."?`,
        answer: line,
        topic_tag: `Topic ${idx + 1}`,
        difficulty: (idx % 3 === 0
          ? 'easy'
          : idx % 3 === 1
            ? 'medium'
            : 'hard') as 'easy' | 'medium' | 'hard',
        mastered: false,
      };
    });
  }

  return [];
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
