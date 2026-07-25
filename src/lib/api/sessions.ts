import { http } from "./http";
import type { ApiParticipant, ApiRosterStudent, ApiSession } from "./types";

export type CreateSessionInput = {
  course_id: string; title: string; allow_late_join?: boolean; guest_expiry_minutes?: number;
};

export function createSession(input: CreateSessionInput) {
  return http<{ session: ApiSession; join_url: string; qr_payload: string }>('/sessions', {
    method: 'POST', body: JSON.stringify(input)
  });
}

export async function createLiveSession(input: {
  lecturerName: string; lecturerEmail: string; courseCode: string; courseTitle: string; roster: ApiRosterStudent[];
}) {
  const course = await http<{ id: string }>('/courses', {
    method: 'POST', body: JSON.stringify({ code: input.courseCode, title: input.courseTitle })
  });
  if (input.roster.length) {
    await http(`/courses/${course.id}/roster`, { method: 'POST', body: JSON.stringify({ students: input.roster }) });
  }
  return createSession({ course_id: course.id, title: `${input.courseCode}: ${input.courseTitle}` });
}

export function lookupSessionByCode(code: string) {
  return http<{ session: ApiSession; participant_count: number }>(`/sessions/code/${encodeURIComponent(code)}`);
}

export function joinSessionByCode(code: string, matric: string, name?: string) {
  return http<ApiParticipant>(`/sessions/code/${encodeURIComponent(code)}/join`, {
    method: 'POST', body: JSON.stringify({ matric_number: matric, display_name: name || undefined })
  });
}

export function getParticipants(code: string) {
  return http<ApiParticipant[]>(`/sessions/code/${encodeURIComponent(code)}/participants`);
}

export function getAttendanceSummary(code: string) {
  return http<{ total: number; verified: number; provisional: number }>(`/sessions/code/${encodeURIComponent(code)}/attendance`);
}

export function reviewParticipantAttendance(code: string, participantId: string, status: 'verified' | 'rejected') {
  return http<ApiParticipant>(`/sessions/code/${encodeURIComponent(code)}/participants/${encodeURIComponent(participantId)}/review`, {
    method: 'POST', body: JSON.stringify({ status })
  });
}

export function resolveInviteToken(token: string) {
  return http<{ session: ApiSession; course_code: string }>(`/invites/${encodeURIComponent(token)}`);
}

export function revokeInvite(code: string) {
  return http<{ revoked: boolean }>(`/sessions/code/${encodeURIComponent(code)}/invite/revoke`, { method: 'POST' });
}

export function getQrSvgUrl(code: string): string {
  const base = import.meta.env.VITE_API_URL ?? 'http://127.0.0.1:8787/api/v1';
  return `${base}/sessions/code/${encodeURIComponent(code)}/invite/qr.svg`;
}

export function getAttendanceCsvUrl(code: string): string {
  const base = import.meta.env.VITE_API_URL ?? 'http://127.0.0.1:8787/api/v1';
  return `${base}/sessions/code/${encodeURIComponent(code)}/attendance.csv`;
}

export function endSession(code: string) {
  return http<ApiSession>(`/sessions/code/${encodeURIComponent(code)}/end`, { method: 'POST' });
}

export function sendHeartbeat(participantId: string) {
  return http<ApiParticipant>(`/participants/${encodeURIComponent(participantId)}/heartbeat`, { method: 'POST' });
}
