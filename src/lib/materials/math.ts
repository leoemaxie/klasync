/**
 * KaTeX rendering for equations and inline `$math$`. KaTeX is loaded lazily so
 * it stays out of the main bundle until a handout is opened.
 */
import { inlineHtml } from './doc';

type Katex = typeof import('katex').default;
let katex: Katex | null = null;
let loading: Promise<Katex> | null = null;

export function loadKatex(): Promise<Katex> {
  if (katex) return Promise.resolve(katex);
  loading ??= Promise.all([
    import('katex'),
    import('katex/dist/katex.min.css'),
  ]).then(([m]) => (katex = m.default));
  return loading;
}

const escape = (s: string) =>
  s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

/** Block equation to HTML; falls back to escaped source if KaTeX rejects it. */
export function renderEquation(k: Katex | null, latex: string): string {
  if (!k) return `<code>${escape(latex)}</code>`;
  try {
    return k.renderToString(latex, {
      displayMode: true,
      throwOnError: true,
      output: 'htmlAndMathml',
      trust: false,
    });
  } catch {
    return `<code>${escape(latex)}</code>`;
  }
}

/** Inline Markdown plus `$…$` math to safe HTML. */
export function richInline(k: Katex | null, text: string): string {
  if (!k || !text.includes('$')) return inlineHtml(text);
  const parts = text.split(/(\$(?=\S)[^$\n]*?\S\$(?!\d))/g);
  return parts
    .map((part) => {
      const m = /^\$(?=\S)([^$\n]*?\S)\$$/.exec(part);
      if (!m) return inlineHtml(part);
      try {
        return k.renderToString(m[1], { throwOnError: true, trust: false });
      } catch {
        return inlineHtml(part);
      }
    })
    .join('');
}

/** Circular node-link layout for a figure's diagram (mirrors the server SVG). */
export interface LaidOutNode {
  id: string;
  label: string;
  x: number;
  y: number;
  color: string;
  anchor: 'start' | 'middle' | 'end';
}

const PALETTE = [
  '#2f6fdd',
  '#c4552b',
  '#2e8b57',
  '#8a4fbf',
  '#b8860b',
  '#2a8c9c',
];

export function layoutDiagram(
  nodes: { id: string; label: string; group?: string | null }[],
  size = 640
): LaidOutNode[] {
  const order = [...nodes]
    .sort((a, b) => (a.group ?? '').localeCompare(b.group ?? ''))
    .slice(0, 24);
  const groups = [...new Set(order.map((n) => n.group ?? ''))];
  const c = size / 2;
  const r = order.length === 1 ? 0 : size / 2 - 90;
  return order.map((n, i) => {
    const a = (2 * Math.PI * i) / order.length - Math.PI / 2;
    const x = c + r * Math.cos(a);
    const y = c + r * Math.sin(a);
    return {
      id: n.id,
      label: n.label.length > 22 ? `${n.label.slice(0, 21)}…` : n.label,
      x,
      y,
      color: PALETTE[groups.indexOf(n.group ?? '') % PALETTE.length],
      anchor: x < c - 10 ? 'end' : x > c + 10 ? 'start' : 'middle',
    };
  });
}
