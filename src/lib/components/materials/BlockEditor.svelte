<script lang="ts">
  import type { Block, CalloutVariant } from '$lib/api/materials';

  let {
    block,
    onChange,
  }: {
    block: Block;
    /** Receives only the fields that changed. */
    onChange: (patch: Partial<Block>) => void;
  } = $props();

  // `Partial<Block>` over a union is awkward to build field by field.
  const set = (patch: Record<string, unknown>) =>
    onChange(patch as Partial<Block>);

  const variants: CalloutVariant[] = [
    'note',
    'tip',
    'warning',
    'misconception',
  ];

  function setAt<T>(list: T[], i: number, value: T): T[] {
    return list.map((x, j) => (j === i ? value : x));
  }
  const dropAt = <T,>(list: T[], i: number) => list.filter((_, j) => j !== i);
</script>

{#snippet area(
  label: string,
  value: string,
  change: (v: string) => void,
  rows = 3,
  mono = false
)}
  <label class="field">
    <span>{label}</span>
    <textarea
      {rows}
      class:mono
      {value}
      oninput={(e) => change(e.currentTarget.value)}></textarea>
  </label>
{/snippet}

{#snippet line(label: string, value: string, change: (v: string) => void)}
  <label class="field">
    <span>{label}</span>
    <input type="text" {value} oninput={(e) => change(e.currentTarget.value)} />
  </label>
{/snippet}

<div class="editor">
  {#if block.type === 'heading'}
    <div class="row">
      <label class="field narrow">
        <span>Level</span>
        <select
          value={block.level}
          onchange={(e) => set({ level: Number(e.currentTarget.value) })}
        >
          {#each [1, 2, 3, 4] as l}<option value={l}>H{l}</option>{/each}
        </select>
      </label>
      {@render line('Heading', block.text, (v) => set({ text: v }))}
    </div>
  {:else if block.type === 'paragraph'}
    {@render area('Paragraph', block.text, (v) => set({ text: v }), 4)}
  {:else if block.type === 'definition'}
    {@render line('Term', block.term, (v) => set({ term: v }))}
    {@render area('Definition', block.text, (v) => set({ text: v }))}
  {:else if block.type === 'key_result'}
    {@render line('Title (optional)', block.title ?? '', (v) =>
      set({ title: v || undefined })
    )}
    {@render area('Statement', block.text, (v) => set({ text: v }))}
  {:else if block.type === 'worked_example'}
    {@render line('Title', block.title, (v) => set({ title: v }))}
    {@render area('Problem', block.problem, (v) => set({ problem: v }))}
    <div class="list">
      <span class="lbl">Steps</span>
      {#each block.steps as step, i}
        <div class="item">
          <textarea
            rows="2"
            value={step}
            aria-label={`Step ${i + 1}`}
            oninput={(e) =>
              set({ steps: setAt(block.steps, i, e.currentTarget.value) })}
          ></textarea>
          <button
            type="button"
            class="mini"
            aria-label={`Remove step ${i + 1}`}
            onclick={() => set({ steps: dropAt(block.steps, i) })}
          >
            ✕
          </button>
        </div>
      {/each}
      <button
        type="button"
        class="add"
        onclick={() => set({ steps: [...block.steps, ''] })}
      >
        + Add step
      </button>
    </div>
    {@render line('Answer (optional)', block.answer ?? '', (v) =>
      set({ answer: v || undefined })
    )}
  {:else if block.type === 'equation'}
    {@render area('LaTeX', block.latex, (v) => set({ latex: v }), 2, true)}
    {@render line('Caption (optional)', block.caption ?? '', (v) =>
      set({ caption: v || undefined })
    )}
  {:else if block.type === 'figure'}
    {@render line('Caption', block.caption, (v) => set({ caption: v }))}
    {@render line('Alt text', block.alt, (v) => set({ alt: v }))}
    {@render area('Text description', block.text_fallback ?? '', (v) =>
      set({ text_fallback: v || undefined })
    )}
  {:else if block.type === 'table'}
    {@render line('Caption (optional)', block.caption ?? '', (v) =>
      set({ caption: v || undefined })
    )}
    <div class="grid-wrap">
      <table class="grid">
        <thead>
          <tr>
            {#each block.header as h, c}
              <th>
                <input
                  value={h}
                  aria-label={`Header ${c + 1}`}
                  oninput={(e) =>
                    set({
                      header: setAt(block.header, c, e.currentTarget.value),
                    })}
                />
              </th>
            {/each}
            <th class="ctl">
              <button
                type="button"
                class="mini"
                aria-label="Add column"
                onclick={() =>
                  set({
                    header: [...block.header, ''],
                    rows: block.rows.map((r) => [...r, '']),
                  })}
              >
                +
              </button>
            </th>
          </tr>
        </thead>
        <tbody>
          {#each block.rows as row, r}
            <tr>
              {#each row as cell, c}
                <td>
                  <input
                    value={cell}
                    aria-label={`Row ${r + 1}, column ${c + 1}`}
                    oninput={(e) =>
                      set({
                        rows: setAt(
                          block.rows,
                          r,
                          setAt(row, c, e.currentTarget.value)
                        ),
                      })}
                  />
                </td>
              {/each}
              <td class="ctl">
                <button
                  type="button"
                  class="mini"
                  aria-label={`Remove row ${r + 1}`}
                  onclick={() => set({ rows: dropAt(block.rows, r) })}
                >
                  ✕
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    <div class="row">
      <button
        type="button"
        class="add"
        onclick={() =>
          set({ rows: [...block.rows, block.header.map(() => '')] })}
      >
        + Add row
      </button>
      {#if block.header.length > 1}
        <button
          type="button"
          class="add"
          onclick={() =>
            set({
              header: block.header.slice(0, -1),
              rows: block.rows.map((r) => r.slice(0, -1)),
            })}
        >
          − Remove last column
        </button>
      {/if}
    </div>
  {:else if block.type === 'callout'}
    <label class="field narrow">
      <span>Type</span>
      <select
        value={block.variant}
        onchange={(e) => set({ variant: e.currentTarget.value })}
      >
        {#each variants as v}<option value={v}>{v}</option>{/each}
      </select>
    </label>
    {@render area('Text', block.text, (v) => set({ text: v }))}
  {:else if block.type === 'review_questions'}
    <div class="list">
      {#each block.questions as q, i}
        <div class="qa">
          <textarea
            rows="2"
            value={q.question}
            aria-label={`Question ${i + 1}`}
            placeholder="Question"
            oninput={(e) =>
              set({
                questions: setAt(block.questions, i, {
                  ...q,
                  question: e.currentTarget.value,
                }),
              })}></textarea>
          <textarea
            rows="2"
            value={q.answer ?? ''}
            aria-label={`Answer ${i + 1}`}
            placeholder="Answer (optional)"
            oninput={(e) =>
              set({
                questions: setAt(block.questions, i, {
                  ...q,
                  answer: e.currentTarget.value || undefined,
                }),
              })}></textarea>
          <button
            type="button"
            class="mini"
            aria-label={`Remove question ${i + 1}`}
            onclick={() => set({ questions: dropAt(block.questions, i) })}
          >
            ✕
          </button>
        </div>
      {/each}
      <button
        type="button"
        class="add"
        onclick={() =>
          set({ questions: [...block.questions, { question: '' }] })}
      >
        + Add question
      </button>
    </div>
  {:else if block.type === 'further_reading'}
    <div class="list">
      {#each block.items as it, i}
        <div class="qa">
          <input
            value={it.title}
            aria-label={`Title ${i + 1}`}
            placeholder="Title"
            oninput={(e) =>
              set({
                items: setAt(block.items, i, {
                  ...it,
                  title: e.currentTarget.value,
                }),
              })}
          />
          <input
            value={it.note ?? ''}
            aria-label={`Note ${i + 1}`}
            placeholder="Note (optional)"
            oninput={(e) =>
              set({
                items: setAt(block.items, i, {
                  ...it,
                  note: e.currentTarget.value || undefined,
                }),
              })}
          />
          <button
            type="button"
            class="mini"
            aria-label={`Remove item ${i + 1}`}
            onclick={() => set({ items: dropAt(block.items, i) })}
          >
            ✕
          </button>
        </div>
      {/each}
      <button
        type="button"
        class="add"
        onclick={() => set({ items: [...block.items, { title: '' }] })}
      >
        + Add item
      </button>
    </div>
  {:else if block.type === 'summary'}
    <div class="list">
      {#each block.points as pt, i}
        <div class="item">
          <textarea
            rows="2"
            value={pt}
            aria-label={`Point ${i + 1}`}
            oninput={(e) =>
              set({ points: setAt(block.points, i, e.currentTarget.value) })}
          ></textarea>
          <button
            type="button"
            class="mini"
            aria-label={`Remove point ${i + 1}`}
            onclick={() => set({ points: dropAt(block.points, i) })}
          >
            ✕
          </button>
        </div>
      {/each}
      <button
        type="button"
        class="add"
        onclick={() => set({ points: [...block.points, ''] })}
      >
        + Add point
      </button>
    </div>
  {/if}
</div>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .row {
    display: flex;
    gap: 8px;
    align-items: flex-end;
    flex-wrap: wrap;
  }
  .row .field:not(.narrow) {
    flex: 1;
    min-width: 200px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .field > span,
  .lbl {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--text-tertiary);
  }
  input,
  textarea,
  select {
    width: 100%;
    padding: 7px 9px;
    border: 1px solid var(--border-default);
    border-radius: 8px;
    background: var(--surface-inset);
    color: var(--text-primary);
    font: inherit;
    font-size: 14px;
  }
  textarea {
    resize: vertical;
    line-height: 1.5;
  }
  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }
  input:focus-visible,
  textarea:focus-visible,
  select:focus-visible {
    outline: 2px solid var(--text-accent);
    outline-offset: 1px;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .item,
  .qa {
    display: flex;
    gap: 6px;
    align-items: flex-start;
  }
  .qa {
    flex-wrap: wrap;
  }
  .qa > :not(button) {
    flex: 1;
    min-width: 180px;
  }
  .mini,
  .add {
    flex: none;
    border: 1px solid var(--border-default);
    border-radius: 8px;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    font: inherit;
    font-size: 12.5px;
  }
  .mini {
    width: 30px;
    height: 30px;
  }
  .add {
    align-self: flex-start;
    padding: 5px 10px;
  }
  .mini:hover,
  .add:hover {
    color: var(--text-primary);
    border-color: var(--border-strong);
  }
  .grid-wrap {
    overflow-x: auto;
  }
  .grid {
    border-collapse: collapse;
  }
  .grid td,
  .grid th {
    padding: 2px;
  }
  .grid input {
    min-width: 110px;
  }
  .ctl {
    width: 34px;
  }
  .narrow {
    width: 130px;
  }
</style>
