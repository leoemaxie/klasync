import { push, replace } from 'svelte-spa-router';
import type { Screen } from './types';

export const SCREEN_TO_PATH: Record<Screen, string> = {
  home: '/',
  lecturer: '/lecturer',
  'lecturer-login': '/lecturer-login',
  'lecturer-register': '/lecturer-register',
  'student-login': '/student-login',
  'student-register': '/student-register',
  'recover-password': '/recover-password',
  'reset-password': '/reset-password',
  join: '/join',
  live: '/live',
  archive: '/archive',
  'not-found': '/404',
};

export const PATH_TO_SCREEN: Record<string, Screen> = {
  '/': 'home',
  '/home': 'home',
  '/lecturer': 'lecturer',
  '/lecturer-login': 'lecturer-login',
  '/lecturer/login': 'lecturer-login',
  '/lecturer-register': 'lecturer-register',
  '/lecturer/register': 'lecturer-register',
  '/student-login': 'student-login',
  '/student/login': 'student-login',
  '/student-register': 'student-register',
  '/student/register': 'student-register',
  '/recover-password': 'recover-password',
  '/recover': 'recover-password',
  '/reset-password': 'reset-password',
  '/reset': 'reset-password',
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
  return PATH_TO_SCREEN[cleanPath] || 'not-found';
}
