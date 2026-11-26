<script lang="ts">
  import { tick } from 'svelte';
  import {
    Search,
    Copy,
    Check,
    ArrowDown,
    X,
    FileText,
    TriangleAlert,
  } from '@lucide/svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import EmptyState from '$lib/components/ui/EmptyState.svelte';
  import SkeletonCard from '$lib/components/shared/SkeletonCard.svelte';
  import { useStudio } from '$lib/studio/lectureStudio.svelte';
  import { clock } from '$lib/studio/time';
  import SelectionToolbar, {
    type SelectionAction,
  } from './SelectionToolbar.svelte';

  let {
    onAction,
  }: {
    onAction: (
      action: SelectionAction,
      text: string,
      seconds: number | null
    ) => void;
  } = $props();

  const studio = useStudio();

  let query = $state('');
  let fontSize = $state(16);
  let copied = $state(false);
  let follow = $state(true);
  let scroller: HTMLDivElement | undefined = $state();
  let sel = $state<{
    x: number;
    y: number;
    text: string;
    seconds: number | null;
    placement: 'above' | 'below';
  } | null>(null);

  const term = $derived(query.trim().toLowerCase());
  const filtering = $derived(term.length > 0);

  // Which chapter each line falls in, so we can drop chapter headings in.
  const chapterIndexFor = (t: number | null) => {
    if (t == null) return -1;
    let found = -1;
    for (let i = 0; i < studio.chapters.length; i++)
      if (studio.chapters[i].start_timestamp_sec <= t) found = i;
    return found;
  };

  const rows = $derived.by(() => {
    let lastChapter = -2;
    const out: {
      i: number;
      t: number | null;
      text: string;
      heading: { n: number; title: string } | null;
    }[] = [];
    studio.lines.forEach((line, i) => {
      if (filtering && !line.text.toLowerCase().includes(term)) return;
      let heading: { n: number; title: string } | null = null;
      if (!filtering) {
        const c = chapterIndexFor(line.offset_sec);
        if (c !== lastChapter && c >= 0) {
          heading = { n: c + 1, title: studio.chapters[c].title };
        }
        if (c >= 0) lastChapter = c;
      }
      out.push({ i, t: line.offset_sec, text: line.text, heading });
    });
    return out;
  });

  function parts(text: string): { s: string; hit: boolean }[] {
    if (!term) return [{ s: text, hit: false }];
    const out: { s: string; hit: boolean }[] = [];
    const lower = text.toLowerCase();
    let at = 0;
    while (at < text.length) {
      const next = lower.indexOf(term, at);
      if (next < 0) {
        out.push({ s: text.slice(at), hit: false });
        break;
      }
      if (next > at) out.push({ s: text.slice(at, next), hit: false });
      out.push({ s: text.slice(next, next + term.length), hit: true });
      at = next + term.length;
    }
    return out;
  }

  // ── Follow the playhead ────────────────────────────────────
  const reduceMotion = () =>
    typeof matchMedia !== 'undefined' &&
    matchMedia('(prefers-reduced-motion: reduce)').matches;

  function scrollToActive(smooth = true) {
    const idx = studio.activeLineIndex;
    if (!scroller || idx < 0) return;
    const el = scroller.querySelector<HTMLElement>(`[data-idx="${idx}"]`);
    if (!el) return;
    const top = el.offsetTop - scroller.clientHeight * 0.32;
    scroller.scrollTo({
      top: Math.max(top, 0),
      behavior: smooth && !reduceMotion() ? 'smooth' : 'auto',
    });
  }

  $effect(() => {
    void studio.activeLineIndex;
    if (follow && !filtering) scrollToActive();
  });

  // Any explicit seek (scrubber, chapter, line) resumes following.
  $effect(() => {
    void studio.seekTick;
    follow = true;
    void tick().then(() => !filtering && scrollToActive());
  });

  // Opening at a moment (resume / search hit) happens before the lines exist,
  // so bring the line into view once they have loaded.
  $effect(() => {
    if (studio.transcriptStatus !== 'ready') return;
    void tick().then(() => !filtering && scrollToActive(false));
  });

  const pauseFollow = () => (follow = false);

  function jumpToLive() {
    follow = true;
    scrollToActive();
  }

  const showJump = $derived(
    !follow && !filtering && studio.engaged && studio.activeLineIndex >= 0
  );

  // ── Row interactions ───────────────────────────────────────
  function playFrom(t: number | null) {
    if (t == null) return;
    void studio.seek(t, { play: studio.hasAudio });
  }

  let clickTimer: ReturnType<typeof setTimeout> | undefined;
  function rowClick(event: MouseEvent, t: number | null) {
    clearTimeout(clickTimer);
    if ((event.target as HTMLElement).closest('button')) return;
    // A double-click (select a word) or a drag-select must not seek, so the
    // seek waits a beat and is dropped if a selection appeared.
    if (event.detail > 1) return;
    clickTimer = setTimeout(() => {
      if (window.getSelection()?.toString()) return;
      playFrom(t);
    }, 230);
  }

  async function copyAll() {
    const text = studio.lines
      .map(
        (l) =>
          `${l.offset_sec != null ? `[${clock(l.offset_sec)}] ` : ''}${l.text}`
      )
      .join('\n');
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
      studio.announce('Transcript copied');
      setTimeout(() => (copied = false), 1800);
    } catch {
      studio.announce('Copy is not available here');
    }
  }

  // ── Select to act ──────────────────────────────────────────
  function readSelection() {
    const s = window.getSelection();
    if (!s || s.isCollapsed || !scroller || s.rangeCount === 0) {
      sel = null;
      return;
    }
    const range = s.getRangeAt(0);
    if (!scroller.contains(range.commonAncestorContainer)) {
      sel = null;
      return;
    }
    const text = s.toString().replace(/\s+/g, ' ').trim();
    if (text.length < 2) {
      sel = null;
      return;
    }
    const rect = range.getBoundingClientRect();
    const startEl =
      range.startContainer instanceof Element
        ? range.startContainer
        : range.startContainer.parentElement;
    const row = startEl?.closest<HTMLElement>('[data-idx]');
    const idx = row ? Number(row.dataset.idx) : -1;
    const seconds = idx >= 0 ? (studio.lines[idx]?.offset_sec ?? null) : null;
    const coarse = matchMedia('(pointer: coarse)').matches;
    const below = coarse || rect.top < 64;
    const x = Math.min(
      Math.max(rect.left + rect.width / 2, 120),
      window.innerWidth - 120
    );
    sel = {
      x,
      y: below ? rect.bottom + 10 : rect.top - 10,
      text,
      seconds,
      placement: below ? 'below' : 'above',
    };
  }

  let selTimer: ReturnType<typeof setTimeout> | undefined;
  function onSelectionChange() {
    clearTimeout(selTimer);
    selTimer = setTimeout(readSelection, 120);
  }

  function runAction(action: SelectionAction) {
    if (!sel) return;
    onAction(action, sel.text, sel.seconds);
    window.getSelection()?.removeAllRanges();
    sel = null;
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && sel) {
      window.getSelection()?.removeAllRanges();
      sel = null;
    }
  }
</script>

<svelte:document onselectionchange={onSelectionChange} />
<svelte:window onkeydown={onKeydown} onresize={() => (sel = null)} />

<div class="transcript">
  <div class="toolbar">
    <div class="search">
      <Search size={15} />
      <input
        type="search"
        bind:value={query}
        placeholder="Search the transcript"
        aria-label="Search the transcript"
        autocomplete="off"
        spellcheck="false"
      />
      {#if query}
        <Button
          icon
          size="sm"
          class="clear"
          aria-label="Clear search"
          onclick={() => (query = '')}
        >
          <X size={14} />
        </Button>
      {/if}
    </div>
    {#if filtering}
      <span class="count" role="status"
        >{rows.length} {rows.length === 1 ? 'match' : 'matches'}</span
      >
    {/if}
    <div class="tools">
      <div class="font" role="group" aria-label="Text size">
        <Button
          variant="outline"
          icon
          aria-label="Smaller text"
          title="Smaller text"
          disabled={fontSize <= 13}
          onclick={() => (fontSize = Math.max(13, fontSize - 1))}
          >A<small>−</small></Button
        >
        <Button
          variant="outline"
          icon
          aria-label="Larger text"
          title="Larger text"
          disabled={fontSize >= 24}
          onclick={() => (fontSize = Math.min(24, fontSize + 1))}
          >A<small>+</small></Button
        >
      </div>
      <Button
        variant="outline"
        class="copy"
        onclick={copyAll}
        disabled={!studio.lines.length}
        title="Copy the full transcript"
      >
        {#if copied}<Check size={14} /><span>Copied</span>{:else}<Copy
            size={14}
          /><span>Copy</span>{/if}
      </Button>
    </div>
  </div>

  <div class="stage">
    {#if studio.transcriptStatus === 'loading'}
      <div class="pad">
        <SkeletonCard lines={4} label="Loading transcript…" />
      </div>
    {:else if studio.transcriptStatus === 'error'}
      <EmptyState
        tone="error"
        icon={TriangleAlert}
        title="Couldn’t load the transcript"
        description={studio.transcriptError}
      >
        <Button variant="outline" onclick={() => studio.loadTranscript()}
          >Try again</Button
        >
      </EmptyState>
    {:else if studio.lines.length === 0}
      <EmptyState
        icon={FileText}
        title="No transcript yet"
        description="Transcripts appear here once the lecturer’s captions have been saved."
      />
    {:else if rows.length === 0}
      <EmptyState
        icon={Search}
        title="No matches"
        description="Nothing in this lecture matches “{query}”."
      />
    {:else}
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <div
        class="scroller selectable-text"
        bind:this={scroller}
        role="region"
        aria-label="Transcript"
        tabindex="-1"
        style:font-size="{fontSize}px"
        onwheel={pauseFollow}
        ontouchmove={pauseFollow}
        onkeydown={(e) =>
          [
            'ArrowUp',
            'ArrowDown',
            'PageUp',
            'PageDown',
            'Home',
            'End',
          ].includes(e.key) && pauseFollow()}
      >
        {#each rows as row (row.i)}
          {#if row.heading}
            <div class="chapter-head" role="presentation">
              <span class="n">{row.heading.n}</span>
              <span class="t">{row.heading.title}</span>
              <span class="rule"></span>
            </div>
          {/if}
          {@const active = studio.activeLineIndex === row.i}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <div
            class="row"
            class:active
            class:seekable={row.t != null}
            data-idx={row.i}
            aria-current={active ? 'true' : undefined}
            onclick={(e) => rowClick(e, row.t)}
          >
            {#if row.t != null}
              <button
                type="button"
                class="stamp"
                aria-label="Play from {clock(row.t)}"
                title="Play from here"
                onclick={() => playFrom(row.t)}>{clock(row.t)}</button
              >
            {:else}
              <span class="stamp none" aria-hidden="true">·</span>
            {/if}
            <p class="line-text">
              {#each parts(row.text) as p}{#if p.hit}<mark>{p.s}</mark
                  >{:else}{p.s}{/if}{/each}
            </p>
          </div>
        {/each}
        <div class="end" aria-hidden="true"></div>
      </div>

      {#if showJump}
        <Button variant="accent" class="jump" onclick={jumpToLive}>
          <ArrowDown size={14} /> Jump to live position
        </Button>
      {/if}
    {/if}
  </div>
</div>

{#if sel}
  <SelectionToolbar
    x={sel.x}
    y={sel.y}
    placement={sel.placement}
    onAction={runAction}
  />
{/if}

<style>
  .transcript {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 0 10px;
    flex-wrap: wrap;
  }
  .search {
    position: relative;
    flex: 1 1 200px;
    min-width: 160px;
    display: flex;
    align-items: center;
  }
  .search :global(.clear) {
    position: absolute;
    right: 4px;
  }
  .font {
    display: flex;
    gap: 4px;
  }
  .font :global(.ui-btn) {
    gap: 1px;
    font-weight: 600;
  }
  .font :global(small) {
    font-size: 11px;
    margin-left: 1px;
  }
  .stage :global(.jump) {
    position: absolute;
    left: 50%;
    bottom: 16px;
    transform: translateX(-50%);
    box-shadow: var(--elev-3);
    animation: rise var(--motion-base) var(--ease-out);
  }
  .search :global(svg:first-child) {
    position: absolute;
    left: 12px;
    color: var(--text-tertiary);
    pointer-events: none;
  }
  .search input {
    width: 100%;
    height: var(--control-h);
    min-height: var(--control-h);
    padding: 0 34px 0 36px;
    background: var(--surface-inset);
    border: 1px solid var(--border-default);
    border-radius: 999px;
    color: var(--text-primary);
    font-family: var(--font-body);
    font-size: 13.5px;
    letter-spacing: 0;
    text-transform: none;
    margin: 0;
    transition:
      border-color var(--motion-fast) var(--ease-out),
      box-shadow var(--motion-fast) var(--ease-out);
  }
  .search input::placeholder {
    color: var(--text-tertiary);
    font-style: normal;
  }
  .search input:focus {
    outline: none;
    border-color: var(--border-strong);
    box-shadow: var(--focus-ring);
  }
  .search input::-webkit-search-cancel-button {
    display: none;
  }
  .count {
    font-size: 12.5px;
    color: var(--text-secondary);
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
  }

  .stage {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--surface-inset);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    overflow: hidden;
  }
  .pad {
    padding: 16px;
  }
  .scroller {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: 8px 8px 40px;
    scrollbar-width: thin;
    scrollbar-color: var(--color-cork-border) transparent;
    user-select: text;
    -webkit-user-select: text;
    outline: none;
  }
  .chapter-head {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 20px 12px 8px;
  }
  .chapter-head:first-child {
    padding-top: 10px;
  }
  .chapter-head .n {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--surface-3);
    color: var(--text-secondary);
    font-size: 11px;
    font-weight: 600;
  }
  .chapter-head .t {
    font-family: var(--font-body);
    font-size: 11.5px;
    font-weight: 600;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--text-tertiary);
  }
  .chapter-head .rule {
    flex: 1;
    height: 1px;
    background: var(--border-subtle);
  }
  .row {
    position: relative;
    display: grid;
    grid-template-columns: 52px minmax(0, 1fr);
    gap: 10px;
    padding: 8px 12px 8px 8px;
    border-radius: var(--radius-md);
    transition: background var(--motion-base) var(--ease-out);
  }
  .row.seekable {
    cursor: pointer;
  }
  .row.seekable:hover {
    background: var(--surface-hover);
  }
  .row.active {
    background: var(--surface-accent);
  }
  .row.active::before {
    content: '';
    position: absolute;
    left: 0;
    top: 8px;
    bottom: 8px;
    width: 3px;
    border-radius: 3px;
    background: var(--color-ember-accent);
  }
  .stamp {
    align-self: start;
    min-width: 0;
    width: 100%;
    height: 28px;
    min-height: 28px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-tertiary);
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    text-align: center;
    line-height: 28px;
    transition: color var(--motion-fast) var(--ease-out);
  }
  .stamp:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }
  .stamp.none {
    cursor: default;
  }
  .row:hover .stamp:not(.none),
  .row.active .stamp {
    color: var(--text-accent);
  }
  .line-text {
    margin: 0;
    line-height: 1.65;
    color: var(--text-primary);
    word-break: break-word;
    text-wrap: pretty;
  }
  .row:not(.active) .line-text {
    color: var(--text-secondary);
  }
  .row.active .line-text,
  .row:hover .line-text {
    color: var(--text-primary);
  }
  mark {
    background: rgba(255, 140, 74, 0.3);
    color: var(--text-primary);
    border-radius: 3px;
    padding: 0 1px;
    box-shadow: 0 0 0 1px rgba(255, 140, 74, 0.35);
  }
  .scroller ::selection {
    background: rgba(220, 80, 0, 0.5);
    color: var(--text-primary);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translate(-50%, 8px);
    }
  }
  @media (max-width: 720px) {
    .row {
      grid-template-columns: 46px minmax(0, 1fr);
      gap: 6px;
    }
    .tools :global(.copy span) {
      display: none;
    }
    .tools :global(.copy) {
      padding: 0 12px;
    }
  }
  @media (pointer: coarse) {
    .stamp {
      height: 44px;
      min-height: 44px;
      line-height: 44px;
    }
  }
</style>
