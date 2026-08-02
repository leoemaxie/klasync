import { apiRequest } from "./http";

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
  difficulty: "easy" | "medium" | "hard";
}

export async function fetchSessionChapters(sessionId: string): Promise<SessionChapter[]> {
  return await apiRequest<SessionChapter[]>(`/archive/sessions/${encodeURIComponent(sessionId)}/chapters`).catch(() => []);
}

export async function fetchSessionFlashcards(sessionId: string): Promise<SessionFlashcard[]> {
  return await apiRequest<SessionFlashcard[]>(`/archive/sessions/${encodeURIComponent(sessionId)}/flashcards`).catch(() => []);
}

export async function generateSessionChapters(sessionId: string): Promise<{ job_id: string; status: string }> {
  return await apiRequest<{ job_id: string; status: string }>(`/archive/sessions/${encodeURIComponent(sessionId)}/ai/generate-chapters`, {
    method: 'POST'
  });
}

export async function generateSessionFlashcards(sessionId: string): Promise<{ job_id: string; status: string }> {
  return await apiRequest<{ job_id: string; status: string }>(`/archive/sessions/${encodeURIComponent(sessionId)}/ai/generate-flashcards`, {
    method: 'POST'
  });
}


