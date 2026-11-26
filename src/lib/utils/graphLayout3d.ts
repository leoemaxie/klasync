import type { ConceptEdge, ConceptNode } from '$lib/api/aiStudy';

export type LaidOutNode3D = ConceptNode & {
  x: number;
  y: number;
  z: number;
  /** Sphere radius in scene units. */
  r: number;
};

const radiusFor = (importance: number) =>
  0.3 + Math.min(Math.max(importance, 1), 6) * 0.09;

/**
 * Deterministic 3D force layout. The most important concept sits at the
 * centre; the rest start on a Fibonacci spiral around it and are relaxed with
 * repulsion, edge springs, group cohesion and centring. Seeded purely from
 * node order, so the same map always lays out identically.
 *
 * The result is scaled so the farthest node sits at `radius` scene units.
 */
export function layoutGraph3D(
  nodes: ConceptNode[],
  edges: ConceptEdge[],
  radius = 7
): LaidOutNode3D[] {
  const count = nodes.length;
  if (count === 0) return [];

  type P = {
    x: number;
    y: number;
    z: number;
    vx: number;
    vy: number;
    vz: number;
  };
  const sorted = [...nodes].sort(
    (a, b) => b.importance - a.importance || a.label.localeCompare(b.label)
  );
  const pos = new Map<string, P>();
  const golden = Math.PI * (3 - Math.sqrt(5));
  sorted.forEach((node, i) => {
    if (i === 0) {
      pos.set(node.id, { x: 0, y: 0, z: 0, vx: 0, vy: 0, vz: 0 });
      return;
    }
    const t = (i - 0.5) / Math.max(count - 1, 1);
    const y = 1 - 2 * t;
    const ring = Math.sqrt(Math.max(1 - y * y, 0));
    const theta = golden * i;
    const dist = 2.4 + Math.sqrt(i) * 1.1;
    pos.set(node.id, {
      x: Math.cos(theta) * ring * dist,
      y: y * dist,
      z: Math.sin(theta) * ring * dist,
      vx: 0,
      vy: 0,
      vz: 0,
    });
  });

  const ids = sorted.map((n) => n.id);
  const group = new Map(nodes.map((n) => [n.id, n.group]));
  const links = edges.filter((e) => pos.has(e.from) && pos.has(e.to));
  const STEPS = 260;

  for (let step = 0; step < STEPS; step++) {
    const cooling = 1 - step / STEPS;
    // Repulsion
    for (let i = 0; i < ids.length; i++) {
      for (let j = i + 1; j < ids.length; j++) {
        const a = pos.get(ids[i])!;
        const b = pos.get(ids[j])!;
        let dx = a.x - b.x;
        let dy = a.y - b.y;
        let dz = a.z - b.z;
        let d2 = dx * dx + dy * dy + dz * dz;
        if (d2 < 0.01) {
          dx = (i - j) * 0.1;
          dy = 0.1;
          dz = 0.05;
          d2 = 0.0225;
        }
        const d = Math.sqrt(d2);
        const force = 9 / d2;
        a.vx += (dx / d) * force;
        a.vy += (dy / d) * force;
        a.vz += (dz / d) * force;
        b.vx -= (dx / d) * force;
        b.vy -= (dy / d) * force;
        b.vz -= (dz / d) * force;
      }
    }
    // Edge springs
    for (const link of links) {
      const a = pos.get(link.from)!;
      const b = pos.get(link.to)!;
      const dx = b.x - a.x;
      const dy = b.y - a.y;
      const dz = b.z - a.z;
      const d = Math.max(Math.sqrt(dx * dx + dy * dy + dz * dz), 0.01);
      const force = (d - 3.2) * 0.045;
      a.vx += (dx / d) * force;
      a.vy += (dy / d) * force;
      a.vz += (dz / d) * force;
      b.vx -= (dx / d) * force;
      b.vy -= (dy / d) * force;
      b.vz -= (dz / d) * force;
    }
    // Group cohesion: same-group concepts drift toward their centroid.
    const centroids = new Map<
      string,
      { x: number; y: number; z: number; n: number }
    >();
    for (const id of ids) {
      const p = pos.get(id)!;
      const g = group.get(id) ?? '';
      const c = centroids.get(g) ?? { x: 0, y: 0, z: 0, n: 0 };
      c.x += p.x;
      c.y += p.y;
      c.z += p.z;
      c.n++;
      centroids.set(g, c);
    }
    for (const id of ids) {
      const p = pos.get(id)!;
      const c = centroids.get(group.get(id) ?? '')!;
      if (c.n > 1) {
        p.vx += (c.x / c.n - p.x) * 0.012;
        p.vy += (c.y / c.n - p.y) * 0.012;
        p.vz += (c.z / c.n - p.z) * 0.012;
      }
      // Centring, slightly stronger on the vertical axis for a flatter shape.
      p.vx -= p.x * 0.012;
      p.vy -= p.y * 0.02;
      p.vz -= p.z * 0.012;
    }
    for (const p of pos.values()) {
      const speed = Math.hypot(p.vx, p.vy, p.vz);
      const cap = 0.6;
      const k = speed > cap ? cap / speed : 1;
      p.x += p.vx * k * cooling;
      p.y += p.vy * k * cooling;
      p.z += p.vz * k * cooling;
      p.vx *= 0.6;
      p.vy *= 0.6;
      p.vz *= 0.6;
    }
  }

  // Re-centre on the centroid and scale to the requested radius.
  let cx = 0;
  let cy = 0;
  let cz = 0;
  for (const p of pos.values()) {
    cx += p.x;
    cy += p.y;
    cz += p.z;
  }
  cx /= count;
  cy /= count;
  cz /= count;
  let max = 0;
  for (const p of pos.values()) {
    max = Math.max(max, Math.hypot(p.x - cx, p.y - cy, p.z - cz));
  }
  const scale = max > 0 ? radius / max : 1;

  return nodes.map((node) => {
    const p = pos.get(node.id)!;
    return {
      ...node,
      x: (p.x - cx) * scale,
      y: (p.y - cy) * scale,
      z: (p.z - cz) * scale,
      r: radiusFor(node.importance),
    };
  });
}
