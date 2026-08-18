import { apiRequest } from './http';
import { resolveSessionUuid } from './sessions';

export interface SessionChapter {
  chapter_index: number;
  title: string;
  summary: string;
  start_timestamp_sec: number;
  end_timestamp_sec: number;
}

export interface SessionFlashcard {
  id: string;
  prompt: string;
  answer: string;
  topic_tag: string;
  difficulty: 'easy' | 'medium' | 'hard';
}

export async function fetchSessionChapters(
  sessionId: string
): Promise<SessionChapter[]> {
  const targetId = await resolveSessionUuid(sessionId);
  if (!targetId) return [];
  return await apiRequest<SessionChapter[]>(
    `/archive/sessions/${encodeURIComponent(targetId)}/chapters`
  ).catch(() => []);
}

export async function fetchSessionFlashcards(
  sessionId: string
): Promise<SessionFlashcard[]> {
  const targetId = await resolveSessionUuid(sessionId);
  if (!targetId) return [];
  return await apiRequest<SessionFlashcard[]>(
    `/archive/sessions/${encodeURIComponent(targetId)}/flashcards`
  ).catch(() => []);
}

export async function generateSessionChapters(
  sessionId: string
): Promise<{ job_id: string; status: string }> {
  const targetId = await resolveSessionUuid(sessionId);
  if (!targetId) {
    throw new Error(`Invalid session identifier: ${sessionId}`);
  }
  return await apiRequest<{ job_id: string; status: string }>(
    `/archive/sessions/${encodeURIComponent(targetId)}/ai/generate-chapters`,
    {
      method: 'POST',
    }
  );
}

export async function generateSessionFlashcards(
  sessionId: string
): Promise<{ job_id: string; status: string }> {
  const targetId = await resolveSessionUuid(sessionId);
  if (!targetId) {
    throw new Error(`Invalid session identifier: ${sessionId}`);
  }
  return await apiRequest<{ job_id: string; status: string }>(
    `/archive/sessions/${encodeURIComponent(targetId)}/ai/generate-flashcards`,
    {
      method: 'POST',
    }
  );
}
