<script lang="ts">
  import {
    Play,
    Pause,
    RotateCcw,
    RotateCw,
    BookmarkPlus,
    Bookmark,
  } from '@lucide/svelte';
  import { triggerHaptic } from '$lib/native/haptics';
  import { useStudio, SPEEDS } from '$lib/studio/lectureStudio.svelte';
  import { clock, spokenClock } from '$lib/studio/time';

  const studio = useStudio();

  const total = $derived(Math.max(studio.duration, 1));
  const pct = $derived(Math.min(100, (studio.time / total) * 100));
  const chapterTitle = $derived(studio.activeChapter?.title ?? '');
  const bookmarked = $derived(studio.hasBookmarkAt(Math.round(studio.time)));
  const canPlay = $derived(studio.hasAudio && !studio.audioLoading);

  // Hover/focus preview over the scrubber.
  let trackEl: HTMLDivElement | undefined = $state();
  let hover = $state<{ x: number; t: number } | null>(null);

  const chapterAt = (t: number) => {
    let found: string | null = null;
    for (const c of studio.chapters)
      if (c.start_timestamp_sec <= t) found = c.title;
    return found;
  };

  function onPointerMove(event: PointerEvent) {
    if (!trackEl) return;
    const rect = trackEl.getBoundingClientRect();
    const x = Math.min(Math.max(event.clientX - rect.left, 0), rect.width);
    hover = { x, t: (x / rect.width) * total };
  }

  function onInput(event: Event) {
    const value = Number((event.currentTarget as HTMLInputElement).value);
    void studio.seek(value);
  }

  function press(fn: () => void) {
    triggerHaptic('light');
    fn();
  }

  const valueText = $derived(
    `${spokenClock(studio.time)} of ${spokenClock(studio.duration)}` +
      (chapterTitle ? `, chapter: ${chapterTitle}` : '')
  );
</script>

<section class="player" aria-label="Lecture player">
  <div class="transport">
    <button
      type="button"
      class="icon-btn"
      aria-label="Back 15 seconds"
      title="Back 15 seconds (J)"
      onclick={() => press(() => studio.skip(-15))}
    >
      <RotateCcw size={18} /><span class="skip-n" aria-hidden="true">15</span>
    </button>
    <button
      type="button"
      class="play"
      class:loading={studio.audioLoading}
      aria-label={studio.playing ? 'Pause' : 'Play'}
      title={studio.hasAudio
        ? `${studio.playing ? 'Pause' : 'Play'} (K)`
        : 'No recording for this lecture'}
      disabled={!studio.hasAudio}
      aria-busy={studio.audioLoading}
      onclick={() => press(() => studio.toggle())}
    >
      {#if studio.audioLoading}
        <span class="spinner" aria-hidden="true"></span>
      {:else if studio.playing}
        <Pause size={20} fill="currentColor" />
      {:else}
        <Play size={20} fill="currentColor" />
      {/if}
    </button>
    <button
      type="button"
      class="icon-btn"
      aria-label="Forward 15 seconds"
      title="Forward 15 seconds (L)"
      onclick={() => press(() => studio.skip(15))}
    >
      <RotateCw size={18} /><span class="skip-n" aria-hidden="true">15</span>
    </button>
  </div>

  <div class="scrub">
    <span class="time now" aria-hidden="true">{clock(studio.time)}</span>
    <div
      class="track-wrap"
      bind:this={trackEl}
      onpointermove={onPointerMove}
      onpointerleave={() => (hover = null)}
      role="presentation"
    >
      <div class="track" aria-hidden="true">
        <div class="fill" style:width="{pct}%"></div>
        {#each studio.chapters as c, i (c.id ?? i)}
          {#if c.start_timestamp_sec > 0}
            <span
              class="tick"
              style:left="{(c.start_timestamp_sec / total) * 100}%"
            ></span>
          {/if}
        {/each}
        {#each studio.bookmarks as b (b.t)}
          <span class="mark" style:left="{(b.t / total) * 100}%"></span>
        {/each}
      </div>
      <div class="thumb" aria-hidden="true" style:left="{pct}%"></div>
      <input
        type="range"
        min="0"
        max={total}
        step="1"
        value={Math.min(studio.time, total)}
        oninput={onInput}
        aria-label="Lecture position"
        aria-valuetext={valueText}
        title="Seek"
      />
      {#if hover}
        <div class="tip" style:left="{hover.x}px" aria-hidden="true">
          <strong>{clock(hover.t)}</strong>
          {#if chapterAt(hover.t)}<span>{chapterAt(hover.t)}</span>{/if}
        </div>
      {/if}
    </div>
    <span class="time total" aria-hidden="true">{clock(studio.duration)}</span>
  </div>

  <div class="meta">
    {#if chapterTitle}
      <span class="chapter-now" title={chapterTitle}>{chapterTitle}</span>
    {/if}
    <div class="right">
      <button
        type="button"
        class="icon-btn bookmark"
        class:on={bookmarked}
        aria-pressed={bookmarked}
        aria-label={bookmarked ? 'Remove saved moment' : 'Save this moment'}
        title="Save this moment (B)"
        onclick={() => press(() => studio.toggleBookmark())}
      >
        {#if bookmarked}<Bookmark
            size={17}
            fill="currentColor"
          />{:else}<BookmarkPlus size={17} />{/if}
      </button>
      <button
        type="button"
        class="speed"
        aria-label="Playback speed {studio.rate}×. Press to change."
        title="Playback speed ( &lt; &gt; )"
        onclick={() => press(() => studio.cycleRate(1))}
      >
        {studio.rate}×
      </button>
    </div>
  </div>

  {#if studio.audioError}
    <p class="note err" role="alert">{studio.audioError}</p>
  {:else if studio.recordingsStatus === 'ready' && !studio.hasAudio}
    <p class="note">
      No audio was recorded. The transcript and chapters still follow the
      timeline.
    </p>
  {/if}
</section>

<style>
  .player {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) minmax(0, 260px);
    grid-template-areas: 'transport scrub meta' 'note note note';
    align-items: center;
    column-gap: 18px;
    padding: 10px 18px;
    background: var(--surface-2);
    border-top: 1px solid var(--border-default);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
  }
  .transport {
    grid-area: transport;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .scrub {
    grid-area: scrub;
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }
  .meta {
    grid-area: meta;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 10px;
    min-width: 0;
  }
  .right {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }
  .note {
    grid-area: note;
    margin: 6px 0 0;
    font-size: 12.5px;
    color: var(--text-tertiary);
  }
  .note.err {
    color: var(--color-error);
  }

  .icon-btn {
    position: relative;
    display: inline-grid;
    place-items: center;
    width: var(--control-h);
    min-width: var(--control-h);
    height: var(--control-h);
    min-height: var(--control-h);
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--text-secondary);
    transition:
      background var(--motion-fast) var(--ease-out),
      color var(--motion-fast) var(--ease-out);
  }
  .icon-btn:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }
  .skip-n {
    position: absolute;
    font-size: 7.5px;
    font-weight: 700;
    letter-spacing: 0;
    margin-top: 1px;
  }
  .bookmark.on {
    color: var(--text-accent);
  }
  .play {
    display: grid;
    place-items: center;
    width: 44px;
    min-width: 44px;
    height: 44px;
    min-height: 44px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: var(--text-primary);
    color: var(--color-walnut-shadow);
    box-shadow: var(--elev-2);
    transition:
      transform var(--motion-fast) var(--ease-out),
      background var(--motion-fast) var(--ease-out);
  }
  .play:hover:not(:disabled) {
    transform: scale(1.06);
  }
  .play:active:not(:disabled) {
    transform: scale(0.96);
  }
  .play:disabled {
    background: var(--surface-3);
    color: var(--text-tertiary);
    box-shadow: none;
    opacity: 1;
  }
  .icon-btn:focus-visible,
  .play:focus-visible,
  .speed:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }
  .track-wrap:has(input:focus-visible) .thumb {
    box-shadow: var(--focus-ring);
    transform: scale(1.25);
  }
  .spinner {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 2px solid rgba(16, 9, 4, 0.25);
    border-top-color: var(--color-walnut-shadow);
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .time {
    flex-shrink: 0;
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 12.5px;
    font-variant-numeric: tabular-nums;
    color: var(--text-secondary);
    min-width: 40px;
  }
  .time.now {
    color: var(--text-primary);
    text-align: right;
  }
  .track-wrap {
    position: relative;
    flex: 1;
    height: 28px;
    min-width: 0;
    display: flex;
    align-items: center;
  }
  .track {
    position: relative;
    width: 100%;
    height: 6px;
    border-radius: 999px;
    background: rgba(255, 237, 215, 0.14);
    transition: height var(--motion-fast) var(--ease-out);
  }
  .track-wrap:hover .track {
    height: 8px;
  }
  .fill {
    position: absolute;
    inset: 0 auto 0 0;
    border-radius: inherit;
    background: linear-gradient(90deg, #b84000, #ff7a2e);
  }
  .tick {
    position: absolute;
    top: -1px;
    bottom: -1px;
    width: 2px;
    margin-left: -1px;
    background: var(--surface-2);
  }
  .mark {
    position: absolute;
    top: 50%;
    width: 8px;
    height: 8px;
    margin: -4px 0 0 -4px;
    transform: rotate(45deg);
    background: var(--color-warm-cream);
    border: 1.5px solid var(--surface-2);
    border-radius: 2px;
  }
  .thumb {
    position: absolute;
    top: 50%;
    width: 14px;
    height: 14px;
    margin: -7px 0 0 -7px;
    border-radius: 50%;
    background: var(--text-primary);
    box-shadow: 0 1px 6px rgba(0, 0, 0, 0.6);
    pointer-events: none;
    transform: scale(0.85);
    transition: transform var(--motion-fast) var(--ease-out);
  }
  .track-wrap:hover .thumb,
  .track-wrap:focus-within .thumb {
    transform: scale(1.15);
  }
  input[type='range'] {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    margin: 0;
    opacity: 0;
    cursor: pointer;
    min-height: 0;
    touch-action: none;
  }
  .tip {
    position: absolute;
    bottom: 100%;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1px;
    max-width: 220px;
    padding: 5px 9px;
    margin-bottom: 4px;
    border-radius: var(--radius-sm);
    background: var(--surface-3);
    border: 1px solid var(--border-default);
    box-shadow: var(--elev-2);
    font-size: 11.5px;
    color: var(--text-secondary);
    pointer-events: none;
    white-space: nowrap;
    z-index: 5;
  }
  .tip strong {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 12px;
    color: var(--text-primary);
  }
  .tip span {
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 200px;
  }

  .chapter-now {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .speed {
    min-width: 52px;
    height: var(--control-h);
    min-height: var(--control-h);
    padding: 0 10px;
    border: 1px solid var(--border-default);
    border-radius: 999px;
    background: transparent;
    color: var(--text-primary);
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 12.5px;
    font-variant-numeric: tabular-nums;
    transition:
      border-color var(--motion-fast) var(--ease-out),
      background var(--motion-fast) var(--ease-out);
  }
  .speed:hover {
    border-color: var(--border-strong);
    background: var(--surface-hover);
  }

  /* Medium: drop the chapter title, keep controls. */
  @media (max-width: 1100px) {
    .chapter-now {
      display: none;
    }
    .player {
      grid-template-columns: auto minmax(0, 1fr) auto;
    }
  }
  /* Narrow / phone: two rows — scrubber on top, controls below. */
  @media (max-width: 720px) {
    .player {
      grid-template-columns: 1fr auto;
      grid-template-areas: 'scrub scrub' 'transport meta' 'note note';
      row-gap: 2px;
      padding: 8px 12px 10px;
    }
    .chapter-now {
      display: block;
      max-width: 42vw;
    }
    .meta {
      justify-content: flex-end;
    }
    .transport {
      gap: 2px;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spinner {
      animation-duration: 1.6s !important;
      animation-iteration-count: infinite !important;
    }
  }
</style>
