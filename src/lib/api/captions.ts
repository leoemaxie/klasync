import { http, WS_BASE } from './http';
import type { ApiCaption } from './types';

export function getCaptions(shortCode: string): Promise<ApiCaption[]> {
  return http<ApiCaption[]>(
    `/sessions/code/${encodeURIComponent(shortCode)}/captions`
  );
}

export function publishCaption(
  shortCode: string,
  text: string
): Promise<ApiCaption> {
  return http<ApiCaption>(
    `/sessions/code/${encodeURIComponent(shortCode)}/captions`,
    {
      method: 'POST',
      body: JSON.stringify({ text }),
    }
  );
}

export function connectCaptionWebSocket(
  shortCode: string,
  onMessage: (caption: ApiCaption) => void
): () => void {
  const wsUrl = `${WS_BASE}/sessions/code/${encodeURIComponent(shortCode)}/captions/ws`;
  const ws = new WebSocket(wsUrl);
  ws.onmessage = (event) => {
    try {
      const data = JSON.parse(event.data) as ApiCaption;
      onMessage(data);
    } catch {
      /* handle non-JSON caption */
    }
  };
  return () => ws.close();
}
