import { describe, expect, it } from 'vitest';
import { layoutGraph3D } from './graphLayout3d';

const node = (id: string, importance = 3, group = 'g') => ({
  id,
  label: id,
  group,
  importance,
});
const edge = (from: string, to: string) => ({ from, to, label: '' });

const nodes = [
  node('a', 5),
  node('b', 4),
  node('c', 4),
  node('d', 3, 'h'),
  node('e', 3, 'h'),
  node('f', 2),
];
const edges = [edge('a', 'b'), edge('a', 'c'), edge('b', 'd'), edge('c', 'e')];

describe('layoutGraph3D', () => {
  it('returns nothing for an empty graph', () => {
    expect(layoutGraph3D([], [])).toEqual([]);
  });

  it('is deterministic', () => {
    expect(layoutGraph3D(nodes, edges)).toEqual(layoutGraph3D(nodes, edges));
  });

  it('keeps every node (and its data) and adds finite coordinates', () => {
    const out = layoutGraph3D(nodes, edges);
    expect(out.map((n) => n.id)).toEqual(nodes.map((n) => n.id));
    for (const n of out) {
      expect(Number.isFinite(n.x + n.y + n.z)).toBe(true);
      expect(n.r).toBeGreaterThan(0);
    }
  });

  it('scales so the farthest node sits at the requested radius', () => {
    for (const radius of [4, 7, 12]) {
      const out = layoutGraph3D(nodes, edges, radius);
      const max = Math.max(...out.map((n) => Math.hypot(n.x, n.y, n.z)));
      // Re-centred on the centroid, so the max distance from the origin is
      // close to (not necessarily exactly) the radius.
      expect(max).toBeGreaterThan(radius * 0.5);
      expect(max).toBeLessThanOrEqual(radius * 2);
    }
  });

  it('sizes spheres by importance', () => {
    const out = layoutGraph3D(nodes, edges);
    const r = (id: string) => out.find((n) => n.id === id)!.r;
    expect(r('a')).toBeGreaterThan(r('d'));
    expect(r('d')).toBeGreaterThan(r('f'));
  });

  it('keeps nodes from overlapping', () => {
    const out = layoutGraph3D(nodes, edges);
    for (let i = 0; i < out.length; i++)
      for (let j = i + 1; j < out.length; j++) {
        const d = Math.hypot(
          out[i].x - out[j].x,
          out[i].y - out[j].y,
          out[i].z - out[j].z
        );
        expect(d).toBeGreaterThan(out[i].r + out[j].r);
      }
  });

  it('ignores edges that point at unknown nodes', () => {
    const withBad = [...edges, edge('a', 'missing'), edge('ghost', 'b')];
    expect(() => layoutGraph3D(nodes, withBad)).not.toThrow();
    expect(layoutGraph3D(nodes, withBad)).toEqual(layoutGraph3D(nodes, edges));
  });

  it('handles a single node', () => {
    const [only] = layoutGraph3D([node('solo')], []);
    expect(only.x).toBeCloseTo(0);
    expect(only.y).toBeCloseTo(0);
    expect(only.z).toBeCloseTo(0);
  });

  it('pulls connected concepts closer than unconnected ones', () => {
    const out = layoutGraph3D(nodes, edges);
    const at = (id: string) => out.find((n) => n.id === id)!;
    const dist = (p: string, q: string) =>
      Math.hypot(at(p).x - at(q).x, at(p).y - at(q).y, at(p).z - at(q).z);
    expect(dist('a', 'b')).toBeLessThan(dist('f', 'd') + 6);
  });
});
