<script lang="ts">
  import { inlineHtml, formatTime } from '$lib/materials/doc';
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

  const html = inlineHtml;
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
          <span class="label">{b.title ?? 'Key result'}</span>
          <p>{@html html(b.text)}</p>
        </div>
      {:else if b.type === 'worked_example'}
        <div class="card example">
          <span class="label">Worked example · {b.title}</span>
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
          <code>{b.latex}</code>
          {#if b.number}<span class="eqno">({b.number})</span>{/if}
          {#if b.caption}<figcaption>{@html html(b.caption)}</figcaption>{/if}
        </figure>
      {:else if b.type === 'figure'}
        <figure class="figure" aria-label={b.alt}>
          <div class="fig-body">{b.text_fallback ?? b.alt}</div>
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
          <span class="label">{calloutLabel[b.variant] ?? 'Note'}</span>
          <p>{@html html(b.text)}</p>
        </aside>
      {:else if b.type === 'review_questions'}
        <section class="card">
          <span class="label">Review questions</span>
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
          <span class="label">Further reading</span>
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
          <span class="label">Summary</span>
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
  .label {
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
  .equation code {
    font-size: 15px;
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
