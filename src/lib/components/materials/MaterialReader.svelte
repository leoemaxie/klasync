<script lang="ts">
  import { onMount } from 'svelte';
  import { formatTime } from '$lib/materials/doc';
  import {
    layoutDiagram,
    loadKatex,
    renderEquation,
    richInline,
  } from '$lib/materials/math';
  import type { Block, MaterialDoc } from '$lib/api/materials';

  let {
    doc,
    onSeek,
    showSources = false,
    highlightId = null,
  }: {
    doc: MaterialDoc;
    /** Called with a second offset when a source chip is clicked. */
    onSeek?: (sec: number) => void;
    /** Show "from the lecture at 12:30" chips (lecturer review). */
    showSources?: boolean;
    highlightId?: string | null;
  } = $props();

  // KaTeX loads on demand; until then equations show as escaped source.
  let katex = $state<Awaited<ReturnType<typeof loadKatex>> | null>(null);
  onMount(() => {
    loadKatex()
      .then((k) => (katex = k))
      .catch(() => {});
  });
  const html = (t: string) => richInline(katex, t);
  const calloutLabel: Record<string, string> = {
    note: 'Note',
    tip: 'Tip',
    warning: 'Warning',
    misconception: 'Common misconception',
  };
</script>

{#snippet sources(b: Block)}
  {#if showSources && b.source_ranges?.length}
    <div class="sources">
      {#each b.source_ranges as r}
        <button
          type="button"
          class="chip"
          disabled={!onSeek}
          onclick={() => onSeek?.(r.start_sec)}
          title="Jump to this moment in the lecture"
        >
          {formatTime(r.start_sec)}–{formatTime(r.end_sec)}
        </button>
      {/each}
    </div>
  {/if}
  {#if showSources && b.origin === 'supplemented'}
    <span class="flag">Added beyond the lecture</span>
  {/if}
  {#if showSources && b.low_confidence}
    <span class="flag warn">Low confidence — check against the recording</span>
  {/if}
{/snippet}

<article class="material">
  <header>
    <h1>{doc.title}</h1>
    {#if doc.subtitle}<p class="subtitle">{doc.subtitle}</p>{/if}
    {#if doc.meta.objectives.length}
      <section class="objectives" aria-label="Learning objectives">
        <h2>Learning objectives</h2>
        <ul>
          {#each doc.meta.objectives as o}<li>{@html html(o)}</li>{/each}
        </ul>
      </section>
    {/if}
    {#if doc.meta.ai_drafted}
      <p class="ai-note">Drafted with AI from the lecture recording.</p>
    {/if}
  </header>

  {#each doc.blocks as b (b.id)}
    <div
      class="block"
      class:hl={highlightId === b.id}
      data-block-id={b.id}
      data-type={b.type}
    >
      {#if b.type === 'heading'}
        <svelte:element this={`h${Math.min(b.level + 1, 6)}`} class="heading">
          {@html html(b.text)}
        </svelte:element>
      {:else if b.type === 'paragraph'}
        <p>{@html html(b.text)}</p>
      {:else if b.type === 'definition'}
        <div class="card def">
          <strong class="term">{b.term}</strong>
          <p>{@html html(b.text)}</p>
        </div>
      {:else if b.type === 'key_result'}
        <div class="card key">
          <span class="mat-label">{b.title ?? 'Key result'}</span>
          <p>{@html html(b.text)}</p>
        </div>
      {:else if b.type === 'worked_example'}
        <div class="card example">
          <span class="mat-label">Worked example · {b.title}</span>
          <p>{@html html(b.problem)}</p>
          <ol>
            {#each b.steps as step}<li>{@html html(step)}</li>{/each}
          </ol>
          {#if b.answer}<p class="answer">
              <strong>Answer:</strong>
              {@html html(b.answer)}
            </p>{/if}
        </div>
      {:else if b.type === 'equation'}
        <figure class="equation">
          <div class="tex">{@html renderEquation(katex, b.latex)}</div>
          {#if b.number}<span class="eqno">({b.number})</span>{/if}
          {#if b.caption}<figcaption>{@html html(b.caption)}</figcaption>{/if}
        </figure>
      {:else if b.type === 'figure'}
        <figure class="figure" aria-label={b.alt}>
          {#if b.diagram?.nodes.length}
            {@const nodes = layoutDiagram(b.diagram.nodes)}
            <svg
              viewBox="-120 0 880 640"
              role="img"
              aria-label={b.alt}
              class="diagram"
            >
              <defs>
                <marker
                  id="arrow"
                  viewBox="0 0 10 10"
                  refX="9"
                  refY="5"
                  markerWidth="6"
                  markerHeight="6"
                  orient="auto-start-reverse"
                >
                  <path d="M0,0 L10,5 L0,10 z" fill="#8a909c" />
                </marker>
              </defs>
              {#each b.diagram.edges as e}
                {@const a = nodes.find((n) => n.id === e.from)}
                {@const z = nodes.find((n) => n.id === e.to)}
                {#if a && z && a !== z}
                  {@const len = Math.hypot(z.x - a.x, z.y - a.y) || 1}
                  {@const ux = (z.x - a.x) / len}
                  {@const uy = (z.y - a.y) / len}
                  <line
                    x1={a.x + ux * 16}
                    y1={a.y + uy * 16}
                    x2={z.x - ux * 18}
                    y2={z.y - uy * 18}
                    stroke="#8a909c"
                    stroke-width="1.3"
                    marker-end="url(#arrow)"
                  />
                {/if}
              {/each}
              {#each nodes as n}
                <circle cx={n.x} cy={n.y} r="10" fill={n.color} />
                <text
                  x={n.x +
                    (n.anchor === 'end' ? -14 : n.anchor === 'start' ? 14 : 0)}
                  y={n.y + (n.anchor === 'middle' ? (n.y < 320 ? -14 : 22) : 4)}
                  font-size="12"
                  text-anchor={n.anchor}
                  fill="currentColor">{n.label}</text
                >
              {/each}
            </svg>
          {:else}
            <div class="fig-body">{b.text_fallback ?? b.alt}</div>
          {/if}
          <figcaption>{@html html(b.caption)}</figcaption>
        </figure>
      {:else if b.type === 'table'}
        <div class="table-wrap">
          <table>
            {#if b.caption}<caption>{@html html(b.caption)}</caption>{/if}
            <thead>
              <tr
                >{#each b.header as c}<th>{@html html(c)}</th>{/each}</tr
              >
            </thead>
            <tbody>
              {#each b.rows as row}
                <tr
                  >{#each row as c}<td>{@html html(c)}</td>{/each}</tr
                >
              {/each}
            </tbody>
          </table>
        </div>
      {:else if b.type === 'callout'}
        <aside class="card callout" data-variant={b.variant}>
          <span class="mat-label">{calloutLabel[b.variant] ?? 'Note'}</span>
          <p>{@html html(b.text)}</p>
        </aside>
      {:else if b.type === 'review_questions'}
        <section class="card">
          <span class="mat-label">Review questions</span>
          <ol>
            {#each b.questions as q}
              <li>
                {@html html(q.question)}
                {#if q.answer}
                  <details>
                    <summary>Answer</summary>{@html html(q.answer)}
                  </details>
                {/if}
              </li>
            {/each}
          </ol>
        </section>
      {:else if b.type === 'further_reading'}
        <section class="card">
          <span class="mat-label">Further reading</span>
          <ul>
            {#each b.items as it}
              <li>
                <strong>{it.title}</strong>
                {#if it.note}
                  — {@html html(it.note)}{/if}
              </li>
            {/each}
          </ul>
        </section>
      {:else if b.type === 'summary'}
        <section class="card key">
          <span class="mat-label">Summary</span>
          <ul>
            {#each b.points as pt}<li>{@html html(pt)}</li>{/each}
          </ul>
        </section>
      {/if}
      {@render sources(b)}
    </div>
  {/each}
</article>

<style>
  .material {
    max-width: 760px;
    margin: 0 auto;
    color: var(--text-secondary);
    font-size: 15.5px;
    line-height: 1.65;
  }
  h1 {
    margin: 0 0 4px;
    color: var(--text-primary);
    font-size: 28px;
    line-height: 1.2;
  }
  .subtitle {
    margin: 0 0 14px;
    color: var(--text-tertiary);
  }
  h2,
  :global(.material .heading) {
    color: var(--text-primary);
    margin: 22px 0 6px;
    line-height: 1.3;
  }
  .objectives h2 {
    font-size: 13px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-tertiary);
    margin: 14px 0 4px;
  }
  .ai-note {
    font-size: 12.5px;
    color: var(--text-tertiary);
  }
  .block {
    margin: 10px 0;
    padding: 2px 6px;
    border-radius: 8px;
  }
  .block.hl {
    background: var(--surface-accent);
  }
  p {
    margin: 0 0 8px;
  }
  .card {
    padding: 12px 14px;
    border: 1px solid var(--border-subtle);
    border-radius: 10px;
    background: var(--surface-2);
  }
  .card.key {
    border-color: var(--border-accent);
  }
  .card.callout[data-variant='warning'],
  .card.callout[data-variant='misconception'] {
    border-color: #c9892b;
  }
  .mat-label {
    display: block;
    margin-bottom: 4px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-accent);
  }
  .term {
    color: var(--text-primary);
  }
  .equation {
    margin: 0;
    text-align: center;
    overflow-x: auto;
  }
  .equation .tex {
    font-size: 1.1em;
    color: var(--text-primary);
  }
  .diagram {
    width: 100%;
    max-width: 520px;
    color: var(--text-primary);
  }
  .eqno {
    margin-left: 12px;
    color: var(--text-tertiary);
  }
  figcaption {
    font-size: 12.5px;
    color: var(--text-tertiary);
    margin-top: 4px;
  }
  .fig-body {
    padding: 14px;
    border: 1px dashed var(--border-default);
    border-radius: 10px;
    white-space: pre-wrap;
  }
  .table-wrap {
    overflow-x: auto;
  }
  table {
    border-collapse: collapse;
    width: 100%;
    font-size: 14px;
  }
  th,
  td {
    border: 1px solid var(--border-default);
    padding: 6px 10px;
    text-align: left;
  }
  th {
    background: var(--surface-3);
    color: var(--text-primary);
  }
  caption {
    caption-side: top;
    text-align: left;
    padding-bottom: 4px;
    color: var(--text-tertiary);
  }
  ol,
  ul {
    margin: 4px 0 4px 22px;
    padding: 0;
  }
  :global(.material code) {
    padding: 1px 5px;
    border-radius: 5px;
    background: var(--surface-inset);
    font-size: 0.92em;
  }
  .sources {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 4px;
  }
  .chip {
    padding: 1px 8px;
    font-size: 11.5px;
    border: 1px solid var(--border-default);
    border-radius: 999px;
    background: transparent;
    color: var(--text-tertiary);
    cursor: pointer;
  }
  .chip:disabled {
    cursor: default;
  }
  .chip:not(:disabled):hover {
    color: var(--text-primary);
    border-color: var(--border-strong);
  }
  .flag {
    display: inline-block;
    margin-top: 4px;
    margin-right: 6px;
    font-size: 11.5px;
    color: var(--text-tertiary);
  }
  .flag.warn {
    color: #f0b25c;
  }
</style>
