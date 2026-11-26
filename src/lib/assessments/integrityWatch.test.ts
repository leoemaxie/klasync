import { describe, expect, it, vi } from 'vitest';
import { watchIntegrity } from './integrityWatch';

function target() {
  const events: string[] = [];
  return {
    events,
    recordEvent: vi.fn(async (kind: string) => {
      events.push(kind);
    }),
  };
}

describe('watchIntegrity', () => {
  it('records focus loss and a hidden tab when monitored', () => {
    const t = target();
    const stop = watchIntegrity(t, {
      settings: {
        records_integrity: true,
        block_paste: false,
        require_fullscreen: false,
      },
    });
    window.dispatchEvent(new Event('blur'));
    Object.defineProperty(document, 'visibilityState', {
      value: 'hidden',
      configurable: true,
    });
    document.dispatchEvent(new Event('visibilitychange'));
    Object.defineProperty(document, 'visibilityState', {
      value: 'visible',
      configurable: true,
    });
    document.dispatchEvent(new Event('visibilitychange'));
    expect(t.events).toEqual(['focus_lost', 'tab_hidden', 'focus_gained']);
    stop();
    window.dispatchEvent(new Event('blur'));
    expect(t.events).toHaveLength(3); // stopped
  });

  it('records nothing when the assessment does not monitor', () => {
    const t = target();
    const stop = watchIntegrity(t, {
      settings: {
        records_integrity: false,
        block_paste: false,
        require_fullscreen: false,
      },
    });
    window.dispatchEvent(new Event('blur'));
    expect(t.recordEvent).not.toHaveBeenCalled();
    stop();
  });

  it('blocks paste and drop when asked, and says so', () => {
    const t = target();
    const area = document.createElement('div');
    const onBlockedPaste = vi.fn();
    const stop = watchIntegrity(t, {
      settings: {
        records_integrity: true,
        block_paste: true,
        require_fullscreen: false,
      },
      area,
      onBlockedPaste,
    });
    const paste = new Event('paste', { cancelable: true });
    area.dispatchEvent(paste);
    const drop = new Event('drop', { cancelable: true });
    area.dispatchEvent(drop);
    expect(paste.defaultPrevented).toBe(true);
    expect(drop.defaultPrevented).toBe(true);
    expect(onBlockedPaste).toHaveBeenCalledTimes(2);
    expect(t.events).toEqual(['paste', 'paste']);
    stop();
  });

  it('records but allows paste when not blocked', () => {
    const t = target();
    const area = document.createElement('div');
    const stop = watchIntegrity(t, {
      settings: {
        records_integrity: true,
        block_paste: false,
        require_fullscreen: false,
      },
      area,
    });
    const paste = new Event('paste', { cancelable: true });
    area.dispatchEvent(paste);
    area.dispatchEvent(new Event('copy'));
    expect(paste.defaultPrevented).toBe(false);
    expect(t.events).toEqual(['paste', 'copy']);
    stop();
  });

  it('flags leaving fullscreen only when fullscreen is required', () => {
    const t = target();
    const stop = watchIntegrity(t, {
      settings: {
        records_integrity: true,
        block_paste: false,
        require_fullscreen: true,
      },
    });
    document.dispatchEvent(new Event('fullscreenchange'));
    expect(t.events).toEqual(['fullscreen_exit']);
    stop();
  });
});
