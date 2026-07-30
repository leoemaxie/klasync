import { http } from "./http";

export type ReconciliationReport = {
  session_id: string;
  total_participants: number;
  scored_participants: number;
  average_score: number;
};

export function reconcileAttendance(shortCode: string): Promise<ReconciliationReport> {
  return http<ReconciliationReport>(`/sessions/code/${encodeURIComponent(shortCode)}/attendance/reconcile`, {
    method: 'POST'
  });
}
