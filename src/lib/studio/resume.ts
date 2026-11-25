/**
 * Where the student stopped in each lecture, kept on this device so the
 * archive can offer "Continue from 03:12".
 */
const KEY = 'klasync-resume-v1';

export type Resume = {
  /** Playhead position, seconds. */
  t: number;
  /** Lecture length, seconds (0 when unknown). */
  d: number;
  /** When it was saved (ms since epoch). */
  at: number;
};

type Store = Record<string, Resume>;

function read(): Store {
  try {
    const raw = localStorage.getItem(KEY);
    const parsed = raw ? JSON.parse(raw) : {};
    return parsed && typeof parsed === 'object' ? (parsed as Store) : {};
  } catch {
    return {};
  }
}

function write(store: Store) {
  try {
    localStorage.setItem(KEY, JSON.stringify(store));
  } catch {
    /* storage unavailable or full */
  }
}

/** Ignore the very start and the very end: neither is worth resuming. */
export const isResumable = (r: Pick<Resume, 't' | 'd'>) =>
  r.t >= 5 && (r.d <= 0 || r.t <= r.d - 5);

export function getResume(id: string): Resume | null {
  const r = read()[id];
  return r && Number.isFinite(r.t) && isResumable(r) ? r : null;
}

export function saveResume(id: string, t: number, d: number) {
  if (!id) return;
  const store = read();
  const next = { t: Math.floor(t), d: Math.floor(d), at: Date.now() };
  if (isResumable(next)) store[id] = next;
  else delete store[id]; // finished or barely started
  write(store);
}

export function clearResume(id: string) {
  const store = read();
  if (id in store) {
    delete store[id];
    write(store);
  }
}

/** The most recently saved lecture that is still resumable. */
export function lastResumed(): ({ id: string } & Resume) | null {
  let best: ({ id: string } & Resume) | null = null;
  for (const [id, r] of Object.entries(read())) {
    if (!r || !isResumable(r)) continue;
    if (!best || r.at > best.at) best = { id, ...r };
  }
  return best;
}

/** 0‒1 progress, or null when unknown. */
export const progressOf = (r: Resume | null) =>
  r && r.d > 0 ? Math.min(Math.max(r.t / r.d, 0), 1) : null;
