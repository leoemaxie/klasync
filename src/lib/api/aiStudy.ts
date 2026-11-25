import { apiRequest, httpBlob } from './http';
import { resolveSessionUuid } from './sessions';

export interface SessionChapter {
  id?: string;
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

export interface TranscriptLine {
  offset_sec: number | null;
  text: string;
}

export interface TranscriptResponse {
  session_id: string;
  source: 'captions' | 'transcript' | 'none';
  duration_sec: number;
  lines: TranscriptLine[];
}

export interface RollingSummary {
  id: string;
  sequence: number;
  summary: string;
  key_points: string[];
  source: 'ai' | 'heuristic';
  created_at: string;
}

export interface RecordingInfo {
  resource_id: string;
  content_type: string | null;
  byte_size: number | null;
  duration_sec: number | null;
  created_at: string;
}

export interface ConceptNode {
  id: string;
  label: string;
  group: string;
  importance: number;
}
export interface ConceptEdge {
  from: string;
  to: string;
  label: string;
}
export interface SessionVisualization {
  id: string;
  kind: 'concept_map' | 'timeline';
  title: string;
  spec: { title?: string; nodes: ConceptNode[]; edges: ConceptEdge[] };
  source: 'ai' | 'heuristic';
  created_at: string;
}

export interface StudyJob {
  job_id: string;
  status: 'ready' | 'processing' | string;
}

export interface JobStatus {
  id: string;
  job_type: string;
  status: 'queued' | 'running' | 'completed' | 'failed' | 'cancelled';
  error_message: string | null;
  attempts: number;
}

/** Resolve whatever identifier the UI has (UUID or short code) to a UUID. */
async function sessionUuid(sessionId: string): Promise<string> {
  const id = await resolveSessionUuid(sessionId);
  if (!id) throw new Error(`Invalid session identifier: ${sessionId}`);
  return id;
}

const base = (id: string) => `/archive/sessions/${encodeURIComponent(id)}`;

// Reads throw on failure so the UI can show *why* (e.g. access denied) instead
// of an indistinguishable empty list.
export async function fetchSessionChapters(
  sessionId: string
): Promise<SessionChapter[]> {
  const id = await sessionUuid(sessionId);
  return apiRequest<SessionChapter[]>(`${base(id)}/chapters`);
}

export async function fetchSessionFlashcards(
  sessionId: string
): Promise<SessionFlashcard[]> {
  const id = await sessionUuid(sessionId);
  return apiRequest<SessionFlashcard[]>(`${base(id)}/flashcards`);
}

export async function fetchSessionVisualizations(
  sessionId: string
): Promise<SessionVisualization[]> {
  const id = await sessionUuid(sessionId);
  return apiRequest<SessionVisualization[]>(`${base(id)}/visualizations`);
}

export async function fetchSessionTranscript(
  sessionId: string
): Promise<TranscriptResponse> {
  const id = await sessionUuid(sessionId);
  return apiRequest<TranscriptResponse>(`${base(id)}/transcript`);
}

export async function fetchArchiveSummaries(
  sessionId: string
): Promise<RollingSummary[]> {
  const id = await sessionUuid(sessionId);
  return apiRequest<RollingSummary[]>(`${base(id)}/summaries`);
}

export async function fetchRecordings(
  sessionId: string
): Promise<RecordingInfo[]> {
  const id = await sessionUuid(sessionId);
  return apiRequest<RecordingInfo[]>(`${base(id)}/recordings`);
}

export async function fetchRecordingBlob(
  sessionId: string,
  resourceId: string
): Promise<Blob> {
  const id = await sessionUuid(sessionId);
  return httpBlob(
    `${base(id)}/recordings/${encodeURIComponent(resourceId)}/stream`
  );
}

async function generate(
  sessionId: string,
  kind: 'chapters' | 'flashcards' | 'visualization',
  topic?: string
): Promise<StudyJob> {
  const id = await sessionUuid(sessionId);
  return apiRequest<StudyJob>(`${base(id)}/ai/generate-${kind}`, {
    method: 'POST',
    body: JSON.stringify(topic ? { topic } : {}),
  });
}

export const generateSessionChapters = (sessionId: string) =>
  generate(sessionId, 'chapters');
export const generateSessionFlashcards = (sessionId: string, topic?: string) =>
  generate(sessionId, 'flashcards', topic);
export const generateSessionVisualization = (
  sessionId: string,
  topic?: string
) => generate(sessionId, 'visualization', topic);

export function fetchJobStatus(jobId: string): Promise<JobStatus> {
  return apiRequest<JobStatus>(`/archive/jobs/${encodeURIComponent(jobId)}`);
}

/**
 * Starts a generation job and waits for it. Resolves when the job completed or
 * the content already existed; throws with the server-side failure reason when
 * it failed, or a timeout message if the model is just slow.
 */
export async function runStudyJob(
  start: () => Promise<StudyJob>,
  { timeoutMs = 150_000, intervalMs = 2_500 } = {}
): Promise<void> {
  const job = await start();
  if (job.status === 'ready') return;
  const deadline = Date.now() + timeoutMs;
  let lastFailure = '';
  while (Date.now() < deadline) {
    await new Promise((resolve) => setTimeout(resolve, intervalMs));
    const status = await fetchJobStatus(job.job_id).catch(() => null);
    if (!status) continue;
    if (status.status === 'completed') return;
    if (status.status === 'cancelled')
      throw new Error('Generation was cancelled.');
    if (status.status === 'failed') {
      lastFailure = status.error_message ?? 'The AI model could not finish.';
      // The worker retries failed jobs automatically; only give up once it
      // has used all its attempts.
      if (status.attempts >= 3) throw new Error(lastFailure);
    }
  }
  throw new Error(
    lastFailure ||
      'This is taking longer than expected. The AI model may be busy; try again in a minute.'
  );
}

export interface AskSource {
  start_sec: number;
  end_sec: number;
  text: string;
  score: number;
}
export interface AskResponse {
  answer: string;
  sources: AskSource[];
}
export interface SearchHit {
  session_id: string;
  session_title: string;
  course_code: string;
  start_sec: number;
  end_sec: number;
  text: string;
  score: number;
}

/** Answers a question from the lecture's own passages (semantic retrieval). */
export async function askLecture(
  sessionId: string,
  question: string
): Promise<AskResponse> {
  const id = await sessionUuid(sessionId);
  return apiRequest<AskResponse>(`${base(id)}/ask`, {
    method: 'POST',
    body: JSON.stringify({ question }),
  });
}

/** Meaning-based search across every lecture the student can open. */
export function searchLectures(query: string): Promise<SearchHit[]> {
  return apiRequest<SearchHit[]>(
    `/students/search?q=${encodeURIComponent(query)}`
  );
}
