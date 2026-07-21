import { http } from "./http";
import type { ApiRosterStudent } from "../api";

export type Course = { id: string; code: string; title: string; roster_count?: number; last_session_at?: string };

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
