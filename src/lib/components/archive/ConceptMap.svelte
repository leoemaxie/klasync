<script lang="ts">
  import type { SessionVisualization } from '$lib/api/aiStudy';
  import { layoutGraph } from '$lib/utils/graphLayout';

  let { visualization }: { visualization: SessionVisualization } = $props();

  const W = 900;
  const H = 560;
  const PALETTE = [
    '#dc5000',
    '#e0a458',
    '#7fb069',
    '#6aa6c9',
    '#b48ead',
    '#d98880',
    '#9aa5b1',
  ];

  const nodes = $derived(
    layoutGraph(
      visualization.spec.nodes ?? [],
      visualization.spec.edges ?? [],
      W,
      H
    )
  );
  const byId = $derived(new Map(nodes.map((n) => [n.id, n])));
  const groups = $derived([...new Set(nodes.map((n) => n.group))]);
  const colorFor = (group: string) =>
    PALETTE[Math.max(groups.indexOf(group), 0) % PALETTE.length];
  let hovered = $state<string | null>(null);

  const edges = $derived(
    (visualization.spec.edges ?? []).filter(
      (e) => byId.has(e.from) && byId.has(e.to)
    )
  );
  const isNeighbour = (id: string) =>
    hovered === null ||
    id === hovered ||
    edges.some(
      (e) =>
        (e.from === hovered && e.to === id) ||
        (e.to === hovered && e.from === id)
    );

  function words(label: string): string[] {
    const parts = label.split(' ');
    const lines: string[] = [];
    let current = '';
    for (const part of parts) {
      if ((current + ' ' + part).trim().length > 14 && current) {
        lines.push(current);
        current = part;
      } else {
        current = (current + ' ' + part).trim();
      }
    }
    if (current) lines.push(current);
    return lines.slice(0, 3);
  }
</script>

<figure class="concept-map">
  <svg
    viewBox="0 0 {W} {H}"
    role="img"
    aria-label="Concept map: {visualization.title}. {nodes.length} concepts."
  >
    {#each edges as edge}
      {@const a = byId.get(edge.from)!}
      {@const b = byId.get(edge.to)!}
      <g
        class="edge"
        opacity={hovered === null ||
        edge.from === hovered ||
        edge.to === hovered
          ? 1
          : 0.12}
      >
        <line x1={a.x} y1={a.y} x2={b.x} y2={b.y} />
        {#if edge.label && (hovered === edge.from || hovered === edge.to)}
          <text x={(a.x + b.x) / 2} y={(a.y + b.y) / 2 - 4} text-anchor="middle"
            >{edge.label}</text
          >
        {/if}
      </g>
    {/each}
    {#each nodes as node (node.id)}
      <g
        class="node"
        role="button"
        tabindex="0"
        aria-label={node.label}
        opacity={isNeighbour(node.id) ? 1 : 0.25}
        onmouseenter={() => (hovered = node.id)}
        onmouseleave={() => (hovered = null)}
        onfocus={() => (hovered = node.id)}
        onblur={() => (hovered = null)}
      >
        <circle
          cx={node.x}
          cy={node.y}
          r={node.r}
          fill={colorFor(node.group)}
        />
        <text x={node.x} y={node.y + node.r + 13} text-anchor="middle">
          {#each words(node.label) as line, i}
            <tspan x={node.x} dy={i === 0 ? 0 : 12}>{line}</tspan>
          {/each}
        </text>
      </g>
    {/each}
  </svg>

  {#if groups.length > 1}
    <figcaption class="legend">
      {#each groups as group}
        <span class="legend-item">
          <span class="dot" style="background:{colorFor(group)}"></span>{group}
        </span>
      {/each}
    </figcaption>
  {/if}
</figure>

<style>
  .concept-map {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--spacing-8);
  }
  svg {
    width: 100%;
    height: auto;
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards, 8px);
  }
  .edge line {
    stroke: rgba(255, 237, 215, 0.28);
    stroke-width: 1.5;
  }
  .edge text {
    fill: var(--color-warm-cream-dim);
    font-size: 10px;
  }
  .node {
    cursor: pointer;
    outline: none;
  }
  .node circle {
    stroke: rgba(255, 237, 215, 0.55);
    stroke-width: 1.5;
  }
  .node:focus circle,
  .node:hover circle {
    stroke: var(--color-warm-cream);
    stroke-width: 3;
  }
  .node text {
    fill: var(--color-warm-cream);
    font-size: 11px;
    font-weight: 600;
    pointer-events: none;
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 10px 16px;
    font-size: 11px;
    color: var(--color-warm-cream-dim);
  }
  .legend-item {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    display: inline-block;
  }
</style>
