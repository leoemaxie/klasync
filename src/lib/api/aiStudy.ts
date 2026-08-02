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
  return await apiRequest<SessionChapter[]>(`/api/v1/archive/sessions/${encodeURIComponent(sessionId)}/chapters`).catch(() => []);
}

export async function fetchSessionFlashcards(sessionId: string): Promise<SessionFlashcard[]> {
  return await apiRequest<SessionFlashcard[]>(`/api/v1/archive/sessions/${encodeURIComponent(sessionId)}/flashcards`).catch(() => []);
}

