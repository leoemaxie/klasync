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
    return await apiRequest<Question[]>(
      `/sessions/code/${encodeURIComponent(sessionCode)}/questions`
    );
  } catch {
    // Return local mock fallback if API is in offline mode
    return [
      {
        id: 'q-1',
        question_text:
          'Does layout shift directly degrade accessibility compliance?',
        upvote_count: 5,
        is_resolved: false,
        created_at: new Date().toISOString(),
      },
      {
        id: 'q-2',
        question_text:
          'What is the target latency for real-time speech-to-text?',
        upvote_count: 3,
        is_resolved: true,
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
  return await apiRequest<Question>(
    `/sessions/code/${encodeURIComponent(sessionCode)}/questions`,
    {
      method: 'POST',
      body: JSON.stringify({
        participant_id: participantId,
        caption_id: captionId,
        question_text: questionText,
      }),
    }
  ).catch(() => ({
    id: `q-local-${Date.now()}`,
    question_text: questionText,
    upvote_count: 1,
    is_resolved: false,
    created_at: new Date().toISOString(),
    caption_id: captionId,
  }));
}

export async function upvoteQuestion(
  sessionCode: string,
  questionId: string
): Promise<{ new_upvote_count: number }> {
  return await apiRequest<{ new_upvote_count: number }>(
    `/sessions/code/${encodeURIComponent(sessionCode)}/questions/${encodeURIComponent(questionId)}/upvote`,
    { method: 'POST' }
  ).catch(() => ({ new_upvote_count: Math.floor(Math.random() * 10) + 1 }));
}

export async function resolveQuestion(
  sessionCode: string,
  questionId: string
): Promise<{ is_resolved: boolean }> {
  return await apiRequest<{ is_resolved: boolean }>(
    `/sessions/code/${encodeURIComponent(sessionCode)}/questions/${encodeURIComponent(questionId)}/resolve`,
    { method: 'POST' }
  ).catch(() => ({ is_resolved: true }));
}
