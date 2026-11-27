<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import type { ClaimRecord } from '$lib/api';
  import {
    PanelLeftClose,
    PanelLeftOpen,
    PanelRightClose,
    PanelRightOpen,
  } from '@lucide/svelte';
  import StudioHeader from '$lib/components/archive/StudioHeader.svelte';
  import {
    LectureStudio,
    provideStudio,
  } from '$lib/studio/lectureStudio.svelte';
  import { clock } from '$lib/studio/time';
  import Button from '$lib/components/ui/Button.svelte';
  import ToolTabs from './ToolTabs.svelte';
  import PlayerBar from './PlayerBar.svelte';
  import ChapterRail from './ChapterRail.svelte';
  import SyncedTranscript from './SyncedTranscript.svelte';
  import NotesPane from './NotesPane.svelte';
  import AskPane from './AskPane.svelte';
  import CardsPane from './CardsPane.svelte';
  import QuizPane from './QuizPane.svelte';
  import SummaryPane from './SummaryPane.svelte';
  import HandoutPane from './HandoutPane.svelte';
  import MapPane from './MapPane.svelte';
  import Splitter from './Splitter.svelte';
  import ShortcutsDialog from './ShortcutsDialog.svelte';
  import type { SelectionAction } from './SelectionToolbar.svelte';
  import { TOOLS, tabId, type ToolId } from './tools';

  let {
    claim,
    onBack,
    focus = $bindable(false),
    listOpen = $bindable(false),
    showListToggle = false,
    startAt = null,
  }: {
    claim: ClaimRecord;
    /** Open at this moment (seconds), e.g. from a search hit. */
    startAt?: number | null;
    onBack?: () => void;
    focus?: boolean;
    listOpen?: boolean;
    showListToggle?: boolean;
  } = $props();

  // The archive record's id is the lecture session UUID, which every study
  // endpoint is keyed by. (Guest/offline records fall back to the join code.)
  // The parent keys this component by lecture, so reading `claim` once is right.
  const { sessionKey, studio } = untrack(() => {
    const key =
      claim.attended === undefined && claim.session_code
        ? claim.session_code
        : claim.id || claim.session_code || '';
    return {
      sessionKey: key,
      studio: new LectureStudio({
        sessionKey: key,
        lectureId: claim.id,
        title: claim.session_title,
        startAt,
      }),
    };
  });
  provideStudio(studio);
  onMount(() => studio.init());

  // ── Layout state ────────────────────────────────────────────
  const LAYOUT_KEY = 'klasync-studio-layout';
  const SIDE_MIN = 320;
  const SIDE_MAX = 720;

  let width = $state(1200);
  const mode = $derived<'wide' | 'medium' | 'narrow'>(
    width >= 1060 ? 'wide' : width >= 900 ? 'medium' : 'narrow'
  );

  let stageTool = $state<ToolId>('transcript');
  let sideTool = $state<ToolId>('notes');
  let sideWidth = $state(440);
  let sideOpen = $state(true);
  let railOpen = $state(true);
  let showHelp = $state(false);

  onMount(() => {
    try {
      const saved = JSON.parse(localStorage.getItem(LAYOUT_KEY) ?? 'null');
      if (saved) {
        if (TOOLS.some((t) => t.id === saved.stage)) stageTool = saved.stage;
        if (TOOLS.some((t) => t.id === saved.side)) sideTool = saved.side;
        if (Number.isFinite(saved.sideWidth))
          sideWidth = Math.min(Math.max(saved.sideWidth, SIDE_MIN), SIDE_MAX);
        if (typeof saved.sideOpen === 'boolean') sideOpen = saved.sideOpen;
        if (typeof saved.railOpen === 'boolean') railOpen = saved.railOpen;
      }
    } catch {
      /* corrupt layout: keep defaults */
    }
  });

  $effect(() => {
    const snapshot = JSON.stringify({
      stage: stageTool,
      side: sideTool,
      sideWidth: Math.round(sideWidth),
      sideOpen,
      railOpen,
    });
    try {
      localStorage.setItem(LAYOUT_KEY, snapshot);
    } catch {
      /* storage unavailable */
    }
  });

  const showRail = $derived(mode === 'wide' && railOpen && !focus);
  const showSide = $derived(mode !== 'narrow' && sideOpen && !focus);
  /** Chapters live in the rail when it is visible, otherwise they are a tool. */
  const toolIds = $derived<ToolId[]>(
    mode === 'wide' && !focus
      ? TOOLS.filter((t) => t.id !== 'chapters').map((t) => t.id)
      : TOOLS.map((t) => t.id)
  );
  const effStage = $derived<ToolId>(
    toolIds.includes(stageTool) ? stageTool : 'transcript'
  );
  const effSide = $derived.by<ToolId>(() => {
    let t: ToolId = toolIds.includes(sideTool) ? sideTool : 'notes';
    if (t === effStage) t = effStage === 'notes' ? 'ask' : 'notes';
    return t;
  });

  // Keep the side pane inside the workspace.
  const sideMax = $derived(
    Math.max(SIDE_MIN, Math.min(SIDE_MAX, width - (showRail ? 256 : 0) - 380))
  );
  const sideClamped = $derived(
    Math.min(Math.max(sideWidth, SIDE_MIN), sideMax)
  );

  function selectTool(pane: 'stage' | 'side', id: ToolId) {
    if (pane === 'stage') {
      if (effSide === id) sideTool = effStage;
      stageTool = id;
    } else {
      if (effStage === id) stageTool = effSide;
      sideTool = id;
    }
  }

  /** Brings a tool on screen without disturbing the pane already showing it. */
  function openTool(id: ToolId) {
    if (id === effStage || (showSide && id === effSide)) return;
    if (mode === 'narrow' || id === 'transcript') stageTool = id;
    else {
      if (!sideOpen || focus) {
        sideOpen = true;
        focus = false;
      }
      sideTool = id;
    }
  }

  // ── Selection actions from the transcript ───────────────────
  function onTranscriptAction(
    action: SelectionAction,
    text: string,
    seconds: number | null
  ) {
    switch (action) {
      case 'note':
        studio.addNote(text, seconds);
        break;
      case 'ask':
        studio.askDraft = text.length > 220 ? text.slice(0, 217) + '…' : text;
        openTool('ask');
        break;
      case 'cards':
        studio.flashcardSeed =
          text.length > 90 ? text.slice(0, 87) + '…' : text;
        openTool('flashcards');
        break;
      case 'bookmark': {
        const t = Math.round(seconds ?? studio.time);
        if (!studio.hasBookmarkAt(t)) studio.toggleBookmark(t, text);
        else studio.announce(`Already saved at ${clock(t)}`);
        break;
      }
    }
  }

  // ── Keyboard ────────────────────────────────────────────────
  function onKeydown(event: KeyboardEvent) {
    if (
      event.defaultPrevented ||
      event.ctrlKey ||
      event.metaKey ||
      event.altKey
    )
      return;
    const target = event.target as HTMLElement | null;
    if (target) {
      const tag = target.tagName;
      const type = (target as HTMLInputElement).type;
      // Typing fields keep every key; a range slider (the scrubber) only
      // keeps the keys it handles natively.
      const typing =
        tag === 'TEXTAREA' ||
        tag === 'SELECT' ||
        target.isContentEditable ||
        (tag === 'INPUT' && type !== 'range' && type !== 'checkbox');
      if (typing) return;
    }
    if (showHelp) return;
    const role = target?.getAttribute?.('role');
    // Controls that own Space / arrows keep them.
    const ownsActivation =
      target?.tagName === 'BUTTON' ||
      target?.tagName === 'INPUT' ||
      role === 'tab' ||
      role === 'separator' ||
      role === 'menuitemradio';
    const key = event.key;

    const handled = (fn: () => void) => {
      event.preventDefault();
      fn();
    };

    if (key === ' ' && !ownsActivation) return handled(() => studio.toggle());
    switch (key.toLowerCase()) {
      case 'k':
        return handled(() => studio.toggle());
      case 'j':
        return handled(() => studio.skip(-10));
      case 'l':
        return handled(() => studio.skip(10));
      case 'b':
        return handled(() => studio.toggleBookmark());
      case 'n':
        return handled(() => {
          openTool('notes');
          studio.pendingStamp = true;
        });
      case 'f':
        if (mode !== 'narrow') return handled(() => (focus = !focus));
        break;
      case '/':
        return handled(() => {
          openTool('ask');
          studio.focusAsk = true;
        });
      case '?':
        return handled(() => (showHelp = true));
      case '[':
        return handled(() => studio.seekChapter(-1));
      case ']':
        return handled(() => studio.seekChapter(1));
      case ',':
      case '<':
        return handled(() => studio.cycleRate(-1));
      case '.':
      case '>':
        return handled(() => studio.cycleRate(1));
      case 'escape':
        if (focus) return handled(() => (focus = false));
        break;
    }
    if (key === 'ArrowLeft' && !ownsActivation)
      return handled(() => studio.skip(-5));
    if (key === 'ArrowRight' && !ownsActivation)
      return handled(() => studio.skip(5));
    const tool = TOOLS.find((t) => t.key === key);
    if (tool) handled(() => openTool(tool.id));
  }

  // Announce nothing visually when there is nothing to say.
  const toast = $derived(studio.announcement);

  const narrowPrimary: ToolId[] = ['transcript', 'chapters', 'notes', 'ask'];
</script>

<svelte:window onkeydown={onKeydown} />

<div
  class="lecture-workspace"
  class:focus
  class:narrow={mode === 'narrow'}
  bind:clientWidth={width}
>
  <StudioHeader
    {claim}
    onBack={focus && mode !== 'narrow' ? undefined : onBack}
    duration={studio.duration}
    {focus}
    {listOpen}
    showListToggle={showListToggle && !focus}
    onToggleFocus={() => (focus = !focus)}
    onToggleList={() => (listOpen = !listOpen)}
    onShortcuts={() => (showHelp = true)}
  />

  <div class="shell">
    <div
      class="body"
      class:with-rail={showRail}
      class:with-side={showSide}
      style:--side-w="{sideClamped}px"
    >
      {#if showRail}
        <aside class="rail" aria-label="Chapters">
          <ChapterRail />
        </aside>
      {/if}

      <section class="pane stage">
        <div class="pane-head">
          {#if mode === 'wide' && !focus}
            <Button
              icon
              onclick={() => (railOpen = !railOpen)}
              aria-pressed={railOpen}
              aria-label={railOpen
                ? 'Hide chapter outline'
                : 'Show chapter outline'}
              title={railOpen ? 'Hide chapter outline' : 'Show chapter outline'}
            >
              {#if railOpen}<PanelLeftClose size={17} />{:else}<PanelLeftOpen
                  size={17}
                />{/if}
            </Button>
          {/if}
          <ToolTabs
            pane="stage"
            label="Lecture tools"
            active={effStage}
            tools={toolIds}
            primary={mode === 'narrow' ? narrowPrimary : undefined}
            labels={mode === 'narrow'}
            onSelect={(id) => selectTool('stage', id)}
          />
          {#if mode !== 'narrow' && !focus && !sideOpen}
            <Button
              icon
              onclick={() => (sideOpen = true)}
              aria-label="Show companion panel"
              title="Show companion panel"
            >
              <PanelRightOpen size={17} />
            </Button>
          {/if}
        </div>
        <div
          class="pane-body"
          id="stage-panel"
          role="tabpanel"
          aria-labelledby={tabId('stage', effStage)}
          tabindex="-1"
        >
          {@render tool(effStage)}
        </div>
      </section>

      {#if showSide}
        <Splitter
          bind:value={sideWidth}
          min={SIDE_MIN}
          max={sideMax}
          label="Resize companion panel"
        />
        <section class="pane side">
          <div class="pane-head">
            <ToolTabs
              pane="side"
              label="Companion tools"
              active={effSide}
              tools={toolIds}
              onSelect={(id) => selectTool('side', id)}
            />
            <Button
              icon
              onclick={() => (sideOpen = false)}
              aria-label="Hide companion panel"
              title="Hide companion panel"
            >
              <PanelRightClose size={17} />
            </Button>
          </div>
          <div
            class="pane-body"
            id="side-panel"
            role="tabpanel"
            aria-labelledby={tabId('side', effSide)}
            tabindex="-1"
          >
            {@render tool(effSide)}
          </div>
        </section>
      {/if}
    </div>

    <PlayerBar />

    {#if toast}
      <div class="toast" aria-hidden="true">{toast}</div>
    {/if}
  </div>

  <span class="sr-only" role="status" aria-live="polite">{toast}</span>
</div>

{#if showHelp}
  <ShortcutsDialog onClose={() => (showHelp = false)} />
{/if}

{#snippet tool(id: ToolId)}
  {#key id}
    <div class="tool-fill" class:scroll={id === 'chapters'}>
      {#if id === 'transcript'}
        <SyncedTranscript onAction={onTranscriptAction} />
      {:else if id === 'notes'}
        <NotesPane />
      {:else if id === 'ask'}
        <AskPane />
      {:else if id === 'summary'}
        <SummaryPane />
      {:else if id === 'visuals'}
        <MapPane />
      {:else if id === 'flashcards'}
        <CardsPane />
      {:else if id === 'quiz'}
        <QuizPane />
      {:else if id === 'handout'}
        <HandoutPane />
      {:else if id === 'chapters'}
        <ChapterRail variant="page" />
      {/if}
    </div>
  {/key}
{/snippet}

<style>
  .lecture-workspace {
    --studio-offset: 100px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    width: 100%;
    min-width: 0;
    height: max(540px, calc(100dvh - var(--studio-offset)));
    min-height: 0;
  }
  .lecture-workspace.focus {
    --studio-offset: 90px;
    gap: 10px;
  }
  .shell {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--surface-1);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-xl);
    box-shadow: var(--elev-2);
    overflow: hidden;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
  }
  .body.with-rail {
    grid-template-columns: 264px minmax(0, 1fr);
  }
  .body.with-side {
    grid-template-columns: minmax(0, 1fr) 14px var(--side-w);
  }
  .body.with-rail.with-side {
    grid-template-columns: 264px minmax(0, 1fr) 14px var(--side-w);
  }
  .rail {
    min-height: 0;
    padding: 14px 10px 10px 12px;
    border-right: 1px solid var(--border-subtle);
    background: rgba(0, 0, 0, 0.14);
  }
  .pane {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    padding: 10px 16px 14px;
  }
  .pane-head {
    flex: none;
    display: flex;
    align-items: center;
    gap: 6px;
    padding-bottom: 10px;
    margin-bottom: 10px;
    border-bottom: 1px solid var(--border-subtle);
  }
  .pane-body {
    flex: 1;
    min-height: 0;
    outline: none;
  }
  .tool-fill {
    height: 100%;
    min-height: 0;
    animation: enter var(--motion-base) var(--ease-out);
  }
  .tool-fill.scroll {
    overflow-y: auto;
    overscroll-behavior: contain;
    padding-right: 4px;
    scrollbar-width: thin;
    scrollbar-color: var(--color-cork-border) transparent;
  }
  @keyframes enter {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
  }
  .side {
    border-left: 0;
    background: transparent;
  }
  .toast {
    position: absolute;
    bottom: 84px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 20;
    padding: 8px 16px;
    border-radius: 999px;
    background: var(--surface-3);
    border: 1px solid var(--border-strong);
    box-shadow: var(--elev-3);
    color: var(--text-primary);
    font-size: 13px;
    white-space: nowrap;
    animation: drop var(--motion-base) var(--ease-out);
    pointer-events: none;
  }
  @keyframes drop {
    from {
      opacity: 0;
      transform: translate(-50%, 8px);
    }
  }

  /* Phone / narrow: one stage, player docked under it. */
  .lecture-workspace.narrow {
    --studio-offset: 100px;
    gap: 10px;
    height: max(440px, calc(100dvh - var(--studio-offset)));
  }
  .narrow .shell {
    border-radius: var(--radius-lg);
  }
  .narrow .pane {
    padding: 8px 10px 10px;
  }
  .narrow .pane-head {
    padding-bottom: 8px;
    margin-bottom: 8px;
  }
  /* The global bottom navigation appears at 800px and below. */
  @media (max-width: 800px) {
    .lecture-workspace.narrow {
      --studio-offset: calc(
        60px + 12px + 74px + env(safe-area-inset-bottom, 0px)
      );
    }
  }
</style>
