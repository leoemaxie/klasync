/**
 * Forwards browser signals to the attempt engine as integrity events. These
 * are flags for the lecturer to review, never penalties: the engine only
 * records them when the assessment monitors integrity, and nothing here
 * changes a score.
 */
import type { StudentSettings } from '$lib/api/assessments';

export interface WatchTarget {
  recordEvent(
    kind: string,
    meta?: Record<string, unknown>
  ): Promise<void> | void;
}

export interface WatchOptions {
  settings: Pick<
    StudentSettings,
    'records_integrity' | 'block_paste' | 'require_fullscreen'
  >;
  /** Element whose copy/paste is watched (the answer area). */
  area?: HTMLElement | null;
  /** Called when a paste was blocked, so the UI can explain why. */
  onBlockedPaste?: () => void;
  doc?: Document;
  win?: Window;
}

/** Starts watching; returns a function that stops. */
export function watchIntegrity(
  engine: WatchTarget,
  options: WatchOptions
): () => void {
  const doc = options.doc ?? document;
  const win = options.win ?? window;
  const { settings, area } = options;
  const cleanups: (() => void)[] = [];
  const listen = <T extends EventTarget>(
    target: T,
    type: string,
    handler: (event: Event) => void
  ) => {
    target.addEventListener(type, handler);
    cleanups.push(() => target.removeEventListener(type, handler));
  };

  const record = (kind: string, meta?: Record<string, unknown>) => {
    if (settings.records_integrity) void engine.recordEvent(kind, meta);
  };

  listen(doc, 'visibilitychange', () => {
    record(doc.visibilityState === 'hidden' ? 'tab_hidden' : 'focus_gained');
  });
  listen(win, 'blur', () => record('focus_lost'));
  listen(win, 'focus', () => record('focus_gained'));
  listen(doc, 'fullscreenchange', () => {
    if (!doc.fullscreenElement && settings.require_fullscreen) {
      record('fullscreen_exit');
    }
  });
  if (area) {
    listen(area, 'paste', (event) => {
      record('paste');
      if (settings.block_paste) {
        event.preventDefault();
        options.onBlockedPaste?.();
      }
    });
    listen(area, 'copy', () => record('copy'));
    if (settings.block_paste) {
      // Dropping text into an answer is a paste by another route.
      listen(area, 'drop', (event) => {
        event.preventDefault();
        record('paste', { via: 'drop' });
        options.onBlockedPaste?.();
      });
    }
  }
  return () => cleanups.forEach((fn) => fn());
}
