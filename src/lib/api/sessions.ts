import { http } from "./http";
import type { ApiParticipant, ApiSession } from "../api";

export type CreateSessionInput = {
  course_id: string; title: string; allow_late_join?: boolean; guest_expiry_minutes?: number;
};

export function createSession(input: CreateSessionInput) {
  return http<{ session: ApiSession; join_url: string; qr_payload: string }>('/sessions', {
    method: 'POST', body: JSON.stringify(input)
  });
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

export function endSession(code: string) {
  return http<ApiSession>(`/sessions/code/${encodeURIComponent(code)}/end`, { method: 'POST' });
}

export function sendHeartbeat(participantId: string) {
  return http<ApiParticipant>(`/participants/${encodeURIComponent(participantId)}/heartbeat`, { method: 'POST' });
}
