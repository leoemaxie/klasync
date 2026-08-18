import { http } from './http';
import type { SuccessResponse } from './types';
import {
  saveLocalStudentClaim,
  claimLecture,
  type ClaimRecord,
} from './archive';

export type ClaimRequestResponse = {
  verification_id: string;
  expires_at: string;
};

export type ClaimVerifyResponse = {
  participant_id: string;
  status: string;
};

export function requestClaimVerification(
  participantId: string
): Promise<ClaimRequestResponse> {
  return http<ClaimRequestResponse>('/students/claims/request-verification', {
    method: 'POST',
    body: JSON.stringify({ participant_id: participantId }),
  });
}

export function verifyClaimCode(
  verificationId: string,
  code: string
): Promise<ClaimVerifyResponse> {
  return http<ClaimVerifyResponse>('/students/claims/verify', {
    method: 'POST',
    body: JSON.stringify({ verification_id: verificationId, code }),
  });
}

export async function claimAttendance(
  shortCode: string,
  matricNumber: string,
  participantId?: string,
  sessionTitle?: string,
  courseCode?: string,
  sessionId?: string
): Promise<SuccessResponse> {
  const claimRecord: ClaimRecord = {
    id: sessionId || participantId || shortCode,
    session_code: shortCode,
    course_code: courseCode || 'COURSE',
    session_title: sessionTitle || `Session (${shortCode})`,
    date: new Date().toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    }),
  };
  saveLocalStudentClaim(claimRecord);

  if (participantId) {
    try {
      await claimLecture(participantId);
    } catch {}
  }

  return { success: true, message: 'Lecture claimed successfully.' };
}

export function verifyClaim(claimId: string): Promise<ClaimVerifyResponse> {
  return http<ClaimVerifyResponse>(
    `/claims/${encodeURIComponent(claimId)}/verify`,
    {
      method: 'POST',
    }
  );
}
