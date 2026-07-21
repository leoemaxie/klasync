import { http } from "./http";

export type ClaimRecord = { id: string; course_code: string; session_title: string; date: string };

export type Resource = { id: string; type: 'transcript' | 'summary' | 'flashcards' | 'audio'; title: string; content?: string };

export function claimLecture(participantId: string): Promise<{ success: boolean }> {
  return http<{ success: boolean }>('/students/claims', {
    method: 'POST', body: JSON.stringify({ participant_id: participantId })
  });
}

export function getStudentArchive(): Promise<ClaimRecord[]> {
  return http<ClaimRecord[]>('/students/archive');
}

export function getResource(resourceId: string): Promise<Resource> {
  return http<Resource>(`/resources/${encodeURIComponent(resourceId)}`);
}
