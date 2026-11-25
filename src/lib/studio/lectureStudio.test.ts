import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('$lib/api/aiStudy', () => ({
  fetchSessionTranscript: vi.fn(),
  fetchSessionChapters: vi.fn(),
  fetchRecordings: vi.fn(),
  fetchRecordingBlob: vi.fn(),
  generateSessionChapters: vi.fn(),
  runStudyJob: vi.fn(),
}));
vi.mock('$lib/native/wakelock', () => ({
  requestWakeLock: vi.fn(),
  releaseWakeLock: vi.fn(),
}));

import * as api from '$lib/api/aiStudy';
import { LectureStudio } from './lectureStudio.svelte';
import { getResume } from './resume';

const lines = [
  { offset_sec: 0, text: 'Intro' },
  { offset_sec: 30, text: 'Light reactions' },
  { offset_sec: 100, text: 'Calvin cycle' },
  { offset_sec: 200, text: 'Recap' },
];
const chapters = [
  {
    id: 'c1',
    chapter_index: 0,
    title: 'Intro',
    summary: '',
    start_timestamp_sec: 0,
    end_timestamp_sec: 29,
  },
  {
    id: 'c2',
    chapter_index: 1,
    title: 'Light',
    summary: '',
    start_timestamp_sec: 30,
    end_timestamp_sec: 99,
  },
  {
    id: 'c3',
    chapter_index: 2,
    title: 'Calvin',
    summary: '',
    start_timestamp_sec: 100,
    end_timestamp_sec: 300,
  },
];

async function open(
  opts: {
    startAt?: number | null;
    recordings?: unknown[];
    noChapters?: boolean;
  } = {}
) {
  vi.mocked(api.fetchSessionTranscript).mockResolvedValue({
    session_id: 's',
    source: 'captions',
    duration_sec: 300,
    lines,
  });
  vi.mocked(api.fetchSessionChapters).mockResolvedValue(
    opts.noChapters ? [] : [...chapters].reverse() // out of order on purpose
  );
  vi.mocked(api.fetchRecordings).mockResolvedValue(
    (opts.recordings ?? []) as never
  );
  const studio = new LectureStudio({
    sessionKey: 'sess',
    lectureId: 'lec-1',
    title: 'Photosynthesis',
    startAt: opts.startAt,
  });
  const dispose = studio.init();
  // Let the three loads settle.
  await vi.waitFor(() => {
    expect(studio.transcriptStatus).toBe('ready');
    expect(studio.chaptersStatus).not.toBe('loading');
    expect(studio.recordingsStatus).toBe('ready');
  });
  return { studio, dispose };
}

beforeEach(() => {
  localStorage.clear();
  vi.clearAllMocks();
});

describe('loading', () => {
  it('loads and sorts chapters', async () => {
    const { studio, dispose } = await open();
    expect(studio.chapters.map((c) => c.id)).toEqual(['c1', 'c2', 'c3']);
    dispose();
  });

  it('reports a transcript failure with the reason', async () => {
    vi.mocked(api.fetchSessionTranscript).mockRejectedValue(
      new Error('denied')
    );
    vi.mocked(api.fetchSessionChapters).mockResolvedValue([]);
    vi.mocked(api.fetchRecordings).mockResolvedValue([]);
    const studio = new LectureStudio({
      sessionKey: 's',
      lectureId: 'l',
      title: 't',
    });
    const dispose = studio.init();
    await vi.waitFor(() => expect(studio.transcriptStatus).toBe('error'));
    expect(studio.transcriptError).toBe('denied');
    expect(studio.lines).toEqual([]);
    dispose();
  });

  it('builds chapters once, only when there is a transcript and none exist', async () => {
    vi.mocked(api.runStudyJob).mockImplementation(async (start) => {
      await start();
      return undefined as never;
    });
    vi.mocked(api.generateSessionChapters).mockResolvedValue({} as never);
    const { studio, dispose } = await open({ noChapters: true });
    await vi.waitFor(() => expect(api.runStudyJob).toHaveBeenCalledTimes(1));
    dispose();
  });
});

describe('timeline', () => {
  it('has no active line or chapter highlight until engaged', async () => {
    const { studio, dispose } = await open();
    expect(studio.activeLineIndex).toBe(-1);
    studio.restore(0);
    expect(studio.activeLineIndex).toBe(0);
    dispose();
  });

  it('follows the playhead through lines and chapters', async () => {
    const { studio, dispose } = await open();
    studio.restore(45);
    expect(studio.activeLineIndex).toBe(1);
    expect(studio.activeChapter?.id).toBe('c2');
    studio.restore(100);
    expect(studio.activeLineIndex).toBe(2);
    expect(studio.activeChapterIndex).toBe(2);
    dispose();
  });

  it('reports progress through the active chapter', async () => {
    const { studio, dispose } = await open();
    studio.restore(65); // halfway through 30..99
    expect(studio.chapterProgress).toBeCloseTo(0.5, 1);
    dispose();
  });

  it('falls back to the transcript for duration when there is no audio', async () => {
    const { studio, dispose } = await open();
    expect(studio.hasAudio).toBe(false);
    expect(studio.duration).toBe(300);
    dispose();
  });

  it('uses recording lengths for duration when there is audio', async () => {
    const { studio, dispose } = await open({
      recordings: [
        {
          resource_id: 'r1',
          content_type: 'audio/wav',
          byte_size: 1,
          duration_sec: 200,
          created_at: '',
        },
        {
          resource_id: 'r2',
          content_type: 'audio/wav',
          byte_size: 1,
          duration_sec: 100,
          created_at: '',
        },
      ],
    });
    expect(studio.hasAudio).toBe(true);
    expect(studio.duration).toBe(300);
    dispose();
  });

  it('seeks without audio, clamped to the lecture, and bumps seekTick', async () => {
    const { studio, dispose } = await open();
    const tick = studio.seekTick;
    await studio.seek(9999);
    expect(studio.time).toBe(300);
    await studio.seek(-5);
    expect(studio.time).toBe(0);
    expect(studio.seekTick).toBe(tick + 2);
    expect(studio.engaged).toBe(true);
    dispose();
  });

  it('skips by an offset', async () => {
    const { studio, dispose } = await open();
    await studio.seek(50);
    studio.skip(-15);
    expect(studio.time).toBe(35);
    dispose();
  });
});

describe('chapter navigation', () => {
  it('goes to the next and previous chapter', async () => {
    const { studio, dispose } = await open();
    studio.restore(45); // chapter 2
    studio.seekChapter(1);
    expect(studio.time).toBe(100);
    studio.restore(150);
    studio.seekChapter(-1); // far into chapter 3 -> its own start
    expect(studio.time).toBe(100);
    dispose();
  });

  it('goes to the previous chapter when just after a chapter start', async () => {
    const { studio, dispose } = await open();
    studio.restore(101);
    studio.seekChapter(-1);
    expect(studio.time).toBe(30);
    dispose();
  });

  it('stays within bounds', async () => {
    const { studio, dispose } = await open();
    studio.restore(150);
    studio.seekChapter(1);
    expect(studio.time).toBe(100);
    studio.restore(2);
    studio.seekChapter(-1);
    expect(studio.time).toBe(0);
    dispose();
  });
});

describe('speed', () => {
  it('cycles through the speeds and wraps', async () => {
    const { studio, dispose } = await open();
    expect(studio.rate).toBe(1);
    studio.cycleRate(1);
    expect(studio.rate).toBe(1.25);
    studio.setRate(2);
    studio.cycleRate(1);
    expect(studio.rate).toBe(0.75);
    studio.cycleRate(-1);
    expect(studio.rate).toBe(2);
    dispose();
  });
});

describe('notes', () => {
  it('appends a timestamped bullet under a title on first use', async () => {
    const { studio, dispose } = await open();
    studio.addNote('ATP stores  energy', 78);
    expect(studio.notes).toBe(
      '# Study notes: Photosynthesis\n\n- [01:18] ATP stores energy\n'
    );
    studio.addNote('Second', null);
    expect(
      studio.notes.endsWith('- [01:18] ATP stores energy\n- Second\n')
    ).toBe(true);
    dispose();
  });

  it('ignores blank notes', async () => {
    const { studio, dispose } = await open();
    studio.addNote('   ', 5);
    expect(studio.notes).toBe('');
    dispose();
  });

  it('persists notes under the legacy key and reloads them', async () => {
    const first = await open();
    first.studio.setNotes('hello');
    first.dispose(); // flushes
    expect(localStorage.getItem('klasync-notes-lec-1')).toBe('hello');
    const second = await open();
    expect(second.studio.notes).toBe('hello');
    second.dispose();
  });
});

describe('saved moments', () => {
  it('toggles a bookmark, keeping them sorted', async () => {
    const { studio, dispose } = await open();
    studio.toggleBookmark(100, 'Calvin');
    studio.toggleBookmark(30, 'Light');
    expect(studio.bookmarks.map((b) => b.t)).toEqual([30, 100]);
    expect(studio.hasBookmarkAt(30.4)).toBe(true);
    studio.toggleBookmark(30);
    expect(studio.bookmarks.map((b) => b.t)).toEqual([100]);
    dispose();
  });

  it('labels a bookmark with the active chapter when no label is given', async () => {
    const { studio, dispose } = await open();
    studio.restore(45);
    studio.toggleBookmark();
    expect(studio.bookmarks[0].label).toBe('Light');
    dispose();
  });

  it('persists bookmarks', async () => {
    const a = await open();
    a.studio.toggleBookmark(30, 'x');
    a.dispose();
    const b = await open();
    expect(b.studio.bookmarks).toEqual([{ t: 30, label: 'x' }]);
    b.dispose();
  });
});

describe('resume and open-at', () => {
  it('opens at the requested moment', async () => {
    const { studio, dispose } = await open({ startAt: 120 });
    expect(studio.time).toBe(120);
    expect(studio.engaged).toBe(true);
    expect(studio.activeChapter?.id).toBe('c3');
    dispose();
  });

  it('opens at 0 when asked to start over, even if a position was saved', async () => {
    localStorage.setItem(
      'klasync-resume-v1',
      JSON.stringify({ 'lec-1': { t: 150, d: 300, at: 1 } })
    );
    const { studio, dispose } = await open({ startAt: 0 });
    expect(studio.time).toBe(0);
    dispose();
  });

  it('resumes a saved position by default', async () => {
    localStorage.setItem(
      'klasync-resume-v1',
      JSON.stringify({ 'lec-1': { t: 150, d: 300, at: 1 } })
    );
    const { studio, dispose } = await open();
    expect(studio.time).toBe(150);
    dispose();
  });

  it('saves the position when closed, and not if never engaged', async () => {
    const idle = await open();
    idle.dispose();
    expect(getResume('lec-1')).toBeNull();
    const used = await open();
    await used.studio.seek(120);
    used.dispose();
    expect(getResume('lec-1')?.t).toBe(120);
  });
});

describe('announcements', () => {
  it('announces and clears', async () => {
    vi.useFakeTimers();
    const { studio, dispose } = await open().catch(() => {
      throw new Error('open failed under fake timers');
    });
    studio.announce('Hello');
    expect(studio.announcement).toBe('Hello');
    vi.advanceTimersByTime(3000);
    expect(studio.announcement).toBe('');
    vi.useRealTimers();
    dispose();
  });
});
