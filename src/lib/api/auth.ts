import { http, setAccessToken } from './http';
import type { SuccessResponse } from './types';

export type AuthUser = {
  id: string;
  name: string;
  email: string;
  role: string;
  matric_number?: string;
};
export type AuthResponse = {
  access_token: string;
  refresh_token?: string;
  user?: AuthUser;
};

export async function getLecturerProfile(): Promise<AuthUser> {
  return http<AuthUser>('/auth/lecturers/me', { method: 'GET' }).catch(() => ({
    id: '',
    name: 'Lecturer',
    email: '',
    role: 'lecturer',
  }));
}

export async function loginLecturer(
  email: string,
  password: string
): Promise<{ access_token: string; user: AuthUser }> {
  const res = await http<AuthResponse>('/auth/lecturers/login', {
    method: 'POST',
    body: JSON.stringify({ email, password }),
  });
  setAccessToken(res.access_token);

  const user =
    res.user ??
    (await getLecturerProfile().catch(() => ({
      id: '',
      name: 'Lecturer',
      email,
      role: 'lecturer',
    })));
  return { access_token: res.access_token, user };
}

export async function registerLecturer(data: {
  name: string;
  email: string;
  password: string;
}): Promise<{ access_token: string; user: AuthUser }> {
  const res = await http<AuthResponse>('/auth/lecturers/register', {
    method: 'POST',
    body: JSON.stringify(data),
  });
  setAccessToken(res.access_token);

  const user =
    res.user ??
    (await getLecturerProfile().catch(() => ({
      id: '',
      name: data.name,
      email: data.email,
      role: 'lecturer',
    })));
  return { access_token: res.access_token, user };
}

export async function getStudentProfile(): Promise<AuthUser> {
  return http<AuthUser>('/auth/students/me', { method: 'GET' }).catch(() => ({
    id: '',
    name: 'Student',
    email: '',
    role: 'student',
  }));
}

export async function loginStudent(
  email: string,
  password: string
): Promise<{ access_token: string; user: AuthUser }> {
  const res = await http<AuthResponse>('/auth/students/login', {
    method: 'POST',
    body: JSON.stringify({ email, password }),
  });
  setAccessToken(res.access_token);

  const user =
    res.user ??
    (await getStudentProfile().catch(() => ({
      id: '',
      name: 'Student',
      email,
      role: 'student',
    })));
  return { access_token: res.access_token, user };
}

export async function registerStudent(data: {
  matric_number: string;
  name: string;
  email: string;
  password: string;
}): Promise<{ access_token: string; user: AuthUser }> {
  const res = await http<AuthResponse>('/auth/students/register', {
    method: 'POST',
    body: JSON.stringify({
      matric_number: data.matric_number,
      display_name: data.name,
      email: data.email,
      password: data.password,
    }),
  });
  setAccessToken(res.access_token);

  const user =
    res.user ??
    (await getStudentProfile().catch(() => ({
      id: '',
      name: data.name,
      email: data.email,
      role: 'student',
      matric_number: data.matric_number,
    })));
  return { access_token: res.access_token, user };
}

export async function requestPasswordReset(
  email: string,
  role: string
): Promise<SuccessResponse> {
  return http<SuccessResponse>('/auth/password-reset/request', {
    method: 'POST',
    body: JSON.stringify({ email, role }),
  });
}

export async function completePasswordReset(
  token: string,
  newPassword: string
): Promise<SuccessResponse> {
  return http<SuccessResponse>('/auth/password-reset/complete', {
    method: 'POST',
    body: JSON.stringify({ reset_token: token, new_password: newPassword }),
  });
}

export async function refreshToken(token?: string): Promise<AuthResponse> {
  const res = await http<AuthResponse>('/auth/refresh', {
    method: 'POST',
    body: JSON.stringify({ refresh_token: token ?? '' }),
  });
  setAccessToken(res.access_token);
  return res;
}

export async function logout(token?: string): Promise<void> {
  await http('/auth/logout', {
    method: 'POST',
    body: JSON.stringify({ refresh_token: token ?? '' }),
  });
  setAccessToken(null);
}
