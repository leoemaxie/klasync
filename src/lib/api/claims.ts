import { http } from "./http";

export type ClaimRequestResponse = {
  verification_id: string;
  expires_at: string;
};

export type ClaimVerifyResponse = {
  participant_id: string;
  status: string;
};

export function requestClaimVerification(participantId: string): Promise<ClaimRequestResponse> {
  return http<ClaimRequestResponse>('/students/claims/request-verification', {
    method: 'POST',
    body: JSON.stringify({ participant_id: participantId })
  });
}

export function verifyClaimCode(verificationId: string, code: string): Promise<ClaimVerifyResponse> {
  return http<ClaimVerifyResponse>('/students/claims/verify', {
    method: 'POST',
    body: JSON.stringify({ verification_id: verificationId, code })
  });
}

export function claimAttendance(shortCode: string, matricNumber: string): Promise<{ success: boolean }> {
  return http<{ success: boolean }>(`/sessions/code/${encodeURIComponent(shortCode)}/claims`, {
    method: 'POST',
    body: JSON.stringify({ matric_number: matricNumber })
  });
}

export function verifyClaim(claimId: string): Promise<ClaimVerifyResponse> {
  return http<ClaimVerifyResponse>(`/claims/${encodeURIComponent(claimId)}/verify`, {
    method: 'POST'
  });
}
