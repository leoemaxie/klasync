import { http } from "./http";
import type { ApiRosterStudent } from "./types";

export type Course = { id: string; code: string; title: string; roster_count?: number; last_session_at?: string };

export type RosterImportReport = { imported_count: number; issues: string[] };

export function getCourses(): Promise<Course[]> {
  return http<Course[]>('/courses');
}

export function getCourseDetail(courseId: string): Promise<Course> {
  return http<Course>(`/courses/${encodeURIComponent(courseId)}`);
}

export function createCourse(input: { code: string; title: string }): Promise<Course> {
  return http<Course>('/courses', { method: 'POST', body: JSON.stringify(input) });
}

export function uploadRoster(courseId: string, students: ApiRosterStudent[]): Promise<{ count: number }> {
  return http<{ count: number }>(`/courses/${encodeURIComponent(courseId)}/roster`, {
    method: 'POST', body: JSON.stringify({ students })
  });
}

export async function importRosterFile(courseId: string, file: File): Promise<RosterImportReport> {
  const formData = new FormData();
  formData.append("file", file);
  const apiBase = import.meta.env.VITE_API_URL ?? 'http://127.0.0.1:8787/api/v1';
  const response = await fetch(`${apiBase}/courses/${encodeURIComponent(courseId)}/roster/import`, {
    method: 'POST',
    body: formData,
  });
  if (!response.ok) {
    const payload = await response.json().catch(() => ({}));
    throw new Error(payload.error?.replaceAll('_', ' ') ?? 'Roster file import failed');
  }
  return response.json() as Promise<RosterImportReport>;
}

export const uploadCourseRoster = uploadRoster;
export const uploadCourseRosterFile = importRosterFile;
