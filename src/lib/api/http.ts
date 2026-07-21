const API_BASE = import.meta.env.VITE_API_URL ?? 'http://127.0.0.1:8787/api/v1';

export type ApiErrorPayload = { error?: string; message?: string; code?: string };

let accessToken: string | null = null;

export function setAccessToken(token: string | null) {
  accessToken = token;
}

export function getAccessToken(): string | null {
  return accessToken;
}

export async function http<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers: Record<string, string> = {
    'content-type': 'application/json',
    ...(init.headers as Record<string, string> ?? {})
  };
  if (accessToken) {
    headers['authorization'] = `Bearer ${accessToken}`;
  }
  const response = await fetch(`${API_BASE}${path}`, { ...init, headers });
  if (!response.ok) {
    const payload = (await response.json().catch(() => ({}))) as ApiErrorPayload;
    const msg = payload.error ?? payload.message ?? 'Service request failed';
    throw new Error(msg.replaceAll('_', ' '));
  }
  return response.json() as Promise<T>;
}
