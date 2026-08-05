import { apiRequest } from './http';

export interface Question {
  id: string;
  question_text: string;
  upvote_count: number;
  is_resolved: boolean;
  created_at: string;
  caption_id?: string;
}

export async function fetchSessionQuestions(
  sessionCode: string
): Promise<Question[]> {
  try {
    const res = await apiRequest<Question[]>(
      `/sessions/code/${encodeURIComponent(sessionCode)}/questions`
    );
    if (Array.isArray(res)) return res;
    return [];
  } catch {
    // Local fallback if offline
    return [
      {
        id: 'q-1',
        question_text:
          'Does layout shift directly degrade accessibility compliance?',
        upvote_count: 5,
        is_resolved: false,
        created_at: new Date().toISOString(),
      },
    ];
  }
}

export async function submitQuestion(
  sessionCode: string,
  questionText: string,
  participantId?: string,
  captionId?: string
): Promise<Question> {
  const text = questionText.trim();
  try {
    const res = await apiRequest<any>(
      `/sessions/code/${encodeURIComponent(sessionCode)}/questions`,
      {
        method: 'POST',
        body: JSON.stringify({
          participant_id: participantId,
          caption_id: captionId,
          question_text: text,
        }),
      }
    );

    return {
      id: res?.id ? String(res.id) : `q-${Date.now()}`,
      question_text: res?.question_text ?? text,
      upvote_count: typeof res?.upvote_count === 'number' ? res.upvote_count : 0,
      is_resolved: Boolean(res?.is_resolved),
      created_at: res?.created_at ?? new Date().toISOString(),
      caption_id: res?.caption_id ?? captionId,
    };
  } catch {
    return {
      id: `q-local-${Date.now()}`,
      question_text: text,
      upvote_count: 1,
      is_resolved: false,
      created_at: new Date().toISOString(),
      caption_id: captionId,
    };
  }
}

export async function upvoteQuestion(
  sessionCode: string,
  questionId: string
): Promise<{ new_upvote_count: number }> {
  try {
    const res = await apiRequest<any>(
      `/sessions/code/${encodeURIComponent(sessionCode)}/questions/${encodeURIComponent(questionId)}/upvote`,
      { method: 'POST' }
    );
    return {
      new_upvote_count:
        typeof res?.new_upvote_count === 'number'
          ? res.new_upvote_count
          : typeof res?.upvote_count === 'number'
            ? res.upvote_count
            : 1,
    };
  } catch {
    return { new_upvote_count: 1 };
  }
}

export async function resolveQuestion(
  sessionCode: string,
  questionId: string
): Promise<{ is_resolved: boolean }> {
  try {
    const res = await apiRequest<any>(
      `/sessions/code/${encodeURIComponent(sessionCode)}/questions/${encodeURIComponent(questionId)}/resolve`,
      { method: 'POST' }
    );
    return { is_resolved: res?.is_resolved ?? true };
  } catch {
    return { is_resolved: true };
  }
}
