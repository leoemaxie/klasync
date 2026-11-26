import { describe, expect, it } from 'vitest';
import type { Block, MaterialDoc } from '$lib/api/materials';
import {
  checkLatex,
  formatTime,
  inlineHtml,
  insertBlock,
  makeBlock,
  moveBlock,
  moveSection,
  outline,
  removeBlock,
  sectionRange,
  setLocked,
  updateBlock,
  validateDoc,
} from './doc';

const base = {
  origin: 'lecture',
  locked: false,
  low_confidence: false,
} as const;
const h = (id: string, level: 1 | 2 | 3 | 4, text = id): Block => ({
  ...base,
  id,
  type: 'heading',
  level,
  text,
});
const p = (id: string, text = 'text'): Block => ({
  ...base,
  id,
  type: 'paragraph',
  text,
});
const doc = (blocks: Block[]): MaterialDoc => ({
  schema_version: 1,
  title: 'T',
  meta: { objectives: [], ai_drafted: true },
  blocks,
});
const ids = (d: MaterialDoc) => d.blocks.map((b) => b.id);

describe('inlineHtml', () => {
  it('escapes HTML before formatting', () => {
    expect(inlineHtml('<img onerror=x> **b**')).toBe(
      '&lt;img onerror=x&gt; <strong>b</strong>'
    );
  });
  it('handles code, bold and italic', () => {
    expect(inlineHtml('a `x<y` *i* **b**')).toBe(
      'a <code>x&lt;y</code> <em>i</em> <strong>b</strong>'
    );
  });
});

describe('formatTime', () => {
  it('formats minutes and hours', () => {
    expect(formatTime(65)).toBe('1:05');
    expect(formatTime(3725)).toBe('1:02:05');
    expect(formatTime(-3)).toBe('0:00');
  });
});

describe('block operations', () => {
  const d = doc([h('a', 2), p('a1'), p('a2'), h('b', 2), p('b1'), h('c', 3)]);

  it('inserts, removes and clamps', () => {
    expect(ids(insertBlock(d, 1, p('x')))).toEqual([
      'a',
      'x',
      'a1',
      'a2',
      'b',
      'b1',
      'c',
    ]);
    expect(ids(insertBlock(d, 99, p('x'))).at(-1)).toBe('x');
    expect(ids(removeBlock(d, 'a1'))).not.toContain('a1');
  });

  it('locks a block when it is edited', () => {
    const e = updateBlock(d, 'a1', { text: 'new' } as Partial<Block>);
    const b = e.blocks.find((x) => x.id === 'a1')!;
    expect(b.locked).toBe(true);
    expect((b as { text: string }).text).toBe('new');
    expect(setLocked(e, 'a1', false).blocks[1].locked).toBe(false);
  });

  it('moves single blocks and stops at the ends', () => {
    expect(ids(moveBlock(d, 'a1', 1)).slice(0, 3)).toEqual(['a', 'a2', 'a1']);
    expect(moveBlock(d, 'a', -1)).toBe(d);
  });

  it('finds section ranges including subsections', () => {
    const n = doc([h('a', 2), p('1'), h('s', 3), p('2'), h('b', 2)]);
    expect(sectionRange(n, 'a')).toEqual([0, 4]);
    expect(sectionRange(n, 's')).toEqual([2, 4]);
    expect(sectionRange(n, '1')).toBeNull();
  });

  it('moves whole sections', () => {
    // a(a1,a2) then b(b1,c): moving a down swaps it with the next level-2 section.
    expect(ids(moveSection(d, 'a', 1))).toEqual([
      'b',
      'b1',
      'c',
      'a',
      'a1',
      'a2',
    ]);
    expect(ids(moveSection(d, 'b', -1))).toEqual([
      'b',
      'b1',
      'c',
      'a',
      'a1',
      'a2',
    ]);
    expect(moveSection(d, 'a', -1)).toBe(d);
  });

  it('will not move a section past a different level', () => {
    const n = doc([h('a', 2), p('1'), h('s', 3)]);
    expect(moveSection(n, 's', -1)).toBe(n);
  });

  it('builds an outline', () => {
    expect(outline(d).map((o) => o.id)).toEqual(['a', 'b', 'c']);
  });

  it('adds lecturer blocks as locked and supplemented', () => {
    const b = makeBlock('paragraph');
    expect(b.locked).toBe(true);
    expect(b.origin).toBe('supplemented');
    expect(b.id).toMatch(/^b_/);
  });
});

describe('validation', () => {
  it('accepts a good document', () => {
    expect(validateDoc(doc([h('a', 2), p('1')]))).toEqual([]);
  });
  it('flags empty, duplicate and unclosed content', () => {
    const problems = validateDoc(
      doc([p('1', ''), p('1'), p('2', '```js\ncode')])
    ).map((x) => x.message);
    expect(problems).toContain('Empty text');
    expect(problems).toContain('Duplicate block id');
    expect(problems).toContain('Unclosed code fence in text');
  });
  it('flags bad titles, tables and equations', () => {
    const bad = doc([
      { ...base, id: 't', type: 'table', header: ['a', 'b'], rows: [['1']] },
      { ...base, id: 'e', type: 'equation', latex: '\\frac{1}{2' },
    ]);
    bad.title = ' ';
    const msgs = validateDoc(bad).map((x) => x.message);
    expect(msgs).toContain('The document needs a title');
    expect(msgs).toContain('Table has rows of the wrong width');
    expect(msgs.some((m) => m.startsWith('Equation does not parse'))).toBe(
      true
    );
  });
  it('requires content', () => {
    expect(validateDoc(doc([])).map((x) => x.message)).toContain(
      'The document has no content'
    );
  });
});

describe('checkLatex', () => {
  it('accepts balanced input and escaped braces', () => {
    expect(checkLatex('\\frac{a}{b} \\{x\\}')).toBeNull();
    expect(checkLatex('\\left( x \\right)')).toBeNull();
    expect(checkLatex('\\begin{matrix}1\\end{matrix}')).toBeNull();
  });
  it('rejects broken input', () => {
    expect(checkLatex('}{')).not.toBeNull();
    expect(checkLatex('\\left( x')).not.toBeNull();
    expect(checkLatex('\\begin{a}')).not.toBeNull();
  });
});
