const API_BASE = import.meta.env.VITE_API_URL ?? 'http://127.0.0.1:8787/api/v1';

export type ApiErrorPayload = { error?: string; message?: string; code?: string };

let accessToken: string | null = null;

export function setAccessToken(token: string | null) {
  accessToken = token;
}

export function getAccessToken(): string | null {
  return accessToken;
}

let isRefreshingPromise: Promise<boolean> | null = null;

async function doRefreshToken(): Promise<boolean> {
  try {
    const headers: Record<string, string> = { 'content-type': 'application/json' };
    if (accessToken) {
      headers['authorization'] = `Bearer ${accessToken}`;
    }
    const res = await fetch(`${API_BASE}/auth/refresh`, {
      method: 'POST',
      credentials: 'include',
      headers
    });
    if (!res.ok) {
      setAccessToken(null);
      return false;
    }
    const data = (await res.json().catch(() => ({}))) as { access_token?: string };
    if (data.access_token) {
      setAccessToken(data.access_token);
      return true;
    }
    setAccessToken(null);
    return false;
  } catch {
    setAccessToken(null);
    return false;
  } finally {
    isRefreshingPromise = null;
  }
}

export async function http<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers: Record<string, string> = {
    'content-type': 'application/json',
    ...(init.headers as Record<string, string> ?? {})
  };
  if (accessToken) {
    headers['authorization'] = `Bearer ${accessToken}`;
  }

  const requestInit: RequestInit = {
    credentials: 'include',
    ...init,
    headers
  };

  let response = await fetch(`${API_BASE}${path}`, requestInit);

  const isAuthEndpoint = path.startsWith('/auth/refresh') ||
    path.startsWith('/auth/lecturers/login') ||
    path.startsWith('/auth/lecturers/register') ||
    path.startsWith('/auth/students/login') ||
    path.startsWith('/auth/students/register');

  if (response.status === 401 && !isAuthEndpoint) {
    if (!isRefreshingPromise) {
      isRefreshingPromise = doRefreshToken();
    }
    const refreshed = await isRefreshingPromise;
    if (refreshed) {
      if (accessToken) {
        headers['authorization'] = `Bearer ${accessToken}`;
      } else {
        delete headers['authorization'];
      }
      response = await fetch(`${API_BASE}${path}`, { ...requestInit, headers });
    }
  }

  if (!response.ok) {
    const payload = (await response.json().catch(() => ({}))) as ApiErrorPayload;
    const msg = payload.error ?? payload.message ?? 'Service request failed';
    throw new Error(msg.replaceAll('_', ' '));
  }
  return response.json() as Promise<T>;
}

export const apiRequest = http;

