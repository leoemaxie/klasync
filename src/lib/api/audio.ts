import type { Resource } from "./archive";

export async function uploadSessionAudio(shortCode: string, file: File): Promise<Resource> {
  const formData = new FormData();
  formData.append("file", file);
  const apiBase = import.meta.env.VITE_API_URL ?? 'http://127.0.0.1:8787/api/v1';
  const response = await fetch(`${apiBase}/sessions/code/${encodeURIComponent(shortCode)}/audio/upload`, {
    method: 'POST',
    body: formData,
  });
  if (!response.ok) {
    const payload = await response.json().catch(() => ({}));
    throw new Error(payload.error ?? 'Audio file upload failed');
  }
  return response.json() as Promise<Resource>;
}

export function getAudioChunkUrl(shortCode: string, seq: number): string {
  const apiBase = import.meta.env.VITE_API_URL ?? 'http://127.0.0.1:8787/api/v1';
  return `${apiBase}/sessions/code/${encodeURIComponent(shortCode)}/audio/chunk/${seq}`;
}

export async function uploadAudioChunk(shortCode: string, chunk: Blob, seq: number): Promise<void> {
  const formData = new FormData();
  formData.append("chunk", chunk);
  formData.append("seq", seq.toString());
  const apiBase = import.meta.env.VITE_API_URL ?? 'http://127.0.0.1:8787/api/v1';
  await fetch(`${apiBase}/sessions/code/${encodeURIComponent(shortCode)}/audio/chunk`, {
    method: 'POST',
    body: formData,
  });
}
