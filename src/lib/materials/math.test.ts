import { describe, expect, it } from 'vitest';
import { layoutDiagram, loadKatex, renderEquation, richInline } from './math';

describe('math rendering', () => {
  it('renders valid LaTeX with KaTeX and escapes bad input', async () => {
    const k = await loadKatex();
    expect(renderEquation(k, '\\frac{a}{b}')).toContain('katex');
    const bad = renderEquation(k, '\\notacommand{');
    expect(bad).toContain('<code>');
    expect(renderEquation(null, '<x>')).toBe('<code>&lt;x&gt;</code>');
  });

  it('renders inline $math$ and still escapes the rest', async () => {
    const k = await loadKatex();
    const html = richInline(k, 'Energy $E=mc^2$ is <b>**big**</b>');
    expect(html).toContain('katex');
    expect(html).toContain('&lt;b&gt;');
    expect(richInline(null, 'cost $5 and $6')).toBe('cost $5 and $6');
    expect(richInline(k, 'cost $5 and $6')).not.toContain('katex');
  });
});

describe('layoutDiagram', () => {
  it('places nodes on a circle with a colour per group', () => {
    const out = layoutDiagram([
      { id: 'a', label: 'A', group: 'x' },
      { id: 'b', label: 'B', group: 'y' },
      { id: 'c', label: 'C', group: 'x' },
    ]);
    expect(out).toHaveLength(3);
    expect(out.find((n) => n.id === 'a')!.color).toBe(
      out.find((n) => n.id === 'c')!.color
    );
    expect(out.find((n) => n.id === 'a')!.color).not.toBe(
      out.find((n) => n.id === 'b')!.color
    );
  });
  it('caps at 24 nodes and truncates long labels', () => {
    const many = Array.from({ length: 30 }, (_, i) => ({
      id: `${i}`,
      label: 'L'.repeat(40),
    }));
    const out = layoutDiagram(many);
    expect(out).toHaveLength(24);
    expect(out[0].label.endsWith('…')).toBe(true);
  });
});
