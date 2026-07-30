import { http } from "./http";
import type { ApiParticipant } from "./types";

export function sendPresenceHeartbeat(participantId: string): Promise<ApiParticipant> {
  return http<ApiParticipant>(`/participants/${encodeURIComponent(participantId)}/presence`, {
    method: 'POST'
  });
}
