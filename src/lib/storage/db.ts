import { openDB, type DBSchema, type IDBPDatabase } from 'idb';
import type { ApiCaption, Course, Resource } from '../api';

export interface KlasyncDBSchema extends DBSchema {
  captions: {
    key: string;
    value: ApiCaption & {
      session_code: string;
      timestamp: number;
      cached_at: number;
    };
    indexes: {
      'by-session': string;
      'by-timestamp': number;
    };
  };
  courses: {
    key: string;
    value: Course & { updated_at: number };
    indexes: {
      'by-code': string;
    };
  };
  archive: {
    key: string;
    value: Resource & { session_code?: string; cached_at: number };
    indexes: {
      'by-session': string;
      'by-cached-at': number;
    };
  };
  session_state: {
    key: string;
    value: {
      session_code: string;
      data: any;
      updated_at: number;
    };
  };
}

const DB_NAME = 'klasync_client_db';
const DB_VERSION = 1;
const THIRTY_DAYS_MS = 30 * 24 * 60 * 60 * 1000;

let dbPromise: Promise<IDBPDatabase<KlasyncDBSchema>> | null = null;

export function getDB(): Promise<IDBPDatabase<KlasyncDBSchema>> {
  if (typeof window === 'undefined') {
    return Promise.reject(
      new Error('IndexedDB is only available in browser environments')
    );
  }
  if (!dbPromise) {
    dbPromise = openDB<KlasyncDBSchema>(DB_NAME, DB_VERSION, {
      upgrade(db) {
        // 1. Live Captions Store
        if (!db.objectStoreNames.contains('captions')) {
          const captionStore = db.createObjectStore('captions', {
            keyPath: 'id',
          });
          captionStore.createIndex('by-session', 'session_code');
          captionStore.createIndex('by-timestamp', 'timestamp');
        }

        // 2. Courses Store
        if (!db.objectStoreNames.contains('courses')) {
          const courseStore = db.createObjectStore('courses', {
            keyPath: 'id',
          });
          courseStore.createIndex('by-code', 'code');
        }

        // 3. Student Offline Archive Store
        if (!db.objectStoreNames.contains('archive')) {
          const archiveStore = db.createObjectStore('archive', {
            keyPath: 'id',
          });
          archiveStore.createIndex('by-session', 'session_code');
          archiveStore.createIndex('by-cached-at', 'cached_at');
        }

        // 4. General Session State Store
        if (!db.objectStoreNames.contains('session_state')) {
          db.createObjectStore('session_state', { keyPath: 'session_code' });
        }
      },
    });
  }
  return dbPromise;
}

// ---------------------------------------------------------------------------
// Automatic 30-Day Cache Pruning
// ---------------------------------------------------------------------------

export async function pruneOldCache(): Promise<number> {
  try {
    const db = await getDB();
    const cutoff = Date.now() - THIRTY_DAYS_MS;
    let prunedCount = 0;

    // Prune old archive items
    const tx = db.transaction('archive', 'readwrite');
    const index = tx.store.index('by-cached-at');
    let cursor = await index.openCursor(IDBKeyRange.upperBound(cutoff));
    while (cursor) {
      await cursor.delete();
      prunedCount++;
      cursor = await cursor.continue();
    }
    await tx.done;

    return prunedCount;
  } catch {
    return 0;
  }
}

// ---------------------------------------------------------------------------
// Helpers: Captions Local Cache
// ---------------------------------------------------------------------------

export async function cacheCaptions(
  sessionCode: string,
  captions: ApiCaption[]
): Promise<void> {
  try {
    const db = await getDB();
    const tx = db.transaction('captions', 'readwrite');
    const now = Date.now();
    for (const caption of captions) {
      const ts = caption.created_at
        ? new Date(caption.created_at).getTime()
        : now;
      await tx.store.put({
        ...caption,
        session_code: sessionCode.toUpperCase(),
        timestamp: ts,
        cached_at: now,
      });
    }
    await tx.done;
  } catch {}
}

export async function getCachedCaptions(
  sessionCode: string
): Promise<ApiCaption[]> {
  try {
    const db = await getDB();
    const index = db.transaction('captions').store.index('by-session');
    const items = await index.getAll(sessionCode.toUpperCase());
    return items.sort((a, b) => {
      const aTime = a.created_at
        ? new Date(a.created_at).getTime()
        : a.timestamp;
      const bTime = b.created_at
        ? new Date(b.created_at).getTime()
        : b.timestamp;
      return aTime - bTime;
    });
  } catch {
    return [];
  }
}

// ---------------------------------------------------------------------------
// Helpers: Courses Local Cache
// ---------------------------------------------------------------------------

export async function cacheCourses(courses: Course[]): Promise<void> {
  try {
    const db = await getDB();
    const tx = db.transaction('courses', 'readwrite');
    const now = Date.now();
    for (const course of courses) {
      await tx.store.put({ ...course, updated_at: now });
    }
    await tx.done;
  } catch {}
}

export async function getCachedCourses(): Promise<Course[]> {
  try {
    const db = await getDB();
    const items = await db.getAll('courses');
    return items;
  } catch {
    return [];
  }
}

// ---------------------------------------------------------------------------
// Helpers: Archive & Resources Local Cache
// ---------------------------------------------------------------------------

export async function cacheResource(
  resource: Resource,
  sessionCode?: string
): Promise<void> {
  try {
    const db = await getDB();
    await db.put('archive', {
      ...resource,
      session_code: sessionCode?.toUpperCase(),
      cached_at: Date.now(),
    });
  } catch {}
}

export async function getCachedResource(
  resourceId: string
): Promise<Resource | undefined> {
  try {
    const db = await getDB();
    return await db.get('archive', resourceId);
  } catch {
    return undefined;
  }
}
