import { http, API_BASE } from './http';
import type { ApiRosterStudent, CountResponse } from './types';
import type { Course, StudentEnrolledCourse } from '$lib/types';

export type { Course, StudentEnrolledCourse };
export type RosterImportReport = { imported_count: number; issues: string[] };

export function getCourses(filters?: {
  academic_session?: string;
  semester?: string;
  is_active?: boolean;
}): Promise<Course[]> {
  const params = new URLSearchParams();
  if (filters?.academic_session)
    params.set('academic_session', filters.academic_session);
  if (filters?.semester) params.set('semester', filters.semester);
  if (filters?.is_active !== undefined)
    params.set('is_active', filters.is_active.toString());

  const qs = params.toString();
  const url = `/courses${qs ? `?${qs}` : ''}`;

  return http<Course[] | { courses: Course[] }>(url).then((res) => {
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
  academic_session: string;
  semester: string;
}): Promise<Course> {
  return http<Course>('/courses', {
    method: 'POST',
    body: JSON.stringify({
      code: input.code.trim(),
      title: input.title.trim(),
      academic_session: input.academic_session.trim(),
      semester: input.semester.trim(),
    }),
  });
}

export function getStudentCourses(): Promise<StudentEnrolledCourse[]> {
  return http<StudentEnrolledCourse[] | { courses: StudentEnrolledCourse[] }>(
    '/students/courses'
  ).then((res) => {
    if (Array.isArray(res)) return res;
    if (
      res &&
      Array.isArray((res as { courses: StudentEnrolledCourse[] }).courses)
    ) {
      return (res as { courses: StudentEnrolledCourse[] }).courses;
    }
    return [];
  });
}

export function enrollStudentCourse(courseId: string): Promise<void> {
  return http<void>('/students/courses/enroll', {
    method: 'POST',
    body: JSON.stringify({ course_id: courseId }),
  });
}

const UUID_REGEX =
  /^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$/;

export async function resolveCourseUuid(
  codeOrId: string,
  title?: string,
  academic_session: string = '2025/2026',
  semester: string = 'Second Semester'
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
      (c) =>
        (c.code.toLowerCase() === trimmed.toLowerCase() || c.id === trimmed) &&
        (!academic_session || c.academic_session === academic_session) &&
        (!semester || c.semester === semester)
    ) || courses.find(
      (c) => c.code.toLowerCase() === trimmed.toLowerCase() || c.id === trimmed
    );
    if (matched?.id) return matched.id;
  } catch {}

  // 2. Create course if not found
  try {
    const course = await createCourse({
      code: trimmed,
      title: title?.trim() || trimmed,
      academic_session,
      semester,
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
  const response = await fetch(
    `${API_BASE}/courses/${encodeURIComponent(targetId)}/roster/import`,
    {
      method: 'POST',
      credentials: 'include',
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
