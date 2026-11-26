import { describe, expect, it } from 'vitest';
import { clock, parseClock, spokenClock } from './time';

describe('clock', () => {
  it('formats mm:ss and pads', () => {
    expect(clock(0)).toBe('00:00');
    expect(clock(65)).toBe('01:05');
    expect(clock(599)).toBe('09:59');
  });
  it('switches to h:mm:ss past an hour', () => {
    expect(clock(3600)).toBe('1:00:00');
    expect(clock(3723)).toBe('1:02:03');
  });
  it('floors fractions and clamps negatives', () => {
    expect(clock(61.9)).toBe('01:01');
    expect(clock(-5)).toBe('00:00');
  });
  it('shows a placeholder for missing or invalid values', () => {
    expect(clock(null)).toBe('--:--');
    expect(clock(undefined)).toBe('--:--');
    expect(clock(Number.NaN)).toBe('--:--');
    expect(clock(Infinity)).toBe('--:--');
  });
});

describe('parseClock', () => {
  it('parses mm:ss and h:mm:ss', () => {
    expect(parseClock('01:05')).toBe(65);
    expect(parseClock('1:02:03')).toBe(3723);
    expect(parseClock(' 0:09 ')).toBe(9);
  });
  it('round-trips with clock', () => {
    for (const s of [0, 7, 65, 599, 3600, 3723]) {
      expect(parseClock(clock(s))).toBe(s);
    }
  });
  it('rejects things that are not timestamps', () => {
    expect(parseClock('abc')).toBeNull();
    expect(parseClock('12')).toBeNull();
    expect(parseClock('1:2:3:4')).toBeNull();
    expect(parseClock('1:xx')).toBeNull();
    expect(parseClock('123:45')).toBeNull();
  });
});

describe('spokenClock', () => {
  it('speaks minutes and seconds with correct plurals', () => {
    expect(spokenClock(0)).toBe('0 seconds');
    expect(spokenClock(1)).toBe('1 second');
    expect(spokenClock(60)).toBe('1 minute');
    expect(spokenClock(61)).toBe('1 minute 1 second');
    expect(spokenClock(125)).toBe('2 minutes 5 seconds');
  });
});
