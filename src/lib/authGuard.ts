import { replace } from 'svelte-spa-router';
import type { SessionState } from './sessionState.svelte';
import { screenFromPath, SCREEN_TO_PATH } from './router';
import type { Screen } from './types';
import { getAccessToken, getRefreshToken, setTokens } from './api/http';

/**
 * Sanitizes sensitive personal and authentication data upon unauthenticated redirect or logout.
 * Purges tokens, user records, matric numbers, and active session cache without destroying
 * non-sensitive database schemas or breaking active IndexedDB connections.
 */
export function purgeSensitiveAuthStorage() {
  setTokens(null, null);

  if (typeof localStorage !== 'undefined') {
    try {
      const keysToRemove = [
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
      ];
      for (const key of keysToRemove) {
        localStorage.removeItem(key);
      }
    } catch {}
  }

  if (typeof sessionStorage !== 'undefined') {
    try {
      sessionStorage.clear();
    } catch {}
  }

  // Sanitize sensitive session_state in IndexedDB without deleting databases
  if (typeof window !== 'undefined') {
    void import('./storage/db')
      .then(async ({ getDB }) => {
        try {
          const db = await getDB();
          if (db.objectStoreNames.contains('session_state')) {
            await db.clear('session_state');
          }
        } catch {}
      })
      .catch(() => {});
  }
}

/**
 * Standardized, high-efficiency auth guard for protecting /lecturer and /archive endpoints.
 * Clears sensitive credentials and redirects away if no access/refresh token is found or if role checks fail.
 */
export function enforceAuthGuard(path: string, state: SessionState): Screen {
  const currentLoc = path || '/';
  const cleanPath =
    currentLoc.length > 1 && currentLoc.endsWith('/')
      ? currentLoc.slice(0, -1)
      : currentLoc;

  const matched = screenFromPath(cleanPath);
  const isAuthRoute = cleanPath.startsWith('/auth');
  const hasToken = Boolean(getAccessToken() || getRefreshToken());

  // 1. Protect Lecturer Workspace (/lecturer and /lecturer/* sub-routes)
  const isLecturerWorkspace =
    cleanPath === '/lecturer' ||
    (cleanPath.startsWith('/lecturer/') && !isAuthRoute);

  if (isLecturerWorkspace) {
    const isLecturer =
      hasToken &&
      state.currentUser &&
      (state.currentUser.role === 'lecturer' ||
        state.currentUser.role === 'admin');

    if (!isLecturer) {
      if (!hasToken || !state.currentUser) {
        purgeSensitiveAuthStorage();
        state.currentUser = null;
      }
      state.authNotice =
        !hasToken || !state.currentUser
          ? 'Please sign in to access this page.'
          : 'Access restricted: Lecturer Workspace is only accessible to lecturer accounts.';
      const target = SCREEN_TO_PATH['lecturer-login'];
      if (currentLoc !== target) void replace(target);
      return 'lecturer-login';
    }
    return 'lecturer';
  }

  // 2. Protect Student Archive (/archive and /archive/* sub-routes)
  const isArchiveWorkspace =
    cleanPath === '/archive' ||
    (cleanPath.startsWith('/archive/') && !isAuthRoute);

  if (isArchiveWorkspace) {
    const isStudent =
      hasToken && state.currentUser && state.currentUser.role === 'student';

    if (!isStudent) {
      if (!hasToken || !state.currentUser) {
        purgeSensitiveAuthStorage();
        state.currentUser = null;
      }
      state.authNotice =
        !hasToken || !state.currentUser
          ? 'Please sign in to access this page.'
          : 'Access restricted: Student Archive is only accessible to student accounts.';
      const target = SCREEN_TO_PATH['student-login'];
      if (currentLoc !== target) void replace(target);
      return 'student-login';
    }
    return 'archive';
  }

  // 3. Redirect authenticated users with valid tokens away from /auth/* login/register pages
  if (isAuthRoute && hasToken && state.currentUser) {
    if (
      (matched === 'lecturer-login' || matched === 'lecturer-register') &&
      (state.currentUser.role === 'lecturer' ||
        state.currentUser.role === 'admin')
    ) {
      const target = SCREEN_TO_PATH['lecturer'];
      if (currentLoc !== target) void replace(target);
      return 'lecturer';
    }

    if (
      (matched === 'student-login' || matched === 'student-register') &&
      state.currentUser.role === 'student'
    ) {
      const target = SCREEN_TO_PATH['archive'];
      if (currentLoc !== target) void replace(target);
      return 'archive';
    }
  }

  return matched;
}
