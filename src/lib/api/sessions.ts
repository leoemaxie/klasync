import { http } from './http';
import type {
  ApiParticipant,
  ApiRosterStudent,
  ApiSession,
  AttendanceSummaryResponse,
  CreateCourseResponse,
  CreateSessionResponse,
  InviteLookupResponse,
  RevokeInviteResponse,
  SessionLookupResponse,
} from './types';

export type CreateSessionInput = {
  course_id: string;
  title: string;
  allow_late_join?: boolean;
  guest_expiry_minutes?: number;
};

export function createSession(input: CreateSessionInput) {
  return http<CreateSessionResponse>('/sessions', {
    method: 'POST',
    body: JSON.stringify(input),
  });
}

export async function createLiveSession(input: {
  lecturerName: string;
  lecturerEmail: string;
  courseCode: string;
  courseTitle: string;
  academic_session?: string;
  semester?: string;
  course_id?: string;
  roster: ApiRosterStudent[];
}) {
  let courseId = input.course_id;
  if (!courseId) {
    const course = await http<CreateCourseResponse>('/courses', {
      method: 'POST',
      body: JSON.stringify({
        code: input.courseCode,
        title: input.courseTitle,
        academic_session: input.academic_session || '2025/2026',
        semester: input.semester || 'Second Semester',
      }),
    });
    courseId = course.id;
  }

  if (input.roster.length && courseId) {
    await http(`/courses/${courseId}/roster`, {
      method: 'POST',
      body: JSON.stringify({ students: input.roster }),
    });
  }
  return createSession({
    course_id: courseId,
    title: `${input.courseCode}: ${input.courseTitle}`,
  });
}

export function lookupSessionByCode(code: string) {
  return http<SessionLookupResponse>(
    `/sessions/code/${encodeURIComponent(code)}`
  );
}

export function joinSessionByCode(code: string, matric: string, name?: string) {
  return http<ApiParticipant>(
    `/sessions/code/${encodeURIComponent(code)}/join`,
    {
      method: 'POST',
      body: JSON.stringify({
        matric_number: matric,
        display_name: name || undefined,
      }),
    }
  );
}

export function getParticipants(code: string) {
  return http<ApiParticipant[]>(
    `/sessions/code/${encodeURIComponent(code)}/participants`
  );
}

export function getAttendanceSummary(code: string) {
  return http<AttendanceSummaryResponse>(
    `/sessions/code/${encodeURIComponent(code)}/attendance`
  );
}

export function reviewParticipantAttendance(
  code: string,
  participantId: string,
  status: 'verified' | 'rejected'
) {
  const decision = status === 'verified' ? 'approved' : 'rejected';
  return http<ApiParticipant>(
    `/sessions/code/${encodeURIComponent(code)}/participants/${encodeURIComponent(participantId)}/review`,
    {
      method: 'POST',
      body: JSON.stringify({ decision }),
    }
  );
}

export function resolveInviteToken(token: string) {
  return http<InviteLookupResponse>(`/invites/${encodeURIComponent(token)}`);
}

export function revokeInvite(code: string) {
  return http<RevokeInviteResponse>(
    `/sessions/code/${encodeURIComponent(code)}/invite/revoke`,
    { method: 'POST' }
  );
}

export function getQrSvgUrl(code: string): string {
  const apiBase =
    import.meta.env.VITE_API_URL ?? 'http://127.0.0.1:8787/api/v1';
  return `${apiBase}/sessions/code/${encodeURIComponent(code)}/invite/qr.svg`;
}

export function getAttendanceCsvUrl(code: string): string {
  const apiBase =
    import.meta.env.VITE_API_URL ?? 'http://127.0.0.1:8787/api/v1';
  return `${apiBase}/sessions/code/${encodeURIComponent(code)}/attendance.csv`;
}

export function endSession(code: string) {
  return http<ApiSession>(`/sessions/code/${encodeURIComponent(code)}/end`, {
    method: 'POST',
  });
}

export function sendHeartbeat(participantId: string) {
  return http<ApiParticipant>(
    `/participants/${encodeURIComponent(participantId)}/heartbeat`,
    { method: 'POST' }
  );
}

const UUID_REGEX =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export async function resolveSessionUuid(
  sessionIdOrCode: string
): Promise<string | null> {
  const trimmed = sessionIdOrCode?.trim();
  if (!trimmed) return null;
  if (UUID_REGEX.test(trimmed)) {
    return trimmed;
  }

  // 1. Check local session storage if available
  if (typeof localStorage !== 'undefined') {
    try {
      const storedRaw = localStorage.getItem('klasync-session');
      if (storedRaw) {
        const stored = JSON.parse(storedRaw);
        if (
          (stored?.code?.toUpperCase() === trimmed.toUpperCase() ||
            stored?.id === trimmed) &&
          stored?.id &&
          UUID_REGEX.test(stored.id)
        ) {
          return stored.id;
        }
      }
    } catch {}
  }

  // 2. Query backend by session code to resolve its UUID
  try {
    const detail = await lookupSessionByCode(trimmed);
    if (detail?.session?.id) {
      return detail.session.id;
    }
  } catch {}

  return null;
}
