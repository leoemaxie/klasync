import { http, getAccessToken } from './http';
import type { SuccessResponse } from './types';

export type ClaimRecord = {
  /** Lecture session UUID (server records) or a local placeholder id. */
  id: string;
  course_code: string;
  course_title?: string;
  academic_session?: string;
  semester?: string;
  session_title: string;
  session_code?: string;
  date: string;
  started_at?: string;
  status?: string;
  /** False when the lecture is only listed because the student is on the roster. */
  attended?: boolean;
  has_audio?: boolean;
};

export type Resource = {
  id: string;
  type: 'transcript' | 'summary' | 'flashcards' | 'audio';
  title: string;
  content?: string;
};
export type ApiResource = Resource;

export type AiJob = {
  id: string;
  job_type: string;
  status: 'pending' | 'processing' | 'completed' | 'failed';
};

export function getLocalStudentClaims(): ClaimRecord[] {
  if (typeof localStorage === 'undefined') return [];
  try {
    const raw = localStorage.getItem('klasync-student-claims');
    const parsed = raw ? JSON.parse(raw) : [];
    if (Array.isArray(parsed) && parsed.length > 0) {
      return parsed;
    }
  } catch {}

  try {
    const sessionRaw = localStorage.getItem('klasync-session');
    if (sessionRaw) {
      const session = JSON.parse(sessionRaw);
      if (session?.code) {
        return [
          {
            id: session.id || session.code,
            session_code: session.code,
            course_code:
              session.course_code ||
              localStorage.getItem('klasync-courseCode') ||
              'COURSE',
            session_title: session.title || 'Live Lecture Session',
            date: new Date().toLocaleDateString('en-US', {
              month: 'short',
              day: 'numeric',
              year: 'numeric',
            }),
          },
        ];
      }
    }
  } catch {}

  return [];
}

export function saveLocalStudentClaim(claim: ClaimRecord): void {
  if (typeof localStorage === 'undefined') return;
  const existing = getLocalStudentClaims();
  const filtered = existing.filter(
    (c) => c.id !== claim.id && c.session_title !== claim.session_title
  );
  const updated = [claim, ...filtered];
  localStorage.setItem('klasync-student-claims', JSON.stringify(updated));
}

export function getArchiveResources(shortCode?: string): Promise<Resource[]> {
  if (!shortCode) return Promise.resolve([]);
  return http<Resource[]>(
    `/sessions/code/${encodeURIComponent(shortCode)}/resources`
  ).catch(() => []);
}

export function claimLecture(participantId: string): Promise<SuccessResponse> {
  return http<SuccessResponse>('/students/claims', {
    method: 'POST',
    body: JSON.stringify({ participant_id: participantId }),
  });
}

const ARCHIVE_CACHE_KEY = 'klasync-student-archive-cache';

function cacheArchive(records: ClaimRecord[]) {
  try {
    localStorage.setItem(ARCHIVE_CACHE_KEY, JSON.stringify(records));
  } catch {}
}

function readArchiveCache(): ClaimRecord[] {
  try {
    const parsed = JSON.parse(localStorage.getItem(ARCHIVE_CACHE_KEY) ?? '[]');
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

/**
 * The server is the source of truth: it matches the signed-in student to every
 * lecture their matric number attended, so every device shows the same list.
 * Device-local claims are only a fallback for guests / being offline; they used
 * to be shown *instead of* an empty server list, which is why each device only
 * ever showed the one session it had joined.
 */
export async function getStudentArchive(): Promise<ClaimRecord[]> {
  const signedIn = typeof localStorage !== 'undefined' && !!getAccessToken();
  if (signedIn) {
    try {
      const remote = await http<ClaimRecord[]>('/students/archive');
      if (Array.isArray(remote)) {
        cacheArchive(remote);
        return remote;
      }
    } catch {
      return readArchiveCache().length
        ? readArchiveCache()
        : getLocalStudentClaims();
    }
  }
  return getLocalStudentClaims();
}

export function getResource(resourceId: string): Promise<Resource> {
  return http<Resource>(`/resources/${encodeURIComponent(resourceId)}`);
}

export function createSessionResource(
  shortCode: string,
  input: { title: string; type: string; content: string }
) {
  return http<Resource>(
    `/sessions/code/${encodeURIComponent(shortCode)}/resources`,
    {
      method: 'POST',
      body: JSON.stringify(input),
    }
  );
}

export function getAiJobs(shortCode: string): Promise<AiJob[]> {
  return http<AiJob[]>(
    `/sessions/code/${encodeURIComponent(shortCode)}/ai-jobs`
  );
}

export function triggerAiJob(
  shortCode: string,
  jobType: string
): Promise<AiJob> {
  return http<AiJob>(
    `/sessions/code/${encodeURIComponent(shortCode)}/ai-jobs`,
    {
      method: 'POST',
      body: JSON.stringify({ job_type: jobType }),
    }
  );
}

export async function uploadSessionResource(
  shortCode: string,
  resourceType: string,
  file: File
): Promise<Resource> {
  const formData = new FormData();
  formData.append('file', file);
  const apiBase =
    import.meta.env.VITE_API_URL ?? 'http://127.0.0.1:8787/api/v1';
  const res = await fetch(
    `${apiBase}/sessions/code/${encodeURIComponent(shortCode)}/resources/${encodeURIComponent(resourceType)}/upload`,
    {
      method: 'POST',
      body: formData,
    }
  );
  if (!res.ok) {
    const payload = await res.json().catch(() => ({}));
    throw new Error(
      payload.error?.replaceAll('_', ' ') ?? 'Resource upload failed'
    );
  }
  return res.json() as Promise<Resource>;
}
