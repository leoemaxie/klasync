const apiBase = import.meta.env.VITE_API_URL ?? 'http://127.0.0.1:8787/api/v1';

type ApiError = { error?: string };

async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const response = await fetch(`${apiBase}${path}`, {
    ...init,
    headers: { 'content-type': 'application/json', ...(init.headers ?? {}) }
  });
  if (!response.ok) {
    const payload = await response.json().catch(() => ({})) as ApiError;
    throw new Error(payload.error?.replaceAll('_', ' ') ?? 'The KLASYNC service is unavailable.');
  }
  return response.json() as Promise<T>;
}

export type ApiRosterStudent = { matric_number: string; full_name: string; email?: string | null };
export type ApiSession = { id: string; title: string; short_code: string; status: 'live' | 'ended' };
export type ApiParticipant = {
  id: string; matric_number: string; display_name: string;
  verification_status: 'verified' | 'provisional'; heartbeat_count: number;
};
export type ApiCaption = { id: string; text: string; created_at: string };

export async function createLiveSession(input: {
  lecturerName: string; lecturerEmail: string; courseCode: string; courseTitle: string; roster: ApiRosterStudent[];
}) {
  const lecturer = await request<{ id: string }>('/lecturers/register', {
    method: 'POST', body: JSON.stringify({ name: input.lecturerName, email: input.lecturerEmail })
  });
  const course = await request<{ id: string }>('/courses', {
    method: 'POST', body: JSON.stringify({ lecturer_id: lecturer.id, code: input.courseCode, title: input.courseTitle })
  });
  if (input.roster.length) {
    await request(`/courses/${course.id}/roster`, { method: 'POST', body: JSON.stringify({ students: input.roster }) });
  }
  return request<{ session: ApiSession; join_url: string; qr_payload: string }>('/sessions', {
    method: 'POST', body: JSON.stringify({ course_id: course.id, title: `${input.courseCode}: ${input.courseTitle}` })
  });
}

export function joinLiveSession(shortCode: string, matricNumber: string, displayName: string) {
  return request<ApiParticipant>(`/sessions/code/${encodeURIComponent(shortCode)}/join`, {
    method: 'POST', body: JSON.stringify({ matric_number: matricNumber, display_name: displayName || undefined })
  });
}

export function resolveLiveSession(shortCode: string) {
  return request<{ session: ApiSession; participant_count: number }>(`/sessions/code/${encodeURIComponent(shortCode)}`);
}

export function getSessionParticipants(shortCode: string) {
  return request<ApiParticipant[]>(`/sessions/code/${encodeURIComponent(shortCode)}/participants`);
}

export function endLiveSession(shortCode: string) {
  return request<ApiSession>(`/sessions/code/${encodeURIComponent(shortCode)}/end`, { method: 'POST' });
}

export function getLiveCaptions(shortCode: string) {
  return request<ApiCaption[]>(`/sessions/code/${encodeURIComponent(shortCode)}/captions`);
}

export function publishLiveCaption(shortCode: string, text: string) {
  return request<ApiCaption>(`/sessions/code/${encodeURIComponent(shortCode)}/captions`, {
    method: 'POST', body: JSON.stringify({ text })
  });
}

export function sendHeartbeat(participantId: string) {
  return request<ApiParticipant>(`/participants/${encodeURIComponent(participantId)}/heartbeat`, { method: 'POST' });
}
