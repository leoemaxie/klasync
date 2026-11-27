<script lang="ts">
  import { onMount } from 'svelte';
  import {
    Box,
    LayoutGrid,
    Maximize,
    Plus,
    Minus,
    Play,
    Pause,
    SkipBack,
    SkipForward,
    X,
    Radio,
  } from '@lucide/svelte';
  import type { SessionVisualization } from '$lib/api/aiStudy';
  import ConceptMap from '$lib/components/archive/ConceptMap.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Segmented from '$lib/components/ui/Segmented.svelte';
  import { useStudio } from '$lib/studio/lectureStudio.svelte';
  import { clock } from '$lib/studio/time';
  import { colorForGroup, groupsOf } from '$lib/utils/conceptPalette';
  import {
    activeConcepts,
    indexMentions,
    tourOrder,
  } from '$lib/studio/conceptMentions';
  import ConceptMap3D from './ConceptMap3D.svelte';

  let { visualization }: { visualization: SessionVisualization } = $props();

  const studio = useStudio();

  type Mode = '3d' | '2d';
  const MODE_KEY = 'klasync-map-mode';

  // ── Capability + preference ────────────────────────────────
  let reduceMotion = $state(false);
  let lowPower = $state(false);
  let webglFailed = $state(false);
  let userMode = $state<Mode | null>(null);

  onMount(() => {
    const mq = matchMedia('(prefers-reduced-motion: reduce)');
    reduceMotion = mq.matches;
    const onChange = () => (reduceMotion = mq.matches);
    mq.addEventListener('change', onChange);
    const nav = navigator as Navigator & { deviceMemory?: number };
    lowPower =
      (nav.hardwareConcurrency ?? 8) <= 2 || (nav.deviceMemory ?? 8) <= 2;
    try {
      const saved = localStorage.getItem(MODE_KEY);
      if (saved === '2d' || saved === '3d') userMode = saved;
    } catch {
      /* storage unavailable */
    }
    return () => mq.removeEventListener('change', onChange);
  });

  /** 2D is the default for reduced-motion and low-power devices; a saved
   *  choice always wins, and a failed WebGL start forces 2D. */
  const mode = $derived<Mode>(
    webglFailed ? '2d' : (userMode ?? (reduceMotion || lowPower ? '2d' : '3d'))
  );

  function setMode(next: Mode) {
    userMode = next;
    try {
      localStorage.setItem(MODE_KEY, next);
    } catch {
      /* storage unavailable */
    }
  }

  // ── Data ──────────────────────────────────────────────────
  const nodes = $derived(visualization.spec.nodes ?? []);
  const edges = $derived(visualization.spec.edges ?? []);
  const groups = $derived(groupsOf(nodes));
  const colorFor = (group: string) => colorForGroup(groups, group);
  const byId = $derived(new Map(nodes.map((n) => [n.id, n])));
  const sortedNodes = $derived(
    [...nodes].sort(
      (a, b) => b.importance - a.importance || a.label.localeCompare(b.label)
    )
  );

  const mentions = $derived(indexMentions(nodes, studio.lines));
  const activeIds = $derived(
    studio.engaged
      ? activeConcepts(mentions, studio.lines, studio.time)
      : new Set<string>()
  );
  const order = $derived(tourOrder(nodes, mentions));

  // ── Selection ─────────────────────────────────────────────
  let selectedId = $state<string | null>(null);
  const selected = $derived(selectedId ? (byId.get(selectedId) ?? null) : null);

  const connections = $derived.by(() => {
    if (!selectedId) return [];
    const out: {
      id: string;
      label: string;
      relation: string;
      outgoing: boolean;
    }[] = [];
    for (const e of edges) {
      if (e.from === selectedId && byId.has(e.to))
        out.push({
          id: e.to,
          label: byId.get(e.to)!.label,
          relation: e.label,
          outgoing: true,
        });
      else if (e.to === selectedId && byId.has(e.from))
        out.push({
          id: e.from,
          label: byId.get(e.from)!.label,
          relation: e.label,
          outgoing: false,
        });
    }
    return out;
  });

  const moments = $derived.by(() => {
    if (!selectedId) return [];
    const out: { t: number; text: string }[] = [];
    for (const i of mentions.lines.get(selectedId) ?? []) {
      const line = studio.lines[i];
      if (line?.offset_sec != null)
        out.push({ t: line.offset_sec, text: line.text });
    }
    return out;
  });
  const mentionCount = $derived(
    selectedId ? (mentions.lines.get(selectedId)?.length ?? 0) : 0
  );

  function select(id: string | null, source: 'user' | 'tour' = 'user') {
    if (source === 'user' && tourOn) stopTour();
    selectedId = id;
    if (id && source === 'user') {
      const n = byId.get(id);
      const c =
        (n && edges.filter((e) => e.from === id || e.to === id).length) ?? 0;
      studio.announce(
        `${n?.label} selected, ${c} connection${c === 1 ? '' : 's'}`
      );
    }
  }

  // ── Camera controls (3D) ──────────────────────────────────
  let touched = $state(false);
  let follow = $state(false);
  let resetTick = $state(0);
  let zoomTick = $state(0);
  let zoomDir = $state<1 | -1>(1);
  const zoom = (d: 1 | -1) => {
    zoomDir = d;
    zoomTick++;
  };

  // ── Tour ──────────────────────────────────────────────────
  let tourOn = $state(false);
  let tourIndex = $state(-1);
  let tourTimer: ReturnType<typeof setInterval> | undefined;
  const STEP_MS = 4600;

  function tourStep(delta: 1 | -1) {
    if (!order.length) return;
    tourIndex = (tourIndex + delta + order.length) % order.length;
    select(order[tourIndex], 'tour');
  }
  function startTour() {
    if (!order.length) return;
    tourOn = true;
    if (tourIndex < 0 || order[tourIndex] !== selectedId) tourIndex = -1;
    tourStep(1);
    clearInterval(tourTimer);
    tourTimer = setInterval(() => tourStep(1), STEP_MS);
    studio.announce('Tour started');
  }
  function stopTour() {
    tourOn = false;
    clearInterval(tourTimer);
  }
  $effect(() => () => clearInterval(tourTimer));

  // ── Keyboard on the map surface ───────────────────────────
  function onMapKeydown(event: KeyboardEvent) {
    const key = event.key;
    const ids = sortedNodes.map((n) => n.id);
    if (!ids.length) return;
    const i = selectedId ? ids.indexOf(selectedId) : -1;
    if (key === 'ArrowRight' || key === 'ArrowDown') {
      event.preventDefault();
      select(ids[(i + 1) % ids.length]);
    } else if (key === 'ArrowLeft' || key === 'ArrowUp') {
      event.preventDefault();
      select(ids[(i - 1 + ids.length) % ids.length]);
    } else if (key === 'Escape' && selectedId) {
      event.preventDefault();
      event.stopPropagation();
      select(null);
    } else if (mode === '3d' && (key === '+' || key === '=')) {
      event.preventDefault();
      zoom(1);
    } else if (mode === '3d' && (key === '-' || key === '_')) {
      event.preventDefault();
      zoom(-1);
    } else if (mode === '3d' && key === '0') {
      event.preventDefault();
      resetTick++;
    }
  }

  function seekTo(t: number) {
    void studio.seek(t, { play: studio.hasAudio });
  }
</script>

<div class="explorer">
  <div class="toolbar">
    <Segmented
      label="Map view"
      value={mode}
      options={[
        { value: '3d', label: '3D', icon: Box },
        { value: '2d', label: '2D', icon: LayoutGrid },
      ]}
      onchange={(v) => {
        if (v === '3d') webglFailed = false;
        setMode(v);
      }}
    />

    <div class="group" role="group" aria-label="Tour">
      <Button
        size="sm"
        icon
        aria-label="Previous concept"
        title="Previous concept"
        onclick={() => tourStep(-1)}
        disabled={!order.length}
      >
        <SkipBack size={15} />
      </Button>
      <Button
        size="sm"
        variant={tourOn ? 'soft' : 'outline'}
        onclick={() => (tourOn ? stopTour() : startTour())}
        disabled={!order.length}
      >
        {#if tourOn}<Pause size={14} /> Pause tour{:else}<Play size={14} /> Tour{/if}
      </Button>
      <Button
        size="sm"
        icon
        aria-label="Next concept"
        title="Next concept"
        onclick={() => tourStep(1)}
        disabled={!order.length}
      >
        <SkipForward size={15} />
      </Button>
    </div>

    {#if mode === '3d'}
      <div class="group" role="group" aria-label="Camera">
        <Button
          size="sm"
          variant="outline"
          pressed={follow}
          title="Let the camera drift to the concepts being discussed"
          onclick={() => (follow = !follow)}
          disabled={reduceMotion}
        >
          <Radio size={14} /> Follow lecture
        </Button>
        <Button
          size="sm"
          icon
          aria-label="Zoom in"
          title="Zoom in (+)"
          onclick={() => zoom(1)}
        >
          <Plus size={15} />
        </Button>
        <Button
          size="sm"
          icon
          aria-label="Zoom out"
          title="Zoom out (−)"
          onclick={() => zoom(-1)}
        >
          <Minus size={15} />
        </Button>
        <Button
          size="sm"
          icon
          aria-label="Reset view"
          title="Reset view (0)"
          onclick={() => resetTick++}
        >
          <Maximize size={15} />
        </Button>
      </div>
    {/if}
  </div>

  <div class="body">
    <!-- A custom widget with its own arrow-key navigation, so role="application"
         is intended; the concept list beside it is the screen-reader path. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <div
      class="map"
      role="application"
      aria-roledescription="concept map"
      tabindex="0"
      aria-label="Concept map of {visualization.title}. {nodes.length} concepts. Use the arrow keys to move between concepts, Escape to clear."
      onkeydown={onMapKeydown}
      onpointerdown={() => (touched = true)}
    >
      {#if mode === '3d'}
        {#key visualization.id}
          <ConceptMap3D
            {visualization}
            {selectedId}
            {activeIds}
            follow={follow && !tourOn}
            {reduceMotion}
            {resetTick}
            {zoomTick}
            {zoomDir}
            onSelect={(id) => select(id)}
            onUnsupported={() => (webglFailed = true)}
          />
        {/key}
        {#if !touched}
          <p class="hint-overlay" aria-hidden="true">
            Drag to orbit · scroll to zoom · click a concept
          </p>
        {/if}
      {:else}
        <div class="flat">
          <ConceptMap
            {visualization}
            {selectedId}
            {activeIds}
            onSelect={(id) => select(id)}
          />
        </div>
      {/if}
      {#if webglFailed}
        <p class="note" role="status">
          3D isn’t available on this device, so the 2D map is shown.
        </p>
      {/if}
    </div>

    <aside class="side" aria-label="Concept details">
      {#if selected}
        <section class="detail" aria-live="polite">
          <header>
            <span class="dot" style:background={colorFor(selected.group)}
            ></span>
            <div class="titles">
              <h3>{selected.label}</h3>
              <p class="sub">
                {selected.group} ·
                <span
                  class="meter"
                  role="img"
                  aria-label="Importance {selected.importance} of 5"
                >
                  {#each [1, 2, 3, 4, 5] as n}<i
                      class:on={n <= selected.importance}
                    ></i>{/each}
                </span>
              </p>
            </div>
            <Button
              icon
              size="sm"
              aria-label="Clear selection"
              onclick={() => select(null)}
            >
              <X size={15} />
            </Button>
          </header>

          <h4>In the lecture</h4>
          {#if moments.length}
            <p class="count">
              Mentioned {mentionCount} time{mentionCount === 1 ? '' : 's'}
            </p>
            <ul class="moments">
              {#each moments.slice(0, 6) as m}
                <li>
                  <button
                    type="button"
                    onclick={() => seekTo(m.t)}
                    title={m.text}
                  >
                    <span class="when">{clock(m.t)}</span>
                    <span class="snip">{m.text}</span>
                  </button>
                </li>
              {/each}
            </ul>
          {:else}
            <p class="muted">Not mentioned by this name in the transcript.</p>
          {/if}

          {#if connections.length}
            <h4>Connections</h4>
            <ul class="links">
              {#each connections as c}
                <li>
                  <button type="button" onclick={() => select(c.id)}>
                    <span
                      class="dot sm"
                      style:background={colorFor(byId.get(c.id)?.group ?? '')}
                    ></span>
                    <span class="rel">
                      {#if c.relation}{c.outgoing
                          ? c.relation
                          : `${c.relation} (from)`}{:else}{c.outgoing
                          ? 'links to'
                          : 'linked from'}{/if}
                    </span>
                    <strong>{c.label}</strong>
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </section>
      {:else}
        <p class="empty">
          Pick a concept to see where it comes up in the lecture and how it
          connects.
        </p>
      {/if}

      <div class="list-wrap">
        <h3 class="section">
          All concepts <span class="n">{nodes.length}</span>
        </h3>
        <ul class="list">
          {#each sortedNodes as n (n.id)}
            <li>
              <button
                type="button"
                class="item"
                class:on={selectedId === n.id}
                aria-pressed={selectedId === n.id}
                onclick={() => select(selectedId === n.id ? null : n.id)}
              >
                <span class="dot sm" style:background={colorFor(n.group)}
                ></span>
                <span class="name">{n.label}</span>
                {#if activeIds.has(n.id)}
                  <span class="now" title="Being discussed now">now</span>
                {/if}
              </button>
            </li>
          {/each}
        </ul>
      </div>
    </aside>
  </div>

  {#if groups.length > 1}
    <ul class="legend" aria-label="Legend">
      {#each groups as g}
        <li><span class="dot sm" style:background={colorFor(g)}></span>{g}</li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .explorer {
    container-type: inline-size;
    container-name: explorer;
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 0;
  }
  .toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .group {
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }
  .body {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 12px;
    min-height: 0;
  }
  @container explorer (min-width: 760px) {
    .body {
      grid-template-columns: minmax(0, 1fr) 280px;
    }
  }
  .map {
    position: relative;
    min-height: 300px;
    height: clamp(320px, 64cqw, 640px);
    border-radius: var(--radius-lg);
    outline: none;
  }
  .map:focus-visible {
    box-shadow: var(--focus-ring);
  }
  .flat {
    height: 100%;
    overflow: auto;
    border-radius: var(--radius-lg);
  }
  .hint-overlay {
    position: absolute;
    left: 12px;
    bottom: 10px;
    margin: 0;
    padding: 3px 10px;
    border-radius: 999px;
    background: rgba(16, 9, 4, 0.7);
    font-size: 11.5px;
    color: var(--text-secondary);
    pointer-events: none;
  }
  .note {
    position: absolute;
    top: 10px;
    left: 10px;
    right: 10px;
    margin: 0;
    padding: 6px 12px;
    border-radius: var(--radius-md);
    background: var(--surface-3);
    border: 1px solid var(--border-default);
    font-size: 12.5px;
    color: var(--text-secondary);
  }
  .side {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-width: 0;
  }
  .detail,
  .empty {
    padding: 14px;
    background: var(--surface-inset);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
  }
  .empty {
    margin: 0;
    font-size: 13.5px;
    line-height: 1.55;
    color: var(--text-secondary);
  }
  .detail header {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    margin-bottom: 10px;
  }
  .titles {
    flex: 1;
    min-width: 0;
  }
  .detail h3 {
    margin: 0;
    font-size: 20px;
    line-height: 1.2;
    overflow-wrap: anywhere;
  }
  .sub {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 3px 0 0;
    font-size: 12.5px;
    color: var(--text-secondary);
    text-transform: capitalize;
  }
  .meter {
    display: inline-flex;
    gap: 3px;
  }
  .meter i {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--border-default);
  }
  .meter i.on {
    background: var(--color-ember-accent);
  }
  .dot {
    flex: none;
    width: 12px;
    height: 12px;
    margin-top: 6px;
    border-radius: 50%;
  }
  .dot.sm {
    width: 9px;
    height: 9px;
    margin: 0;
  }
  h4,
  .section {
    margin: 12px 0 6px;
    font-family: var(--font-body);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--text-tertiary);
  }
  .count,
  .muted {
    margin: 0 0 6px;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .moments,
  .links,
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .moments button,
  .links button {
    display: flex;
    align-items: baseline;
    gap: 8px;
    width: 100%;
    min-height: 34px;
    padding: 6px 8px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-secondary);
    font-family: var(--font-body);
    font-size: 13px;
    letter-spacing: 0;
    text-transform: none;
    text-align: left;
    white-space: normal;
  }
  .moments button:hover,
  .links button:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }
  .moments button:focus-visible,
  .links button:focus-visible,
  .item:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }
  .when {
    flex: none;
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    color: var(--text-accent);
  }
  .snip {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    line-height: 1.4;
  }
  .links button {
    align-items: center;
  }
  .rel {
    color: var(--text-tertiary);
  }
  .links strong {
    color: var(--text-primary);
    font-weight: 500;
  }
  .list-wrap {
    min-height: 0;
  }
  .list-wrap .section {
    margin-top: 0;
  }
  .n {
    margin-left: 4px;
    font-family: var(--font-mono, ui-monospace, monospace);
  }
  .list {
    max-height: 220px;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: var(--color-cork-border) transparent;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    min-height: 34px;
    padding: 6px 10px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-secondary);
    font-family: var(--font-body);
    font-size: 13.5px;
    letter-spacing: 0;
    text-transform: none;
    text-align: left;
  }
  .item:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }
  .item.on {
    background: var(--surface-accent);
    color: var(--text-primary);
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .now {
    padding: 1px 7px;
    border: 1px solid var(--border-accent);
    border-radius: 999px;
    font-size: 10.5px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-accent);
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 16px;
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: 12px;
    color: var(--text-secondary);
  }
  .legend li {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
</style>
