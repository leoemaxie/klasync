export const API_BASE =
  import.meta.env.VITE_API_URL ?? 'http://127.0.0.1:8787/api/v1';

export const WS_BASE = API_BASE.replace(/^http/, 'ws');

export type ApiErrorPayload = {
  error?: string;
  message?: string;
  code?: string;
};

let accessToken: string | null =
  typeof localStorage !== 'undefined'
    ? localStorage.getItem('klasync_access_token')
    : null;
let refreshToken: string | null =
  typeof localStorage !== 'undefined'
    ? localStorage.getItem('klasync_refresh_token')
    : null;

export function setTokens(access: string | null, refresh?: string | null) {
  accessToken = access;
  if (typeof localStorage !== 'undefined') {
    if (access) {
      localStorage.setItem('klasync_access_token', access);
    } else {
      localStorage.removeItem('klasync_access_token');
    }
  }

  if (refresh !== undefined) {
    refreshToken = refresh;
    if (typeof localStorage !== 'undefined') {
      if (refresh) {
        localStorage.setItem('klasync_refresh_token', refresh);
      } else {
        localStorage.removeItem('klasync_refresh_token');
      }
    }
  }
}

export function setAccessToken(token: string | null) {
  setTokens(token);
}

export function setRefreshToken(token: string | null) {
  setTokens(accessToken, token);
}

export function getAccessToken(): string | null {
  return accessToken;
}

export function getRefreshToken(): string | null {
  return refreshToken;
}

let isRefreshingPromise: Promise<boolean> | null = null;

async function doRefreshToken(): Promise<boolean> {
  const curRefreshToken = getRefreshToken();
  if (!curRefreshToken) {
    setTokens(null, null);
    return false;
  }

  try {
    const res = await fetch(`${API_BASE}/auth/refresh`, {
      method: 'POST',
      credentials: 'include',
      headers: {
        'content-type': 'application/json',
      },
      body: JSON.stringify({ refresh_token: curRefreshToken }),
    });
    if (!res.ok) {
      setTokens(null, null);
      return false;
    }
    const data = (await res.json().catch(() => ({}))) as {
      access_token?: string;
      refresh_token?: string;
    };
    if (data.access_token) {
      setTokens(data.access_token, data.refresh_token ?? curRefreshToken);
      return true;
    }
    setTokens(null, null);
    return false;
  } catch {
    setTokens(null, null);
    return false;
  } finally {
    isRefreshingPromise = null;
  }
}

export async function http<T>(
  path: string,
  init: RequestInit = {}
): Promise<T> {
  const isAuthEndpoint =
    path.startsWith('/auth/') &&
    !path.startsWith('/auth/lecturers/me') &&
    !path.startsWith('/auth/students/me');

  const makeHeaders = () => {
    const headers: Record<string, string> = {
      'content-type': 'application/json',
      ...((init.headers as Record<string, string>) ?? {}),
    };
    const token = getAccessToken();
    if (token && !isAuthEndpoint) {
      headers['authorization'] = `Bearer ${token}`;
    }
    return headers;
  };

  const options: RequestInit = {
    credentials: 'include',
    ...init,
    headers: makeHeaders(),
  };

  let response = await fetch(`${API_BASE}${path}`, options);

  if (response.status === 401 && !isAuthEndpoint) {
    if (!isRefreshingPromise) {
      isRefreshingPromise = doRefreshToken();
    }
    const refreshed = await isRefreshingPromise;
    if (refreshed) {
      const retryOptions: RequestInit = {
        ...init,
        credentials: 'include',
        headers: makeHeaders(),
      };
      response = await fetch(`${API_BASE}${path}`, retryOptions);
    }
  }

  if (!response.ok) {
    const payload = (await response
      .json()
      .catch(() => ({}))) as ApiErrorPayload;
    const msg = payload.error ?? payload.message ?? 'Service request failed';
    throw new Error(msg.replaceAll('_', ' '));
  }
  return response.json() as Promise<T>;
}

export const apiRequest = http;
