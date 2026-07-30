import { http } from "./http";
import type { ApiParticipant } from "./types";

export function sendPresenceHeartbeat(participantIdOrShortCode: string, matricNumber?: string): Promise<ApiParticipant> {
  const path = matricNumber
    ? `/sessions/code/${encodeURIComponent(participantIdOrShortCode)}/presence`
    : `/participants/${encodeURIComponent(participantIdOrShortCode)}/presence`;
  return http<ApiParticipant>(path, {
    method: 'POST',
    body: matricNumber ? JSON.stringify({ matric_number: matricNumber }) : undefined
  });
}
