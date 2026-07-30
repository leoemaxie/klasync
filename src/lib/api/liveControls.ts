import { http } from "./http";

export type LiveControlPatch = {
  captions_paused?: boolean;
  audio_ingestion_active?: boolean;
  late_join_policy?: 'allowed' | 'roster_only' | 'closed';
};

export type LiveControlState = {
  session_id: string;
  captions_paused: boolean;
  audio_ingestion_active: boolean;
  late_join_policy: string;
};

export type CaptionModerationInput = {
  text?: string;
  hidden?: boolean;
  note?: string;
};

export function updateLiveControls(shortCode: string, input: LiveControlPatch): Promise<LiveControlState> {
  return http<LiveControlState>(`/sessions/code/${encodeURIComponent(shortCode)}/live-controls`, {
    method: 'POST',
    body: JSON.stringify(input)
  });
}

export function pauseCaptions(shortCode: string): Promise<LiveControlState> {
  return http<LiveControlState>(`/sessions/code/${encodeURIComponent(shortCode)}/captions/pause`, { method: 'POST' });
}

export function resumeCaptions(shortCode: string): Promise<LiveControlState> {
  return http<LiveControlState>(`/sessions/code/${encodeURIComponent(shortCode)}/captions/resume`, { method: 'POST' });
}

export function startAudioIngestion(shortCode: string): Promise<LiveControlState> {
  return http<LiveControlState>(`/sessions/code/${encodeURIComponent(shortCode)}/audio/start`, { method: 'POST' });
}

export function stopAudioIngestion(shortCode: string): Promise<LiveControlState> {
  return http<LiveControlState>(`/sessions/code/${encodeURIComponent(shortCode)}/audio/stop`, { method: 'POST' });
}

export function moderateParticipant(shortCode: string, participantId: string, action: 'mute' | 'remove'): Promise<void> {
  return http<void>(`/sessions/code/${encodeURIComponent(shortCode)}/participants/${encodeURIComponent(participantId)}/${action}`, {
    method: 'POST'
  });
}

export function moderateCaption(shortCode: string, captionId: string, input: CaptionModerationInput): Promise<void> {
  return http<void>(`/sessions/code/${encodeURIComponent(shortCode)}/captions/${encodeURIComponent(captionId)}/moderate`, {
    method: 'POST',
    body: JSON.stringify(input)
  });
}
