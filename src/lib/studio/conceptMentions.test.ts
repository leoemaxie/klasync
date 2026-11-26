import { describe, expect, it } from 'vitest';
import {
  activeConcepts,
  findMentions,
  indexMentions,
  tourOrder,
} from './conceptMentions';

const lines = [
  { offset_sec: 0, text: 'Today we cover photosynthesis in the chloroplast.' },
  {
    offset_sec: 30,
    text: 'The light-dependent reactions happen in the thylakoid.',
  },
  { offset_sec: 80, text: 'Energy is stored as ATP and NADPH.' },
  { offset_sec: 160, text: 'RuBisCO drives the Calvin cycles, using ATP.' },
  { offset_sec: 190, text: 'PATPAT is not related; neither is ATP-free.' },
  { offset_sec: null, text: 'An untimed remark about the Calvin cycle.' },
];

describe('findMentions', () => {
  it('matches whole words only', () => {
    expect(findMentions('ATP', lines)).toEqual([2, 3]);
  });
  it('is case- and plural-insensitive', () => {
    expect(findMentions('calvin cycle', lines)).toEqual([3, 5]);
    expect(findMentions('Reaction', lines)).toEqual([1]);
  });
  it('ignores a parenthetical in the label', () => {
    expect(findMentions('Chloroplast (organelle)', lines)).toEqual([0]);
  });
  it('returns nothing for empty or one-character labels', () => {
    expect(findMentions('', lines)).toEqual([]);
    expect(findMentions('x', lines)).toEqual([]);
  });
  it('is safe with regex characters in the label', () => {
    expect(() =>
      findMentions('C++ (programming) [draft] $5', lines)
    ).not.toThrow();
    expect(findMentions('a.b*c', lines)).toEqual([]);
  });
});

const nodes = [
  { id: 'photo', label: 'Photosynthesis', group: 'core', importance: 5 },
  { id: 'atp', label: 'ATP', group: 'mol', importance: 3 },
  { id: 'calvin', label: 'Calvin cycle', group: 'stage', importance: 4 },
  { id: 'ghost', label: 'Quantum tunnelling', group: 'x', importance: 2 },
];

describe('indexMentions', () => {
  const m = indexMentions(nodes, lines);
  it('maps concepts to their lines', () => {
    expect(m.lines.get('atp')).toEqual([2, 3]);
    expect(m.lines.get('ghost')).toEqual([]);
  });
  it('records the first timed mention', () => {
    expect(m.first.get('photo')).toBe(0);
    expect(m.first.get('atp')).toBe(80);
    expect(m.first.has('ghost')).toBe(false);
  });
});

describe('activeConcepts', () => {
  const m = indexMentions(nodes, lines);
  it('returns concepts mentioned just before the playhead', () => {
    expect([...activeConcepts(m, lines, 85)]).toEqual(['atp']);
  });
  it('includes a mention a couple of seconds ahead', () => {
    expect(activeConcepts(m, lines, 78).has('atp')).toBe(true);
  });
  it('drops concepts once they fall out of the window', () => {
    expect(activeConcepts(m, lines, 120).has('atp')).toBe(false);
  });
  it('is empty when nothing is being discussed', () => {
    expect(activeConcepts(m, lines, 400).size).toBe(0);
  });
});

describe('tourOrder', () => {
  it('orders by first mention, then importance, unmentioned last', () => {
    const m = indexMentions(nodes, lines);
    expect(tourOrder(nodes, m)).toEqual(['photo', 'atp', 'calvin', 'ghost']);
  });
});
