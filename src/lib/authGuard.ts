import { replace } from 'svelte-spa-router';
import type { SessionState } from './sessionState.svelte';
import { screenFromPath, SCREEN_TO_PATH } from './router';
import type { Screen } from './types';
import { getAccessToken, getRefreshToken, setTokens } from './api/http';

export function purgeSensitiveAuthStorage() {
  setTokens(null, null);
  if (typeof localStorage !== 'undefined') {
    [
      'klasync_access_token',
      'klasync_refresh_token',
      'klasync-user',
      'klasync-lecturer',
      'klasync-session',
      'klasync-roster',
      'klasync-rosterText',
      'klasync-courseCode',
      'klasync-courseTitle',
      'klasync-captions',
    ].forEach((k) => {
      try { localStorage.removeItem(k); } catch {}
    });
  }
  if (typeof sessionStorage !== 'undefined') {
    try { sessionStorage.clear(); } catch {}
  }
  if (typeof window !== 'undefined') {
    void import('./storage/db')
      .then(async ({ getDB }) => {
        try {
          const db = await getDB();
          if (db.objectStoreNames.contains('session_state')) await db.clear('session_state');
        } catch {}
      })
      .catch(() => {});
  }
}

export function enforceAuthGuard(path: string, state: SessionState): Screen {
  const currentLoc = path || '/';
  const cleanPath = currentLoc.length > 1 && currentLoc.endsWith('/') ? currentLoc.slice(0, -1) : currentLoc;
  const matched = screenFromPath(cleanPath);
  const isAuthRoute = cleanPath.startsWith('/auth');
  const hasToken = Boolean(getAccessToken() || getRefreshToken());
  const role = state.currentUser?.role;

  // 1. Protect Lecturer Workspace
  if (cleanPath === '/lecturer' || (cleanPath.startsWith('/lecturer/') && !isAuthRoute)) {
    const isLecturer = hasToken && (role === 'lecturer' || role === 'admin');
    if (!isLecturer) {
      if (!hasToken || !state.currentUser) {
        purgeSensitiveAuthStorage();
        state.currentUser = null;
      }
      state.authNotice = !hasToken || !state.currentUser
        ? 'Please sign in to access this page.'
        : 'Access restricted: Lecturer Workspace is only accessible to lecturer accounts.';
      const target = SCREEN_TO_PATH['lecturer-login'];
      if (currentLoc !== target) void replace(target);
      return 'lecturer-login';
    }
    return 'lecturer';
  }

  // 2. Protect Student Archive
  if (cleanPath === '/archive' || (cleanPath.startsWith('/archive/') && !isAuthRoute)) {
    const isStudent = hasToken && role === 'student';
    if (!isStudent) {
      if (!hasToken || !state.currentUser) {
        purgeSensitiveAuthStorage();
        state.currentUser = null;
      }
      state.authNotice = !hasToken || !state.currentUser
        ? 'Please sign in to access this page.'
        : 'Access restricted: Student Archive is only accessible to student accounts.';
      const target = SCREEN_TO_PATH['student-login'];
      if (currentLoc !== target) void replace(target);
      return 'student-login';
    }
    return 'archive';
  }

  // 3. Redirect authenticated users away from /auth/* login/register pages
  if (isAuthRoute && hasToken && state.currentUser) {
    if ((matched === 'lecturer-login' || matched === 'lecturer-register') && (role === 'lecturer' || role === 'admin')) {
      const target = SCREEN_TO_PATH['lecturer'];
      if (currentLoc !== target) void replace(target);
      return 'lecturer';
    }
    if ((matched === 'student-login' || matched === 'student-register') && role === 'student') {
      const target = SCREEN_TO_PATH['archive'];
      if (currentLoc !== target) void replace(target);
      return 'archive';
    }
  }

  // 4. Redirect authenticated users navigating to root / home to their dedicated hubs
  if ((cleanPath === '/' || cleanPath === '/home') && hasToken && state.currentUser) {
    if (role === 'student') {
      const target = SCREEN_TO_PATH['archive'];
      if (currentLoc !== target) void replace(target);
      return 'archive';
    }
    if (role === 'lecturer' || role === 'admin') {
      const target = SCREEN_TO_PATH['lecturer'];
      if (currentLoc !== target) void replace(target);
      return 'lecturer';
    }
  }

  return matched;
}
