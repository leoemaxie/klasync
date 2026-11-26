<script lang="ts">
  import { onDestroy } from 'svelte';
  import {
    ArrowDown,
    ArrowUp,
    Lock,
    LockOpen,
    Plus,
    RefreshCw,
    Trash2,
    TriangleAlert,
    Eye,
    Pencil,
  } from '@lucide/svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import BlockEditor from './BlockEditor.svelte';
  import MaterialReader from './MaterialReader.svelte';
  import {
    ADDABLE_BLOCKS,
    blockLabel,
    docFingerprint,
    formatTime,
    insertBlock,
    makeBlock,
    moveBlock,
    moveSection,
    outline,
    removeBlock,
    setLocked,
    updateBlock,
    validateDoc,
    type AddableType,
  } from '$lib/materials/doc';
  import {
    regenerateMaterialSection,
    saveMaterialDraft,
    type Block,
    type MaterialDoc,
  } from '$lib/api/materials';

  let {
    sessionCode,
    initialDoc,
    initialRev,
    transcript = [],
    onRegenerating,
    onConflict,
  }: {
    sessionCode: string;
    initialDoc: MaterialDoc;
    initialRev: number;
    /** Lines of the lecture transcript, shown beside the selected block. */
    transcript?: { offset_sec: number | null; text: string }[];
    /** A section rewrite was queued; the parent should poll for the result. */
    onRegenerating?: () => void;
    /** The server has a newer draft (another tab, or a regenerate finished). */
    onConflict?: () => void;
  } = $props();

  // Seeded once; the parent remounts this component to load a newer draft.
  // svelte-ignore state_referenced_locally
  let doc = $state<MaterialDoc>(structuredClone($state.snapshot(initialDoc)));
  // svelte-ignore state_referenced_locally
  let rev = $state(initialRev);
  // svelte-ignore state_referenced_locally
  let savedPrint = $state(docFingerprint(initialDoc));
  let saveState = $state<'saved' | 'dirty' | 'saving' | 'error' | 'conflict'>(
    'saved'
  );
  let saveError = $state('');
  let mode = $state<'edit' | 'preview'>('edit');
  let selectedId = $state<string | null>(null);
  let addType = $state<AddableType>('paragraph');
  let rewriteFor = $state<string | null>(null);
  let instruction = $state('');
  let rewriteBusy = $state(false);
  let rewriteError = $state('');

  const problems = $derived(validateDoc(doc));
  const problemsById = $derived.by(() => {
    const m = new Map<string, string[]>();
    for (const p of problems) {
      if (!p.blockId) continue;
      m.set(p.blockId, [...(m.get(p.blockId) ?? []), p.message]);
    }
    return m;
  });
  const toc = $derived(outline(doc));

  // ---- autosave -----------------------------------------------------------

  let timer: ReturnType<typeof setTimeout> | undefined;
  let inflight = false;

  function change(next: MaterialDoc) {
    doc = next;
    if (saveState === 'conflict') return;
    saveState = 'dirty';
    clearTimeout(timer);
    timer = setTimeout(() => void save(), 1200);
  }

  async function save() {
    clearTimeout(timer);
    if (inflight || saveState === 'conflict') return;
    const print = docFingerprint(doc);
    if (print === savedPrint) {
      saveState = 'saved';
      return;
    }
    inflight = true;
    saveState = 'saving';
    try {
      const res = await saveMaterialDraft(
        sessionCode,
        $state.snapshot(doc) as MaterialDoc,
        rev
      );
      rev = res.draft_rev;
      savedPrint = print;
      // Edits made while the request was in flight still need saving.
      saveState = docFingerprint(doc) === print ? 'saved' : 'dirty';
      if (saveState === 'dirty') timer = setTimeout(() => void save(), 400);
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Could not save';
      if (/stale|conflict|409|newer|changed/i.test(msg)) {
        saveState = 'conflict';
        onConflict?.();
      } else {
        saveState = 'error';
        saveError = msg;
        timer = setTimeout(() => void save(), 8000);
      }
    } finally {
      inflight = false;
    }
  }

  /** Flush pending edits now (e.g. before publishing or exporting). */
  export async function flush(): Promise<boolean> {
    if (saveState === 'dirty' || saveState === 'error') await save();
    return saveState === 'saved';
  }

  export const isValid = () => problems.length === 0;

  onDestroy(() => {
    clearTimeout(timer);
    if (saveState === 'dirty') void save();
  });

  // ---- block operations ---------------------------------------------------

  const patch = (id: string, p: Partial<Block>) =>
    change(updateBlock(doc, id, p));

  function addBlock(after: number) {
    const b = makeBlock(addType);
    change(insertBlock(doc, after + 1, b));
    selectedId = b.id;
  }

  function remove(b: Block) {
    if (b.type === 'heading' || b.locked) {
      if (!confirm(`Delete “${blockLabel(b) || 'this block'}”?`)) return;
    }
    if (selectedId === b.id) selectedId = null;
    change(removeBlock(doc, b.id));
  }

  function move(b: Block, delta: -1 | 1, whole: boolean) {
    change(
      whole && b.type === 'heading'
        ? moveSection(doc, b.id, delta)
        : moveBlock(doc, b.id, delta)
    );
  }

  async function rewrite(headingId: string) {
    rewriteBusy = true;
    rewriteError = '';
    try {
      // The server rewrites from its saved draft, so save first.
      if (!(await flush())) throw new Error('Fix save errors before rewriting');
      await regenerateMaterialSection(
        sessionCode,
        headingId,
        instruction.trim() || undefined
      );
      rewriteFor = null;
      instruction = '';
      onRegenerating?.();
    } catch (e) {
      rewriteError = e instanceof Error ? e.message : 'Could not rewrite';
    } finally {
      rewriteBusy = false;
    }
  }

  // ---- transcript beside the block ------------------------------------------

  const selected = $derived(
    doc.blocks.find((b) => b.id === selectedId) ?? null
  );
  const sourceLines = $derived.by(() => {
    const ranges = selected?.source_ranges ?? [];
    if (!ranges.length) return [];
    return transcript.filter(
      (l) =>
        l.offset_sec != null &&
        ranges.some(
          (r) => l.offset_sec! >= r.start_sec && l.offset_sec! <= r.end_sec
        )
    );
  });

  function jump(id: string) {
    selectedId = id;
    document
      .querySelector(`[data-edit-id="${CSS.escape(id)}"]`)
      ?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }

  const statusText = $derived(
    {
      saved: 'All changes saved',
      dirty: 'Unsaved changes…',
      saving: 'Saving…',
      error: `Save failed: ${saveError}. Retrying…`,
      conflict: 'This draft changed elsewhere',
    }[saveState]
  );
</script>

<div class="editor-shell">
  <div class="mat-bar">
    <div class="status" data-state={saveState} role="status" aria-live="polite">
      {statusText}
    </div>
    {#if saveState === 'error' || saveState === 'dirty'}
      <Button size="sm" variant="outline" onclick={() => void save()}
        >Save now</Button
      >
    {/if}
    <div class="spacer"></div>
    {#if problems.length}
      <span class="problems" title={problems.map((p) => p.message).join('\n')}>
        <TriangleAlert size={14} />
        {problems.length} to fix before publishing
      </span>
    {/if}
    <Button
      size="sm"
      variant="outline"
      onclick={() => (mode = mode === 'edit' ? 'preview' : 'edit')}
    >
      {#if mode === 'edit'}<Eye size={14} /> Preview{:else}<Pencil size={14} /> Edit{/if}
    </Button>
  </div>

  {#if saveState === 'conflict'}
    <p class="banner" role="alert">
      A newer version of this draft exists (another tab, or a rewrite finished).
      Reload to see it; your unsaved edits here are not saved.
    </p>
  {/if}

  {#if mode === 'preview'}
    <div class="preview"><MaterialReader {doc} /></div>
  {:else}
    <div class="layout" class:with-source={!!sourceLines.length}>
      <nav class="toc" aria-label="Sections">
        <h3>Sections</h3>
        {#if toc.length === 0}<p class="muted">No headings yet.</p>{/if}
        <ul>
          {#each toc as t}
            <li style:padding-left={`${(t.level - 1) * 10}px`}>
              <button type="button" onclick={() => jump(t.id)}>{t.text}</button>
            </li>
          {/each}
        </ul>
      </nav>

      <div class="blocks">
        <label class="field">
          <span>Title</span>
          <input
            type="text"
            value={doc.title}
            oninput={(e) => change({ ...doc, title: e.currentTarget.value })}
          />
        </label>
        <label class="field">
          <span>Subtitle</span>
          <input
            type="text"
            value={doc.subtitle ?? ''}
            oninput={(e) =>
              change({ ...doc, subtitle: e.currentTarget.value || undefined })}
          />
        </label>
        <label class="field">
          <span>Learning objectives (one per line)</span>
          <textarea
            rows="3"
            value={doc.meta.objectives.join('\n')}
            oninput={(e) =>
              change({
                ...doc,
                meta: {
                  ...doc.meta,
                  objectives: e.currentTarget.value
                    .split('\n')
                    .map((l) => l.trim())
                    .filter(Boolean),
                },
              })}></textarea>
        </label>

        {#each doc.blocks as b, i (b.id)}
          {@const errs = problemsById.get(b.id)}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <section
            class="block"
            class:selected={selectedId === b.id}
            class:bad={!!errs}
            data-edit-id={b.id}
            onclick={() => (selectedId = b.id)}
          >
            <header class="block-head">
              <span class="type">{b.type.replace('_', ' ')}</span>
              {#if b.origin === 'supplemented'}<span class="tag">added</span
                >{/if}
              {#if b.low_confidence}<span class="tag warn">low confidence</span
                >{/if}
              {#if b.source_ranges?.length}
                <span class="tag"
                  >{formatTime(b.source_ranges[0].start_sec)}</span
                >
              {/if}
              <span class="spacer"></span>
              <Button
                icon
                size="sm"
                aria-label="Move up"
                onclick={() => move(b, -1, true)}
                disabled={i === 0}><ArrowUp size={14} /></Button
              >
              <Button
                icon
                size="sm"
                aria-label="Move down"
                onclick={() => move(b, 1, true)}
                disabled={i === doc.blocks.length - 1}
                ><ArrowDown size={14} /></Button
              >
              <Button
                icon
                size="sm"
                pressed={b.locked}
                aria-label={b.locked
                  ? 'Unlock (allow regeneration to replace)'
                  : 'Lock (keep through regeneration)'}
                title={b.locked
                  ? 'Locked: kept when the lecture is regenerated'
                  : 'Unlocked: may be replaced when regenerated'}
                onclick={() => change(setLocked(doc, b.id, !b.locked))}
              >
                {#if b.locked}<Lock size={14} />{:else}<LockOpen
                    size={14}
                  />{/if}
              </Button>
              {#if b.type === 'heading'}
                <Button
                  icon
                  size="sm"
                  aria-label="Rewrite this section"
                  onclick={() =>
                    (rewriteFor = rewriteFor === b.id ? null : b.id)}
                  ><RefreshCw size={14} /></Button
                >
              {/if}
              <Button
                icon
                size="sm"
                aria-label="Delete block"
                onclick={() => remove(b)}><Trash2 size={14} /></Button
              >
            </header>

            {#if rewriteFor === b.id}
              <div class="rewrite">
                <label class="field">
                  <span>How should this section change? (optional)</span>
                  <input
                    type="text"
                    bind:value={instruction}
                    placeholder="e.g. add a worked example, simplify the language"
                  />
                </label>
                <Button
                  size="sm"
                  variant="accent"
                  disabled={rewriteBusy}
                  onclick={() => void rewrite(b.id)}
                >
                  {rewriteBusy ? 'Queuing…' : 'Rewrite section'}
                </Button>
                <p class="muted">Locked blocks in the section are kept.</p>
                {#if rewriteError}<p class="err" role="alert">
                    {rewriteError}
                  </p>{/if}
              </div>
            {/if}

            <BlockEditor block={b} onChange={(p) => patch(b.id, p)} />
            {#if errs}
              <ul class="errs">
                {#each errs as m}<li>{m}</li>{/each}
              </ul>
            {/if}

            <div class="insert">
              <select bind:value={addType} aria-label="Block type to add">
                {#each ADDABLE_BLOCKS as a}<option value={a.type}
                    >{a.label}</option
                  >{/each}
              </select>
              <Button size="sm" variant="outline" onclick={() => addBlock(i)}
                ><Plus size={14} /> Add below</Button
              >
            </div>
          </section>
        {:else}
          <p class="muted">Nothing here yet.</p>
          <div class="insert">
            <select bind:value={addType} aria-label="Block type to add">
              {#each ADDABLE_BLOCKS as a}<option value={a.type}
                  >{a.label}</option
                >{/each}
            </select>
            <Button size="sm" variant="outline" onclick={() => addBlock(-1)}
              ><Plus size={14} /> Add block</Button
            >
          </div>
        {/each}
      </div>

      {#if sourceLines.length}
        <aside class="source" aria-label="Transcript for the selected block">
          <h3>From the lecture</h3>
          {#each sourceLines as l}
            <p>
              <span class="time">{formatTime(l.offset_sec ?? 0)}</span>
              {l.text}
            </p>
          {/each}
        </aside>
      {/if}
    </div>
  {/if}
</div>

<style>
  .editor-shell {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .mat-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .spacer {
    flex: 1;
  }
  .status {
    font-size: 12.5px;
    color: var(--text-tertiary);
  }
  .status[data-state='error'],
  .status[data-state='conflict'] {
    color: #ff9b7a;
  }
  .problems {
    display: inline-flex;
    gap: 5px;
    align-items: center;
    font-size: 12.5px;
    color: #f0b25c;
  }
  .banner,
  .err {
    margin: 0;
    padding: 8px 12px;
    border: 1px solid #c9892b;
    border-radius: 8px;
    font-size: 13px;
    color: #f0b25c;
  }
  .err {
    border-color: #ff9b7a;
    color: #ff9b7a;
  }
  .layout {
    display: grid;
    grid-template-columns: 190px minmax(0, 1fr);
    gap: 16px;
    align-items: start;
  }
  .layout.with-source {
    grid-template-columns: 190px minmax(0, 1fr) 280px;
  }
  .toc,
  .source {
    position: sticky;
    top: calc(var(--nav-height, 56px) + 70px);
    max-height: 70vh;
    overflow: auto;
    font-size: 13px;
  }
  .toc h3,
  .source h3 {
    margin: 0 0 6px;
    font-size: 11px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-tertiary);
  }
  .toc ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .toc button {
    width: 100%;
    padding: 4px 6px;
    text-align: left;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--text-secondary);
    font: inherit;
    cursor: pointer;
  }
  .toc button:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }
  .source p {
    margin: 0 0 6px;
    color: var(--text-secondary);
  }
  .time {
    color: var(--text-accent);
    font-variant-numeric: tabular-nums;
  }
  .blocks {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-width: 0;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .field > span {
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
  .block {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px;
    border: 1px solid var(--border-subtle);
    border-radius: 12px;
    background: var(--surface-1);
  }
  .block.selected {
    border-color: var(--border-accent);
  }
  .block.bad {
    border-color: #c9892b;
  }
  .block-head {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-wrap: wrap;
  }
  .type {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-accent);
  }
  .tag {
    padding: 0 7px;
    border: 1px solid var(--border-default);
    border-radius: 999px;
    font-size: 11px;
    color: var(--text-tertiary);
  }
  .tag.warn {
    color: #f0b25c;
    border-color: #c9892b;
  }
  .rewrite {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px;
    border: 1px dashed var(--border-default);
    border-radius: 8px;
  }
  .errs {
    margin: 0;
    padding-left: 18px;
    font-size: 12.5px;
    color: #f0b25c;
  }
  .insert {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .insert select {
    width: auto;
  }
  .muted {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-tertiary);
  }
  .preview {
    padding: 8px 0;
  }
  @media (max-width: 900px) {
    .layout,
    .layout.with-source {
      grid-template-columns: minmax(0, 1fr);
    }
    .toc,
    .source {
      position: static;
      max-height: none;
    }
  }
</style>
