/**
 * Pure helpers for the lecture material document: safe inline rendering,
 * block factories and edit operations, and client-side validation that mirrors
 * `backend/src/materials/blocks.rs::validate` so the editor can flag problems
 * before the server rejects a save. Nothing here touches the network or DOM.
 */
import type { Block, BlockKind, MaterialDoc } from '$lib/api/materials';

// ---- inline text ----------------------------------------------------------

const escapeHtml = (s: string) =>
  s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

/** Inline Markdown (`**bold**`, `*italic*`, `` `code` ``) to safe HTML. */
export function inlineHtml(text: string): string {
  return escapeHtml(text ?? '')
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
    .replace(/(^|[^*])\*([^*\n]+)\*(?!\*)/g, '$1<em>$2</em>');
}

/** `mm:ss` (or `h:mm:ss`) for a number of seconds. */
export function formatTime(totalSec: number): string {
  const s = Math.max(0, Math.floor(totalSec));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  const pad = (n: number) => String(n).padStart(2, '0');
  return h > 0 ? `${h}:${pad(m)}:${pad(sec)}` : `${m}:${pad(sec)}`;
}

// ---- block factories ------------------------------------------------------

export function newBlockId(): string {
  const c = globalThis.crypto;
  return c?.randomUUID
    ? `b_${c.randomUUID().slice(0, 12)}`
    : `b_${Math.random().toString(36).slice(2, 14)}`;
}

export type AddableType = Exclude<BlockKind['type'], 'figure'>;

export const ADDABLE_BLOCKS: { type: AddableType; label: string }[] = [
  { type: 'heading', label: 'Section heading' },
  { type: 'paragraph', label: 'Paragraph' },
  { type: 'definition', label: 'Definition' },
  { type: 'key_result', label: 'Key result' },
  { type: 'worked_example', label: 'Worked example' },
  { type: 'equation', label: 'Equation' },
  { type: 'table', label: 'Table' },
  { type: 'callout', label: 'Callout' },
  { type: 'review_questions', label: 'Review questions' },
  { type: 'further_reading', label: 'Further reading' },
  { type: 'summary', label: 'Summary' },
];

export function blankKind(type: AddableType): BlockKind {
  switch (type) {
    case 'heading':
      return { type, level: 2, text: 'New section' };
    case 'paragraph':
      return { type, text: '' };
    case 'definition':
      return { type, term: '', text: '' };
    case 'key_result':
      return { type, text: '' };
    case 'worked_example':
      return { type, title: '', problem: '', steps: [''] };
    case 'equation':
      return { type, latex: '' };
    case 'table':
      return { type, header: ['Column 1', 'Column 2'], rows: [['', '']] };
    case 'callout':
      return { type, variant: 'note', text: '' };
    case 'review_questions':
      return { type, questions: [{ question: '' }] };
    case 'further_reading':
      return { type, items: [{ title: '' }] };
    case 'summary':
      return { type, points: [''] };
  }
}

/** A lecturer-authored block: not tied to the transcript, so not "AI". */
export function makeBlock(type: AddableType): Block {
  return {
    ...blankKind(type),
    id: newBlockId(),
    origin: 'supplemented',
    locked: true,
    low_confidence: false,
  } as Block;
}

// ---- edit operations (all immutable) --------------------------------------

export const insertBlock = (
  doc: MaterialDoc,
  index: number,
  block: Block
): MaterialDoc => {
  const at = Math.min(Math.max(index, 0), doc.blocks.length);
  return {
    ...doc,
    blocks: [...doc.blocks.slice(0, at), block, ...doc.blocks.slice(at)],
  };
};

export const removeBlock = (doc: MaterialDoc, id: string): MaterialDoc => ({
  ...doc,
  blocks: doc.blocks.filter((b) => b.id !== id),
});

/** Replace a block's content. Editing locks it so regeneration keeps the edit. */
export const updateBlock = (
  doc: MaterialDoc,
  id: string,
  patch: Partial<Block>
): MaterialDoc => ({
  ...doc,
  blocks: doc.blocks.map((b) =>
    b.id === id ? ({ ...b, locked: true, ...patch } as Block) : b
  ),
});

export const setLocked = (
  doc: MaterialDoc,
  id: string,
  locked: boolean
): MaterialDoc => ({
  ...doc,
  blocks: doc.blocks.map((b) => (b.id === id ? { ...b, locked } : b)),
});

/** Move a block up (-1) or down (+1); no-op at the ends. */
export function moveBlock(
  doc: MaterialDoc,
  id: string,
  delta: -1 | 1
): MaterialDoc {
  const i = doc.blocks.findIndex((b) => b.id === id);
  const j = i + delta;
  if (i < 0 || j < 0 || j >= doc.blocks.length) return doc;
  const blocks = [...doc.blocks];
  [blocks[i], blocks[j]] = [blocks[j], blocks[i]];
  return { ...doc, blocks };
}

/** A section is a heading and the blocks up to the next heading of <= level. */
export function sectionRange(
  doc: MaterialDoc,
  headingId: string
): [number, number] | null {
  const start = doc.blocks.findIndex((b) => b.id === headingId);
  const head = doc.blocks[start];
  if (!head || head.type !== 'heading') return null;
  let end = start + 1;
  while (end < doc.blocks.length) {
    const b = doc.blocks[end];
    if (b.type === 'heading' && b.level <= head.level) break;
    end++;
  }
  return [start, end];
}

/** Move a whole section (heading + body) above or below its neighbour. */
export function moveSection(
  doc: MaterialDoc,
  headingId: string,
  delta: -1 | 1
): MaterialDoc {
  const range = sectionRange(doc, headingId);
  if (!range) return doc;
  const [start, end] = range;
  const level = (doc.blocks[start] as Extract<Block, { type: 'heading' }>)
    .level;
  if (delta === 1) {
    const next = doc.blocks[end];
    if (!next || next.type !== 'heading' || next.level !== level) return doc;
    const [ns, ne] = sectionRange(doc, next.id)!;
    return {
      ...doc,
      blocks: [
        ...doc.blocks.slice(0, start),
        ...doc.blocks.slice(ns, ne),
        ...doc.blocks.slice(start, end),
        ...doc.blocks.slice(ne),
      ],
    };
  }
  // Previous sibling: nearest earlier heading of the same level, with no
  // shallower heading in between.
  let p = start - 1;
  while (p >= 0) {
    const b = doc.blocks[p];
    if (b.type === 'heading' && b.level < level) return doc;
    if (b.type === 'heading' && b.level === level) break;
    p--;
  }
  if (p < 0) return doc;
  return {
    ...doc,
    blocks: [
      ...doc.blocks.slice(0, p),
      ...doc.blocks.slice(start, end),
      ...doc.blocks.slice(p, start),
      ...doc.blocks.slice(end),
    ],
  };
}

export interface OutlineEntry {
  id: string;
  level: number;
  text: string;
}

export const outline = (doc: MaterialDoc): OutlineEntry[] =>
  doc.blocks.flatMap((b) =>
    b.type === 'heading' ? [{ id: b.id, level: b.level, text: b.text }] : []
  );

// ---- validation (mirror of blocks.rs) -------------------------------------

const MAX_TEXT = 20_000;
const MAX_BLOCKS = 2_000;

const fencesClosed = (t: string) => (t.match(/```/g)?.length ?? 0) % 2 === 0;

/** Balanced braces, `\left`/`\right` and `\begin`/`\end` (cheap structural check). */
export function checkLatex(latex: string): string | null {
  let depth = 0;
  for (let i = 0; i < latex.length; i++) {
    const c = latex[i];
    if (c === '\\') i++;
    else if (c === '{') depth++;
    else if (c === '}' && --depth < 0) return 'unmatched closing brace';
  }
  if (depth !== 0) return 'unclosed brace';
  const count = (re: RegExp) => latex.match(re)?.length ?? 0;
  if (count(/\\left(?![a-zA-Z])/g) !== count(/\\right(?![a-zA-Z])/g))
    return 'unbalanced \\left / \\right';
  if (count(/\\begin\{/g) !== count(/\\end\{/g))
    return 'unbalanced \\begin / \\end';
  return null;
}

export interface Problem {
  blockId: string | null;
  message: string;
}

/** Problems the server would reject the document for. Empty means valid. */
export function validateDoc(doc: MaterialDoc): Problem[] {
  const out: Problem[] = [];
  const add = (blockId: string | null, message: string) =>
    out.push({ blockId, message });
  if (!doc.title.trim()) add(null, 'The document needs a title');
  if (doc.blocks.length === 0) add(null, 'The document has no content');
  if (doc.blocks.length > MAX_BLOCKS)
    add(null, `Too many blocks (limit ${MAX_BLOCKS})`);
  const seen = new Set<string>();
  const text = (b: Block, label: string, t: string) => {
    if (!t.trim()) add(b.id, `Empty ${label}`);
    else if (t.length > MAX_TEXT) add(b.id, `Too much text in ${label}`);
    else if (!fencesClosed(t)) add(b.id, `Unclosed code fence in ${label}`);
  };
  for (const b of doc.blocks) {
    if (!b.id.trim()) add(null, 'A block has no id');
    else if (seen.has(b.id)) add(b.id, 'Duplicate block id');
    seen.add(b.id);
    switch (b.type) {
      case 'heading':
        text(b, 'heading', b.text);
        break;
      case 'paragraph':
      case 'callout':
      case 'key_result':
        text(b, 'text', b.text);
        break;
      case 'definition':
        text(b, 'term', b.term);
        text(b, 'definition', b.text);
        break;
      case 'worked_example':
        text(b, 'title', b.title);
        text(b, 'problem', b.problem);
        if (b.steps.length === 0) add(b.id, 'Worked example has no steps');
        break;
      case 'equation': {
        if (!b.latex.trim()) add(b.id, 'Empty equation');
        else {
          const why = checkLatex(b.latex);
          if (why) add(b.id, `Equation does not parse: ${why}`);
        }
        break;
      }
      case 'figure':
        text(b, 'caption', b.caption);
        text(b, 'alt text', b.alt);
        break;
      case 'table':
        if (b.header.length === 0) add(b.id, 'Table has no header');
        if (b.rows.some((r) => r.length !== b.header.length))
          add(b.id, 'Table has rows of the wrong width');
        break;
      case 'review_questions':
        if (
          b.questions.length === 0 ||
          b.questions.some((q) => !q.question.trim())
        )
          add(b.id, 'Review questions are empty');
        break;
      case 'further_reading':
        if (b.items.length === 0 || b.items.some((i) => !i.title.trim()))
          add(b.id, 'Further reading is empty');
        break;
      case 'summary':
        if (b.points.length === 0 || b.points.some((p) => !p.trim()))
          add(b.id, 'Summary is empty');
        break;
    }
    for (const r of b.source_ranges ?? []) {
      if (r.start_sec < 0 || r.end_sec < r.start_sec)
        add(b.id, 'Invalid source range');
    }
  }
  return out;
}

/** Stable text fingerprint, used to tell whether the editor has unsaved edits. */
export const docFingerprint = (doc: MaterialDoc) => JSON.stringify(doc);

/** One-line label for a block in lists and validation messages. */
export function blockLabel(b: Block): string {
  switch (b.type) {
    case 'heading':
      return b.text;
    case 'paragraph':
    case 'callout':
    case 'key_result':
      return b.text.slice(0, 60);
    case 'definition':
      return `Definition: ${b.term}`;
    case 'worked_example':
      return `Example: ${b.title}`;
    case 'equation':
      return `Equation ${b.latex.slice(0, 30)}`;
    case 'figure':
      return `Figure: ${b.caption}`;
    case 'table':
      return `Table ${b.caption ?? ''}`.trim();
    case 'review_questions':
      return 'Review questions';
    case 'further_reading':
      return 'Further reading';
    case 'summary':
      return 'Summary';
  }
}
