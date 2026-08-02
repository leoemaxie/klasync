import { http } from "./http";
import type { SuccessResponse } from "./types";

export type ClaimRecord = { id: string; course_code: string; session_title: string; date: string };

export type Resource = { id: string; type: 'transcript' | 'summary' | 'flashcards' | 'audio'; title: string; content?: string };
export type ApiResource = Resource;

export type AiJob = { id: string; job_type: string; status: 'pending' | 'processing' | 'completed' | 'failed' };

export function getArchiveResources(shortCode?: string): Promise<Resource[]> {
  const path = shortCode ? `/sessions/code/${encodeURIComponent(shortCode)}/resources` : '/resources';
  return http<Resource[]>(path);
}

export function claimLecture(participantId: string): Promise<SuccessResponse> {
  return http<SuccessResponse>('/students/claims', {
    method: 'POST', body: JSON.stringify({ participant_id: participantId })
  });
}

export function getStudentArchive(): Promise<ClaimRecord[]> {
  return http<ClaimRecord[]>('/students/archive');
}

export function getResource(resourceId: string): Promise<Resource> {
  return http<Resource>(`/resources/${encodeURIComponent(resourceId)}`);
}

export function createSessionResource(shortCode: string, input: { title: string; type: string; content: string }) {
  return http<Resource>(`/sessions/code/${encodeURIComponent(shortCode)}/resources`, {
    method: 'POST', body: JSON.stringify(input)
  });
}

export function getAiJobs(shortCode: string): Promise<AiJob[]> {
  return http<AiJob[]>(`/sessions/code/${encodeURIComponent(shortCode)}/ai-jobs`);
}

export function triggerAiJob(shortCode: string, jobType: string): Promise<AiJob> {
  return http<AiJob>(`/sessions/code/${encodeURIComponent(shortCode)}/ai-jobs`, {
    method: 'POST', body: JSON.stringify({ job_type: jobType })
  });
}

export async function uploadSessionResource(shortCode: string, resourceType: string, file: File): Promise<Resource> {
  const formData = new FormData();
  formData.append("file", file);
  const apiBase = import.meta.env.VITE_API_URL ?? 'http://127.0.0.1:8787/api/v1';
  const res = await fetch(`${apiBase}/sessions/code/${encodeURIComponent(shortCode)}/resources/${encodeURIComponent(resourceType)}/upload`, {
    method: 'POST',
    body: formData
  });
  if (!res.ok) {
    const payload = await res.json().catch(() => ({}));
    throw new Error(payload.error?.replaceAll('_', ' ') ?? 'Resource upload failed');
  }
  return res.json() as Promise<Resource>;
}
