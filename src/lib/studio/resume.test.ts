import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  clearResume,
  getResume,
  isResumable,
  lastResumed,
  progressOf,
  saveResume,
} from './resume';

beforeEach(() => localStorage.clear());

describe('isResumable', () => {
  it('ignores the very start and the very end', () => {
    expect(isResumable({ t: 2, d: 100 })).toBe(false);
    expect(isResumable({ t: 96, d: 100 })).toBe(false);
    expect(isResumable({ t: 50, d: 100 })).toBe(true);
  });
  it('treats an unknown length as resumable once past the start', () => {
    expect(isResumable({ t: 30, d: 0 })).toBe(true);
  });
});

describe('save / get / clear', () => {
  it('round-trips a position (floored)', () => {
    saveResume('a', 42.9, 480.4);
    expect(getResume('a')).toMatchObject({ t: 42, d: 480 });
  });
  it('removes the entry when the lecture is finished or barely started', () => {
    saveResume('a', 50, 100);
    saveResume('a', 99, 100);
    expect(getResume('a')).toBeNull();
    saveResume('a', 50, 100);
    saveResume('a', 1, 100);
    expect(getResume('a')).toBeNull();
  });
  it('clears one lecture without touching others', () => {
    saveResume('a', 50, 500);
    saveResume('b', 60, 500);
    clearResume('a');
    expect(getResume('a')).toBeNull();
    expect(getResume('b')).not.toBeNull();
  });
  it('ignores an empty id', () => {
    saveResume('', 50, 500);
    expect(localStorage.getItem('klasync-resume-v1')).toBeNull();
  });
  it('survives corrupt storage', () => {
    localStorage.setItem('klasync-resume-v1', '{not json');
    expect(getResume('a')).toBeNull();
    expect(lastResumed()).toBeNull();
    expect(() => saveResume('a', 50, 500)).not.toThrow();
    expect(getResume('a')).not.toBeNull();
  });
});

describe('lastResumed', () => {
  it('returns the most recently saved resumable lecture', () => {
    const now = vi.spyOn(Date, 'now');
    now.mockReturnValue(1000);
    saveResume('old', 50, 500);
    now.mockReturnValue(2000);
    saveResume('new', 60, 500);
    expect(lastResumed()?.id).toBe('new');
  });
  it('is null when nothing is in progress', () => {
    expect(lastResumed()).toBeNull();
  });
});

describe('progressOf', () => {
  it('is a 0-1 fraction, or null when unknown', () => {
    expect(progressOf({ t: 50, d: 200, at: 0 })).toBe(0.25);
    expect(progressOf({ t: 50, d: 0, at: 0 })).toBeNull();
    expect(progressOf(null)).toBeNull();
    expect(progressOf({ t: 500, d: 200, at: 0 })).toBe(1);
  });
});
