import { getContext, setContext } from 'svelte';
import {
  fetchRecordingBlob,
  fetchRecordings,
  fetchSessionChapters,
  fetchSessionTranscript,
  generateSessionChapters,
  runStudyJob,
  type RecordingInfo,
  type SessionChapter,
  type TranscriptLine,
} from '$lib/api/aiStudy';
import { requestWakeLock, releaseWakeLock } from '$lib/native/wakelock';
import { clock } from './time';

export const SPEEDS = [0.75, 1, 1.25, 1.5, 1.75, 2] as const;
export type Bookmark = { t: number; label: string };

const KEY = Symbol('lecture-studio');
export const provideStudio = (studio: LectureStudio) => setContext(KEY, studio);
export const useStudio = () => getContext<LectureStudio>(KEY);

const readJSON = <T>(key: string, fallback: T): T => {
  try {
    const raw = localStorage.getItem(key);
    return raw ? (JSON.parse(raw) as T) : fallback;
  } catch {
    return fallback;
  }
};
const writeStorage = (key: string, value: string) => {
  try {
    localStorage.setItem(key, value);
    return true;
  } catch {
    return false;
  }
};

/**
 * One lecture, one timeline. Owns the playhead and everything that follows it
 * (transcript line, chapter, notes stamps) so every tool in the workspace
 * reads the same moment instead of keeping its own state.
 */
export class LectureStudio {
  readonly sessionKey: string;
  readonly lectureId: string;
  readonly title: string;

  // ── Content ───────────────────────────────────────────────
  lines = $state<TranscriptLine[]>([]);
  transcriptStatus = $state<'loading' | 'ready' | 'error'>('loading');
  transcriptError = $state('');
  transcriptSource = $state<'captions' | 'transcript' | 'none'>('none');
  transcriptDuration = $state(0);

  chapters = $state<SessionChapter[]>([]);
  chaptersStatus = $state<'loading' | 'ready' | 'generating' | 'error'>(
    'loading'
  );
  chaptersError = $state('');

  // ── Playhead ──────────────────────────────────────────────
  /** Global position in seconds across all recording parts. */
  time = $state(0);
  playing = $state(false);
  rate = $state<number>(1);
  /** True once the student has played or sought; gates "now playing" styling. */
  engaged = $state(false);
  /** Bumps on every explicit seek so views can re-enable auto-follow. */
  seekTick = $state(0);
  audioLoading = $state(false);
  audioError = $state('');
  recordingsStatus = $state<'loading' | 'ready' | 'error'>('loading');

  // ── Student content ───────────────────────────────────────
  notes = $state('');
  notesSaved = $state(true);
  bookmarks = $state<Bookmark[]>([]);
  /** Set by the `N` shortcut; the notes pane consumes it once mounted. */
  pendingStamp = $state(false);
  /** Set by the `/` shortcut; the ask pane focuses its composer once mounted. */
  focusAsk = $state(false);
  askDraft = $state('');
  flashcardSeed = $state('');
  announcement = $state('');

  // ── Private audio plumbing ────────────────────────────────
  #recordings = $state<RecordingInfo[]>([]);
  #segDurations = $state<number[]>([]);
  #el: HTMLAudioElement | null = null;
  #objectUrl = '';
  #loadedIdx = -1;
  #loading: Promise<void> | null = null;
  #raf = 0;
  #notesTimer: ReturnType<typeof setTimeout> | undefined;
  #announceTimer: ReturnType<typeof setTimeout> | undefined;
  #destroyed = false;
  #chaptersRequested = false;

  constructor(opts: { sessionKey: string; lectureId: string; title: string }) {
    this.sessionKey = opts.sessionKey;
    this.lectureId = opts.lectureId;
    this.title = opts.title;
  }

  // ── Derived state ─────────────────────────────────────────
  hasAudio = $derived(this.#recordings.length > 0);

  /** Seconds at which each recording part starts on the global timeline. */
  #offsets = $derived.by(() => {
    const out: number[] = [];
    let acc = 0;
    for (let i = 0; i < this.#recordings.length; i++) {
      out.push(acc);
      acc += this.#segDurations[i] || this.#recordings[i].duration_sec || 0;
    }
    return out;
  });

  duration = $derived.by(() => {
    const fromAudio = this.#recordings.reduce(
      (sum, rec, i) => sum + (this.#segDurations[i] || rec.duration_sec || 0),
      0
    );
    if (fromAudio > 0) return fromAudio;
    if (this.transcriptDuration > 0) return this.transcriptDuration;
    let lastOffset = 0;
    for (const l of this.lines)
      if (l.offset_sec != null && l.offset_sec > lastOffset)
        lastOffset = l.offset_sec;
    const lastChapter = this.chapters.at(-1)?.end_timestamp_sec ?? 0;
    return Math.max(lastOffset + 30, lastChapter);
  });

  /** Whether any transcript line carries a timestamp we can follow. */
  isSynced = $derived(this.lines.some((l) => l.offset_sec != null));

  activeLineIndex = $derived.by(() => {
    if (!this.engaged) return -1;
    let found = -1;
    // Lines are chronological; scan from the end for the last started line.
    for (let i = this.lines.length - 1; i >= 0; i--) {
      const t = this.lines[i].offset_sec;
      if (t != null && t <= this.time + 0.25) {
        found = i;
        break;
      }
    }
    return found;
  });

  activeChapterIndex = $derived.by(() => {
    let found = -1;
    for (let i = 0; i < this.chapters.length; i++) {
      if (this.chapters[i].start_timestamp_sec <= this.time + 0.25) found = i;
    }
    return found;
  });

  activeChapter = $derived(
    this.activeChapterIndex >= 0 ? this.chapters[this.activeChapterIndex] : null
  );

  /** 0‒1 progress through the active chapter. */
  chapterProgress = $derived.by(() => {
    const ch = this.activeChapter;
    if (!ch) return 0;
    const span = Math.max(ch.end_timestamp_sec - ch.start_timestamp_sec, 1);
    return Math.min(
      Math.max((this.time - ch.start_timestamp_sec) / span, 0),
      1
    );
  });

  // ── Lifecycle ─────────────────────────────────────────────
  init() {
    this.#destroyed = false;
    this.#loadNotesAndBookmarks();
    void this.loadTranscript();
    void this.loadChapters();
    void this.loadRecordings();
    this.#bindMediaSession();
    return () => this.destroy();
  }

  destroy() {
    this.#destroyed = true;
    cancelAnimationFrame(this.#raf);
    clearTimeout(this.#announceTimer);
    this.#flushNotes();
    if (this.#el) {
      this.#el.pause();
      this.#el.removeAttribute('src');
      this.#el.load();
      this.#el = null;
    }
    if (this.#objectUrl) URL.revokeObjectURL(this.#objectUrl);
    this.#objectUrl = '';
    void releaseWakeLock();
    if ('mediaSession' in navigator) {
      for (const a of [
        'play',
        'pause',
        'seekbackward',
        'seekforward',
        'previoustrack',
        'nexttrack',
      ] as MediaSessionAction[]) {
        try {
          navigator.mediaSession.setActionHandler(a, null);
        } catch {
          /* unsupported action */
        }
      }
    }
  }

  // ── Content loading ───────────────────────────────────────
  async loadTranscript() {
    this.transcriptStatus = 'loading';
    this.transcriptError = '';
    try {
      const res = await fetchSessionTranscript(this.sessionKey);
      if (this.#destroyed) return;
      this.lines = res.lines;
      this.transcriptSource = res.source;
      this.transcriptDuration = res.duration_sec || 0;
      this.transcriptStatus = 'ready';
    } catch (error) {
      if (this.#destroyed) return;
      this.lines = [];
      this.transcriptError =
        error instanceof Error
          ? error.message
          : 'Unable to load the transcript.';
      this.transcriptStatus = 'error';
    }
    // Chapters are built from the transcript, so offer to generate once we
    // know there is something to build them from.
    void this.#maybeAutoGenerateChapters();
  }

  async loadChapters() {
    this.chaptersStatus = 'loading';
    this.chaptersError = '';
    try {
      const list = await fetchSessionChapters(this.sessionKey);
      if (this.#destroyed) return;
      this.chapters = [...list].sort(
        (a, b) => a.start_timestamp_sec - b.start_timestamp_sec
      );
      this.chaptersStatus = 'ready';
    } catch (error) {
      if (this.#destroyed) return;
      this.chaptersError =
        error instanceof Error ? error.message : 'Unable to load chapters.';
      this.chaptersStatus = 'error';
    }
    void this.#maybeAutoGenerateChapters();
  }

  async generateChapters() {
    if (this.chaptersStatus === 'generating') return;
    this.chaptersStatus = 'generating';
    this.chaptersError = '';
    try {
      await runStudyJob(() => generateSessionChapters(this.sessionKey));
      const list = await fetchSessionChapters(this.sessionKey);
      if (this.#destroyed) return;
      this.chapters = [...list].sort(
        (a, b) => a.start_timestamp_sec - b.start_timestamp_sec
      );
      this.chaptersStatus = 'ready';
    } catch (error) {
      if (this.#destroyed) return;
      this.chaptersError =
        error instanceof Error ? error.message : 'Chapter generation failed.';
      this.chaptersStatus = 'error';
    }
  }

  async #maybeAutoGenerateChapters() {
    if (
      this.#chaptersRequested ||
      this.chaptersStatus !== 'ready' ||
      this.transcriptStatus !== 'ready' ||
      this.chapters.length > 0 ||
      this.lines.length === 0
    )
      return;
    // Same behaviour as the old Chapters tab: build them the first time a
    // lecture is opened, but only when there is a transcript to build from.
    this.#chaptersRequested = true;
    await this.generateChapters();
  }

  async loadRecordings() {
    this.recordingsStatus = 'loading';
    try {
      const list = await fetchRecordings(this.sessionKey);
      if (this.#destroyed) return;
      this.#recordings = list;
      this.#segDurations = list.map((r) => r.duration_sec ?? 0);
      this.recordingsStatus = 'ready';
    } catch (error) {
      if (this.#destroyed) return;
      this.recordingsStatus = 'error';
      this.audioError =
        error instanceof Error
          ? error.message
          : 'Unable to look up the recording.';
    }
  }

  // ── Playback ──────────────────────────────────────────────
  #segIndexFor(t: number): number {
    const offs = this.#offsets;
    let idx = 0;
    for (let i = 0; i < offs.length; i++) if (offs[i] <= t) idx = i;
    return idx;
  }

  #ensureElement(): HTMLAudioElement {
    if (this.#el) return this.#el;
    const el = new Audio();
    el.preload = 'metadata';
    el.addEventListener('play', () => {
      this.playing = true;
      this.#startTicker();
      void requestWakeLock();
      this.#syncMediaState();
    });
    el.addEventListener('pause', () => {
      this.playing = false;
      this.#syncTime();
      void releaseWakeLock();
      this.#syncMediaState();
    });
    el.addEventListener('timeupdate', () => this.#syncTime());
    el.addEventListener('seeked', () => this.#syncTime());
    el.addEventListener('loadedmetadata', () => {
      if (Number.isFinite(el.duration) && this.#loadedIdx >= 0)
        this.#segDurations[this.#loadedIdx] = el.duration;
    });
    el.addEventListener('ended', () => void this.#handleEnded());
    el.addEventListener('error', () => {
      if (this.#objectUrl)
        this.audioError =
          'This recording format cannot be played on this device.';
    });
    el.playbackRate = this.rate;
    this.#el = el;
    return el;
  }

  #syncTime() {
    if (!this.#el || this.#loadedIdx < 0) return;
    this.time = (this.#offsets[this.#loadedIdx] ?? 0) + this.#el.currentTime;
  }

  #startTicker() {
    cancelAnimationFrame(this.#raf);
    const tick = () => {
      if (this.#destroyed || !this.playing) return;
      this.#syncTime();
      this.#raf = requestAnimationFrame(tick);
    };
    this.#raf = requestAnimationFrame(tick);
  }

  /**
   * `<audio src>` cannot send the student's bearer token, so each part is
   * downloaded through the authenticated client and played from an object
   * URL — fetched lazily, the first time the student actually plays.
   */
  async #load(idx: number) {
    if (idx === this.#loadedIdx && this.#objectUrl) return;
    if (this.#loading) return this.#loading;
    const rec = this.#recordings[idx];
    if (!rec) return;
    this.audioLoading = true;
    this.audioError = '';
    this.#loading = (async () => {
      try {
        const blob = await fetchRecordingBlob(this.sessionKey, rec.resource_id);
        if (this.#destroyed) return;
        const el = this.#ensureElement();
        const previous = this.#objectUrl;
        this.#objectUrl = URL.createObjectURL(blob);
        this.#loadedIdx = idx;
        el.src = this.#objectUrl;
        el.playbackRate = this.rate;
        if (previous) URL.revokeObjectURL(previous);
        await new Promise<void>((resolve) => {
          const done = () => {
            el.removeEventListener('loadedmetadata', done);
            el.removeEventListener('error', done);
            resolve();
          };
          el.addEventListener('loadedmetadata', done);
          el.addEventListener('error', done);
        });
      } catch (error) {
        this.audioError =
          error instanceof Error
            ? error.message
            : 'The recording could not be loaded.';
      } finally {
        this.audioLoading = false;
        this.#loading = null;
      }
    })();
    return this.#loading;
  }

  async #handleEnded() {
    this.playing = false;
    void releaseWakeLock();
    const next = this.#loadedIdx + 1;
    if (next < this.#recordings.length) {
      await this.#load(next);
      const el = this.#el;
      if (el) {
        this.time = this.#offsets[next] ?? this.time;
        await el.play().catch(() => {});
      }
    } else {
      this.time = this.duration;
    }
  }

  async play() {
    if (!this.hasAudio || this.audioLoading) return;
    this.engaged = true;
    const idx = this.#segIndexFor(this.time);
    if (idx !== this.#loadedIdx || !this.#el) await this.#load(idx);
    const el = this.#el;
    if (!el || this.#destroyed || !this.#objectUrl) return;
    const local = this.time - (this.#offsets[idx] ?? 0);
    if (Math.abs(el.currentTime - local) > 0.4)
      el.currentTime = Math.max(local, 0);
    // Restart from the top if the lecture already finished.
    if (this.duration > 0 && this.time >= this.duration - 0.5) {
      this.time = 0;
      el.currentTime = 0;
    }
    el.playbackRate = this.rate;
    await el.play().catch(() => {
      this.audioError = 'Playback was blocked. Press play again.';
    });
  }

  pause() {
    this.#el?.pause();
  }

  toggle() {
    if (this.playing) this.pause();
    else void this.play();
  }

  /**
   * Moves the playhead. Works without audio too: the transcript, chapters and
   * notes still follow a "virtual" playhead so the workspace is useful for
   * lectures that were never recorded.
   */
  async seek(t: number, opts: { play?: boolean } = {}) {
    const max = this.duration || t;
    const target = Math.min(Math.max(t, 0), Math.max(max, 0));
    this.time = target;
    this.engaged = true;
    this.seekTick++;
    if (!this.hasAudio) return;
    const idx = this.#segIndexFor(target);
    if (idx === this.#loadedIdx && this.#el) {
      this.#el.currentTime = target - (this.#offsets[idx] ?? 0);
      if (opts.play && this.#el.paused) await this.play();
      return;
    }
    // Other part not downloaded yet: only fetch it if playback is wanted.
    if (opts.play || this.playing) await this.play();
  }

  skip(delta: number) {
    void this.seek(this.time + delta);
  }

  setRate(rate: number) {
    this.rate = rate;
    if (this.#el) this.#el.playbackRate = rate;
  }

  cycleRate(direction: 1 | -1 = 1) {
    const i = SPEEDS.findIndex((s) => s === this.rate);
    const next = SPEEDS[(i + direction + SPEEDS.length) % SPEEDS.length];
    this.setRate(next);
    this.announce(`Speed ${next}×`);
  }

  seekChapter(delta: 1 | -1) {
    if (!this.chapters.length) return;
    let idx = this.activeChapterIndex;
    // Pressing "previous" shortly after a chapter starts goes to the one before.
    if (delta < 0 && idx >= 0) {
      const into = this.time - this.chapters[idx].start_timestamp_sec;
      if (into <= 3) idx -= 1;
    } else {
      idx += delta;
    }
    idx = Math.min(Math.max(idx, 0), this.chapters.length - 1);
    void this.seek(this.chapters[idx].start_timestamp_sec);
    this.announce(`Chapter ${idx + 1}: ${this.chapters[idx].title}`);
  }

  // ── Media Session (lock screen / headset controls) ────────
  #bindMediaSession() {
    if (!('mediaSession' in navigator)) return;
    const set = (a: MediaSessionAction, fn: () => void) => {
      try {
        navigator.mediaSession.setActionHandler(a, fn);
      } catch {
        /* unsupported action */
      }
    };
    set('play', () => void this.play());
    set('pause', () => this.pause());
    set('seekbackward', () => this.skip(-15));
    set('seekforward', () => this.skip(15));
    set('previoustrack', () => this.seekChapter(-1));
    set('nexttrack', () => this.seekChapter(1));
    try {
      navigator.mediaSession.metadata = new MediaMetadata({
        title: this.title,
      });
    } catch {
      /* MediaMetadata unavailable */
    }
  }

  #syncMediaState() {
    if (!('mediaSession' in navigator)) return;
    navigator.mediaSession.playbackState = this.playing ? 'playing' : 'paused';
  }

  // ── Notes & moments ───────────────────────────────────────
  get #notesKey() {
    // Same key the old notes tab used, so existing notes carry over.
    return `klasync-notes-${this.lectureId}`;
  }
  get #bookmarksKey() {
    return `klasync-bookmarks-${this.lectureId}`;
  }

  #loadNotesAndBookmarks() {
    try {
      this.notes = localStorage.getItem(this.#notesKey) ?? '';
    } catch {
      this.notes = '';
    }
    this.bookmarks = readJSON<Bookmark[]>(this.#bookmarksKey, []).filter((b) =>
      Number.isFinite(b?.t)
    );
  }

  setNotes(value: string) {
    this.notes = value;
    this.notesSaved = false;
    clearTimeout(this.#notesTimer);
    this.#notesTimer = setTimeout(() => this.#flushNotes(), 500);
  }

  #flushNotes() {
    clearTimeout(this.#notesTimer);
    if (this.notesSaved) return;
    this.notesSaved = writeStorage(this.#notesKey, this.notes);
  }

  /** Appends a timestamped bullet, e.g. `- [01:18] ATP stores energy`. */
  addNote(text: string, seconds: number | null = this.time) {
    const clean = text.replace(/\s+/g, ' ').trim();
    if (!clean) return;
    const stamp = seconds == null ? '' : `[${clock(seconds)}] `;
    const base = this.notes.trimEnd();
    const header = base ? '' : `# Study notes: ${this.title}\n\n`;
    this.setNotes(`${header}${base ? base + '\n' : ''}- ${stamp}${clean}\n`);
    this.#flushNotes();
    this.announce(
      seconds == null ? 'Added to notes' : `Added to notes at ${clock(seconds)}`
    );
  }

  hasBookmarkAt(t: number) {
    return this.bookmarks.some((b) => Math.abs(b.t - t) < 1);
  }

  toggleBookmark(t: number = this.time, label = '') {
    const rounded = Math.round(t);
    if (this.hasBookmarkAt(rounded)) {
      this.bookmarks = this.bookmarks.filter(
        (b) => Math.abs(b.t - rounded) >= 1
      );
      this.announce(`Removed moment at ${clock(rounded)}`);
    } else {
      const fallback =
        this.activeChapter?.title ?? `Moment at ${clock(rounded)}`;
      this.bookmarks = [
        ...this.bookmarks,
        { t: rounded, label: label.trim().slice(0, 140) || fallback },
      ].sort((a, b) => a.t - b.t);
      this.announce(`Saved moment at ${clock(rounded)}`);
    }
    writeStorage(this.#bookmarksKey, JSON.stringify(this.bookmarks));
  }

  announce(message: string) {
    this.announcement = message;
    clearTimeout(this.#announceTimer);
    this.#announceTimer = setTimeout(() => (this.announcement = ''), 2600);
  }
}
