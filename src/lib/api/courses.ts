import { http } from './http';
import type { ApiRosterStudent, CountResponse } from './types';

export type Course = {
  id: string;
  code: string;
  title: string;
  roster_count?: number;
  last_session_at?: string;
};

export type RosterImportReport = { imported_count: number; issues: string[] };

export function getCourses(): Promise<Course[]> {
  return http<Course[] | { courses: Course[] }>('/courses').then((res) => {
    if (Array.isArray(res)) return res;
    if (res && Array.isArray((res as { courses: Course[] }).courses)) {
      return (res as { courses: Course[] }).courses;
    }
    return [];
  });
}

export function getCourseDetail(courseId: string): Promise<Course> {
  return http<Course>(`/courses/${encodeURIComponent(courseId)}`);
}

export function createCourse(input: {
  code: string;
  title: string;
}): Promise<Course> {
  return http<Course>('/courses', {
    method: 'POST',
    body: JSON.stringify(input),
  });
}

const UUID_REGEX =
  /^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$/;

export async function resolveCourseUuid(
  codeOrId: string,
  title?: string
): Promise<string> {
  const trimmed = codeOrId.trim();
  if (!trimmed) return trimmed;
  if (UUID_REGEX.test(trimmed)) {
    return trimmed;
  }

  // 1. Check existing courses list first
  try {
    const courses = await getCourses();
    const matched = courses.find(
      (c) => c.code.toLowerCase() === trimmed.toLowerCase() || c.id === trimmed
    );
    if (matched?.id) return matched.id;
  } catch {}

  // 2. Create course if not found
  try {
    const course = await createCourse({
      code: trimmed,
      title: title?.trim() || trimmed,
    });
    if (course?.id) return course.id;
  } catch {}

  return trimmed;
}

export async function uploadRoster(
  courseIdOrCode: string,
  students: ApiRosterStudent[]
): Promise<CountResponse> {
  const targetId = await resolveCourseUuid(courseIdOrCode);
  return http<CountResponse>(
    `/courses/${encodeURIComponent(targetId)}/roster`,
    {
      method: 'POST',
      body: JSON.stringify({ students }),
    }
  );
}

export async function importRosterFile(
  courseIdOrCode: string,
  file: File
): Promise<RosterImportReport> {
  const targetId = await resolveCourseUuid(courseIdOrCode);
  const formData = new FormData();
  formData.append('file', file);
  const apiBase =
    import.meta.env.VITE_API_URL ?? 'http://127.0.0.1:8787/api/v1';
  const response = await fetch(
    `${apiBase}/courses/${encodeURIComponent(targetId)}/roster/import`,
    {
      method: 'POST',
      body: formData,
    }
  );
  if (!response.ok) {
    const payload = await response.json().catch(() => ({}));
    throw new Error(
      payload.error?.replaceAll('_', ' ') ?? 'Roster file import failed'
    );
  }
  return response.json() as Promise<RosterImportReport>;
}

export async function getCourseRoster(
  courseIdOrCode: string
): Promise<ApiRosterStudent[]> {
  const targetId = await resolveCourseUuid(courseIdOrCode);
  return http<ApiRosterStudent[] | { students: ApiRosterStudent[] }>(
    `/courses/${encodeURIComponent(targetId)}/roster`
  ).then((res) => {
    if (Array.isArray(res)) return res;
    if (
      res &&
      Array.isArray((res as { students: ApiRosterStudent[] }).students)
    ) {
      return (res as { students: ApiRosterStudent[] }).students;
    }
    return [];
  });
}

export const uploadCourseRoster = uploadRoster;
export const uploadCourseRosterFile = importRosterFile;
