import { replace } from 'svelte-spa-router';
import type { SessionState } from './sessionState.svelte';
import { screenFromPath, SCREEN_TO_PATH } from './router';
import type { Screen } from './types';
import { getAccessToken, getRefreshToken, setTokens } from './api/http';

/**
 * Clears localStorage, sessionStorage, and IndexedDB storage upon unauthenticated redirect.
 */
export function clearAllClientStorage() {
  try {
    if (typeof localStorage !== 'undefined') {
      localStorage.clear();
    }
  } catch {}

  try {
    if (typeof sessionStorage !== 'undefined') {
      sessionStorage.clear();
    }
  } catch {}

  try {
    if (typeof indexedDB !== 'undefined' && 'databases' in indexedDB && typeof indexedDB.databases === 'function') {
      indexedDB
        .databases()
        .then((dbs) => {
          for (const db of dbs) {
            if (db.name) {
              try {
                indexedDB.deleteDatabase(db.name);
              } catch {}
            }
          }
        })
        .catch(() => {});
    }
  } catch {}
}

/**
 * Standardized, high-efficiency auth guard for protecting /lecturer and /archive endpoints.
 * Clears storage and redirects away if no access/refresh token is found or if role checks fail.
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
      (state.currentUser.role === 'lecturer' || state.currentUser.role === 'admin');

    if (!isLecturer) {
      if (!hasToken || !state.currentUser) {
        clearAllClientStorage();
        setTokens(null, null);
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

  // 2. Protect Student Archive (/archive and /archive/* sub-routes)
  const isArchiveWorkspace =
    cleanPath === '/archive' ||
    (cleanPath.startsWith('/archive/') && !isAuthRoute);

  if (isArchiveWorkspace) {
    const isStudent =
      hasToken &&
      state.currentUser &&
      state.currentUser.role === 'student';

    if (!isStudent) {
      if (!hasToken || !state.currentUser) {
        clearAllClientStorage();
        setTokens(null, null);
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

  // 3. Redirect authenticated users with valid tokens away from /auth/* login/register pages
  if (isAuthRoute && hasToken && state.currentUser) {
    if (
      (matched === 'lecturer-login' || matched === 'lecturer-register') &&
      (state.currentUser.role === 'lecturer' || state.currentUser.role === 'admin')
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
