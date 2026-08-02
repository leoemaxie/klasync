import { http } from './http';

export type ReconciliationReport = {
  session_id: string;
  total_participants: number;
  scored_participants: number;
  average_score: number;
};

export function reconcileAttendance(
  shortCode: string
): Promise<ReconciliationReport> {
  return http<ReconciliationReport>(
    `/sessions/code/${encodeURIComponent(shortCode)}/attendance/reconcile`,
    {
      method: 'POST',
    }
  );
}

export function getSessionAttendance(
  shortCode: string
): Promise<ReconciliationReport> {
  return http<ReconciliationReport>(
    `/sessions/code/${encodeURIComponent(shortCode)}/attendance`,
    {
      method: 'GET',
    }
  );
}

export function exportSessionAttendanceCsv(shortCode: string): Promise<string> {
  return http<string>(
    `/sessions/code/${encodeURIComponent(shortCode)}/attendance.csv`,
    {
      headers: { accept: 'text/csv' },
    }
  );
}
