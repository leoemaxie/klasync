<script lang="ts">
  import type { SessionVisualization } from '$lib/api/aiStudy';
  import { layoutGraph } from '$lib/utils/graphLayout';
  import { colorForGroup, groupsOf } from '$lib/utils/conceptPalette';

  let {
    visualization,
    selectedId = null,
    activeIds = new Set<string>(),
    onSelect,
  }: {
    visualization: SessionVisualization;
    /** Optional shared selection (used by the studio's concept explorer). */
    selectedId?: string | null;
    /** Concepts being discussed at the playhead; drawn with a ring. */
    activeIds?: Set<string>;
    onSelect?: (id: string | null) => void;
  } = $props();

  const W = 900;
  const H = 560;

  const nodes = $derived(
    layoutGraph(
      visualization.spec.nodes ?? [],
      visualization.spec.edges ?? [],
      W,
      H
    )
  );
  const byId = $derived(new Map(nodes.map((n) => [n.id, n])));
  const groups = $derived(groupsOf(nodes));
  const colorFor = (group: string) => colorForGroup(groups, group);
  let hovered = $state<string | null>(null);
  /** Hover wins, then the shared selection, so the map stays focused. */
  const focusId = $derived(hovered ?? selectedId);

  const edges = $derived(
    (visualization.spec.edges ?? []).filter(
      (e) => byId.has(e.from) && byId.has(e.to)
    )
  );
  const isNeighbour = (id: string) =>
    focusId === null ||
    id === focusId ||
    edges.some(
      (e) =>
        (e.from === focusId && e.to === id) ||
        (e.to === focusId && e.from === id)
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
    role="group"
    aria-label="Concept map: {visualization.title}. {nodes.length} concepts."
  >
    {#each edges as edge}
      {@const a = byId.get(edge.from)!}
      {@const b = byId.get(edge.to)!}
      <g
        class="edge"
        opacity={focusId === null ||
        edge.from === focusId ||
        edge.to === focusId
          ? 1
          : 0.12}
      >
        <line x1={a.x} y1={a.y} x2={b.x} y2={b.y} />
        {#if edge.label && (focusId === edge.from || focusId === edge.to)}
          <text
            x={(a.x + b.x) / 2}
            y={(a.y + b.y) / 2 - 4}
            text-anchor="middle"
          >
            {edge.label}
          </text>
        {/if}
      </g>
    {/each}
    {#each nodes as node (node.id)}
      <g
        class="node"
        role="button"
        tabindex="0"
        aria-label={node.label}
        aria-pressed={onSelect ? selectedId === node.id : undefined}
        opacity={isNeighbour(node.id) ? 1 : 0.25}
        onmouseenter={() => (hovered = node.id)}
        onmouseleave={() => (hovered = null)}
        onfocus={() => (hovered = node.id)}
        onblur={() => (hovered = null)}
        onclick={() => onSelect?.(selectedId === node.id ? null : node.id)}
        onkeydown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            onSelect?.(selectedId === node.id ? null : node.id);
          }
        }}
      >
        {#if activeIds.has(node.id) || selectedId === node.id}
          <circle
            class="ring"
            class:active={activeIds.has(node.id)}
            cx={node.x}
            cy={node.y}
            r={node.r + 6}
          />
        {/if}
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
  .node circle.ring {
    fill: none;
    stroke: var(--text-primary);
    stroke-width: 2;
    opacity: 0.9;
  }
  .node circle.ring.active {
    stroke: #ffb070;
    stroke-dasharray: 4 4;
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
