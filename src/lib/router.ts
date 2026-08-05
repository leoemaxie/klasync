import { push, replace } from 'svelte-spa-router';
import type { Screen } from './types';

export const SCREEN_TO_PATH: Record<Screen, string> = {
  home: '/',
  lecturer: '/lecturer',
  'lecturer-login': '/auth/lecturer/login',
  'lecturer-register': '/auth/lecturer/register',
  'student-login': '/auth/student/login',
  'student-register': '/auth/student/register',
  'recover-password': '/auth/recover-password',
  'reset-password': '/auth/reset-password',
  join: '/join',
  live: '/live',
  archive: '/archive',
  'not-found': '/404',
};

export const PATH_TO_SCREEN: Record<string, Screen> = {
  '/': 'home',
  '/home': 'home',
  '/lecturer': 'lecturer',

  // Standardized /auth/* endpoints
  '/auth/lecturer/login': 'lecturer-login',
  '/auth/lecturer/register': 'lecturer-register',
  '/auth/student/login': 'student-login',
  '/auth/student/register': 'student-register',
  '/auth/recover-password': 'recover-password',
  '/auth/reset-password': 'reset-password',

  '/join': 'join',
  '/live': 'live',
  '/archive': 'archive',
};

export function navigateTo(screen: Screen, replaceRoute = false) {
  const path = SCREEN_TO_PATH[screen] || '/404';
  if (replaceRoute) {
    void replace(path);
  } else {
    void push(path);
  }
}

export function screenFromPath(path: string): Screen {
  if (!path) return 'home';
  const cleanPath =
    path.length > 1 && path.endsWith('/') ? path.slice(0, -1) : path;
  if (PATH_TO_SCREEN[cleanPath]) {
    return PATH_TO_SCREEN[cleanPath];
  }
  if (cleanPath.startsWith('/lecturer') && !cleanPath.startsWith('/auth/')) {
    return 'lecturer';
  }
  if (cleanPath.startsWith('/archive') && !cleanPath.startsWith('/auth/')) {
    return 'archive';
  }
  return 'not-found';
}
