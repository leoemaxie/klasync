import { apiRequest } from './http';

export interface CourseAnalyticsSummary {
  course_id: string;
  course_code: string;
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
  courseId: string
): Promise<CourseAnalyticsSummary | null> {
  if (!courseId) return null;
  return await apiRequest<CourseAnalyticsSummary>(
    `/analytics/courses/${encodeURIComponent(courseId)}/attendance-summary`
  ).catch(() => null);
}

export async function fetchSessionAnomalies(
  sessionId: string
): Promise<AttendanceAnomaly[]> {
  if (!sessionId) return [];
  return await apiRequest<AttendanceAnomaly[]>(
    `/analytics/sessions/${encodeURIComponent(sessionId)}/anomalies`
  ).catch(() => []);
}
