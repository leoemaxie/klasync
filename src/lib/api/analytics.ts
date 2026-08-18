import { apiRequest } from './http';
import { resolveCourseUuid } from './courses';
import { resolveSessionUuid } from './sessions';

export interface CourseAnalyticsSummary {
  course_id: string;
  course_code?: string;
  total_sessions: number;
  avg_attendance_percentage: number;
  roster_verification_match_rate: number;
  total_provisional_students: number;
  total_anomalies_flagged: number;
}

export interface AttendanceAnomaly {
  id: string;
  matric_number: string;
  anomaly_type: 'heartbeat_burst' | 'unverified_location' | 'rapid_checkin';
  description: string;
  severity: 'info' | 'warning' | 'critical';
  logged_at: string;
}

export async function fetchCourseAnalytics(
  courseIdOrCode: string
): Promise<CourseAnalyticsSummary | null> {
  const trimmed = courseIdOrCode.trim();
  if (!trimmed) return null;
  const targetId = await resolveCourseUuid(trimmed);
  return await apiRequest<CourseAnalyticsSummary>(
    `/analytics/courses/${encodeURIComponent(targetId || trimmed)}/attendance-summary`
  ).catch(() => null);
}

export async function fetchCourseAnomalies(
  courseIdOrCode: string
): Promise<AttendanceAnomaly[]> {
  const trimmed = courseIdOrCode.trim();
  if (!trimmed) return [];
  const targetId = await resolveCourseUuid(trimmed);
  return await apiRequest<AttendanceAnomaly[]>(
    `/analytics/courses/${encodeURIComponent(targetId || trimmed)}/anomalies`
  ).catch(() => []);
}

export async function fetchSessionAnomalies(
  sessionIdOrCode: string
): Promise<AttendanceAnomaly[]> {
  const targetId = await resolveSessionUuid(sessionIdOrCode);
  if (!targetId) return [];
  return await apiRequest<AttendanceAnomaly[]>(
    `/analytics/sessions/${encodeURIComponent(targetId)}/anomalies`
  ).catch(() => []);
}
