/**
 * IndexedDB persistence for assessment attempts. Falls back to memory when
 * IndexedDB is unavailable (private windows, tests), so taking an assessment
 * still works there, just without surviving a reload.
 */
import {
  MemoryAttemptStore,
  type AttemptStore,
  type StoredAttempt,
} from '../assessments/attemptEngine';
import { getDB } from './db';

const THIRTY_DAYS_MS = 30 * 24 * 60 * 60 * 1000;

const memory = new MemoryAttemptStore();

class IndexedDbAttemptStore implements AttemptStore {
  async load(id: string): Promise<StoredAttempt | undefined> {
    try {
      return await (await getDB()).get('assessment_attempts', id);
    } catch {
      return memory.load(id);
    }
  }

  async save(attempt: StoredAttempt): Promise<void> {
    try {
      await (await getDB()).put('assessment_attempts', attempt);
    } catch (error) {
      // Keep an in-memory copy so a storage error does not stop the attempt.
      await memory.save(attempt);
      throw error;
    }
  }

  async remove(id: string): Promise<void> {
    try {
      await (await getDB()).delete('assessment_attempts', id);
    } catch {
      await memory.remove(id);
    }
  }
}

export const attemptStore: AttemptStore = new IndexedDbAttemptStore();

/** Attempts saved on this device that have not been confirmed by the server. */
export async function listUnfinishedAttempts(): Promise<StoredAttempt[]> {
  try {
    const all = await (await getDB()).getAll('assessment_attempts');
    return all.filter((a) => a.phase !== 'submitted');
  } catch {
    return [];
  }
}

/** Removes confirmed attempts older than 30 days. Unconfirmed work is never pruned. */
export async function pruneConfirmedAttempts(): Promise<number> {
  try {
    const db = await getDB();
    const cutoff = Date.now() - THIRTY_DAYS_MS;
    let removed = 0;
    for (const a of await db.getAll('assessment_attempts')) {
      if (a.phase === 'submitted' && a.updatedAt < cutoff) {
        await db.delete('assessment_attempts', a.id);
        removed++;
      }
    }
    return removed;
  } catch {
    return 0;
  }
}
