import { http, setAccessToken } from "./http";

export type AuthUser = { id: string; name: string; email: string; role: string; matric_number?: string };
export type AuthResponse = { access_token: string; user: AuthUser };

export async function loginLecturer(email: string, password: string): Promise<AuthResponse> {
  const res = await http<AuthResponse>('/auth/lecturers/login', {
    method: 'POST', body: JSON.stringify({ email, password })
  });
  setAccessToken(res.access_token);
  return res;
}

export async function registerLecturer(data: { name: string; email: string; password: string }): Promise<AuthResponse> {
  const res = await http<AuthResponse>('/auth/lecturers/register', {
    method: 'POST', body: JSON.stringify(data)
  });
  setAccessToken(res.access_token);
  return res;
}

export async function loginStudent(email: string, password: string): Promise<AuthResponse> {
  const res = await http<AuthResponse>('/auth/students/login', {
    method: 'POST', body: JSON.stringify({ email, password })
  });
  setAccessToken(res.access_token);
  return res;
}

export async function registerStudent(data: { matric_number: string; name: string; email: string; password: string }): Promise<AuthResponse> {
  const res = await http<AuthResponse>('/auth/students/register', {
    method: 'POST', body: JSON.stringify(data)
  });
  setAccessToken(res.access_token);
  return res;
}

export async function requestPasswordReset(email: string, role: string): Promise<{ success: boolean }> {
  return http<{ success: boolean }>('/auth/password-reset/request', {
    method: 'POST', body: JSON.stringify({ email, role })
  });
}

export async function completePasswordReset(token: string, newPassword: string): Promise<{ success: boolean }> {
  return http<{ success: boolean }>('/auth/password-reset/complete', {
    method: 'POST', body: JSON.stringify({ token, new_password: newPassword })
  });
}

export async function refreshToken(): Promise<AuthResponse> {
  const res = await http<AuthResponse>('/auth/refresh', { method: 'POST' });
  setAccessToken(res.access_token);
  return res;
}

export async function logout(): Promise<void> {
  await http('/auth/logout', { method: 'POST' });
  setAccessToken(null);
}
