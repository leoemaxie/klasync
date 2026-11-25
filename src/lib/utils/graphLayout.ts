import type { ConceptEdge, ConceptNode } from '$lib/api/aiStudy';

export type LaidOutNode = ConceptNode & {
  x: number;
  y: number;
  r: number;
};

/**
 * Small deterministic force-directed layout (repulsion + springs + centring)
 * so concept maps need no charting dependency. Seeded from node order, so the
 * same map always renders identically.
 */
export function layoutGraph(
  nodes: ConceptNode[],
  edges: ConceptEdge[],
  width = 900,
  height = 560
): LaidOutNode[] {
  const count = nodes.length;
  if (count === 0) return [];
  const cx = width / 2;
  const cy = height / 2;
  const radiusFor = (importance: number) => 16 + importance * 4;

  const sorted = [...nodes].sort((a, b) => b.importance - a.importance);
  const pos = new Map<
    string,
    { x: number; y: number; vx: number; vy: number }
  >();
  sorted.forEach((node, i) => {
    // Most important node in the middle, the rest on a spiral around it.
    const angle = i * 2.399963; // golden angle
    const dist = i === 0 ? 0 : 60 + Math.sqrt(i) * 70;
    pos.set(node.id, {
      x: cx + Math.cos(angle) * dist,
      y: cy + Math.sin(angle) * dist * 0.75,
      vx: 0,
      vy: 0,
    });
  });
  const byId = new Map(nodes.map((n) => [n.id, n]));
  const links = edges.filter((e) => pos.has(e.from) && pos.has(e.to));

  for (let step = 0; step < 320; step++) {
    const cooling = 1 - step / 320;
    const ids = [...pos.keys()];
    for (let i = 0; i < ids.length; i++) {
      for (let j = i + 1; j < ids.length; j++) {
        const a = pos.get(ids[i])!;
        const b = pos.get(ids[j])!;
        let dx = a.x - b.x;
        let dy = a.y - b.y;
        let d2 = dx * dx + dy * dy;
        if (d2 < 1) {
          dx = (i - j) * 0.5;
          dy = 0.5;
          d2 = 1;
        }
        const d = Math.sqrt(d2);
        const force = 5200 / d2;
        a.vx += (dx / d) * force;
        a.vy += (dy / d) * force;
        b.vx -= (dx / d) * force;
        b.vy -= (dy / d) * force;
      }
    }
    for (const link of links) {
      const a = pos.get(link.from)!;
      const b = pos.get(link.to)!;
      const dx = b.x - a.x;
      const dy = b.y - a.y;
      const d = Math.max(Math.sqrt(dx * dx + dy * dy), 1);
      const force = (d - 150) * 0.02;
      a.vx += (dx / d) * force;
      a.vy += (dy / d) * force;
      b.vx -= (dx / d) * force;
      b.vy -= (dy / d) * force;
    }
    for (const p of pos.values()) {
      p.vx += (cx - p.x) * 0.008;
      p.vy += (cy - p.y) * 0.008;
      p.x += Math.max(-24, Math.min(24, p.vx)) * cooling;
      p.y += Math.max(-24, Math.min(24, p.vy)) * cooling;
      p.vx *= 0.55;
      p.vy *= 0.55;
    }
  }

  return nodes.map((node) => {
    const p = pos.get(node.id)!;
    const r = radiusFor(byId.get(node.id)!.importance);
    return {
      ...node,
      r,
      x: Math.max(r + 6, Math.min(width - r - 6, p.x)),
      y: Math.max(r + 6, Math.min(height - r - 6, p.y)),
    };
  });
}
