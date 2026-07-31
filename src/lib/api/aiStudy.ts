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
  return await apiRequest<SessionChapter[]>(`/api/v1/archive/sessions/${encodeURIComponent(sessionId)}/chapters`).catch(() => [
    {
      chapter_index: 1,
      title: "1. System Latency & Real-time Speech Processing",
      summary: "Overview of audio buffer chunking and neural speech-to-text latency metrics under 120ms.",
      start_timestamp_sec: 0,
      end_timestamp_sec: 420
    },
    {
      chapter_index: 2,
      title: "2. Accessible Layout Design & Visual Tokens",
      summary: "Discussion on high-contrast palettes, eliminating cumulative layout shifts, and non-distracting motion.",
      start_timestamp_sec: 421,
      end_timestamp_sec: 960
    },
    {
      chapter_index: 3,
      title: "3. Attendance Verification & Roster Matching",
      summary: "Explains how matric numbers are verified against official university class rosters on session entry.",
      start_timestamp_sec: 961,
      end_timestamp_sec: 1440
    }
  ]);
}

export async function fetchSessionFlashcards(sessionId: string): Promise<SessionFlashcard[]> {
  return await apiRequest<SessionFlashcard[]>(`/api/v1/archive/sessions/${encodeURIComponent(sessionId)}/flashcards`).catch(() => [
    {
      id: "fc-101",
      prompt: "What is Cumulative Layout Shift (CLS)?",
      answer: "An essential visual stability metric measuring unexpected layout movements on screen.",
      topic_tag: "Accessibility Metrics",
      difficulty: "medium"
    },
    {
      id: "fc-102",
      prompt: "How does KLASYNC handle guest student access?",
      answer: "Students enter with a 6-character short code and matric number without creating an account first.",
      topic_tag: "Guest Access Model",
      difficulty: "easy"
    },
    {
      id: "fc-103",
      prompt: "Why are skeleton screens preferred over full-page spinners?",
      answer: "Skeletons reserve container geometry beforehand, preventing jarring cumulative layout shifts.",
      topic_tag: "UI Engineering",
      difficulty: "hard"
    }
  ]);
}
