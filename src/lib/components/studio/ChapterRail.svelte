<script lang="ts">
  import { ListTree, Sparkles, Bookmark } from '@lucide/svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import SkeletonCard from '$lib/components/shared/SkeletonCard.svelte';
  import { useStudio } from '$lib/studio/lectureStudio.svelte';
  import { clock } from '$lib/studio/time';

  let { variant = 'rail' }: { variant?: 'rail' | 'page' } = $props();

  const studio = useStudio();
  let activeEl: HTMLElement | undefined = $state();
  let listEl: HTMLElement | undefined = $state();

  // Keep the current chapter in view as the lecture plays.
  $effect(() => {
    void studio.activeChapterIndex;
    if (!activeEl || !listEl) return;
    const box = listEl.getBoundingClientRect();
    const el = activeEl.getBoundingClientRect();
    if (el.top < box.top || el.bottom > box.bottom) {
      const reduce = matchMedia('(prefers-reduced-motion: reduce)').matches;
      activeEl.scrollIntoView({
        block: 'nearest',
        behavior: reduce ? 'auto' : 'smooth',
      });
    }
  });

  const rangeLabel = (start: number, end: number) =>
    `${clock(start)} – ${clock(end)}`;
</script>

<nav
  class="chapter-outline"
  class:page={variant === 'page'}
  aria-label="Lecture chapters"
>
  <header class="head">
    <h2 class="title"><ListTree size={14} /> Outline</h2>
    {#if studio.chapters.length}
      <span class="count">{studio.chapters.length}</span>
    {/if}
  </header>

  <div class="scroll" bind:this={listEl}>
    {#if studio.chaptersStatus === 'loading' || (studio.chaptersStatus === 'generating' && !studio.chapters.length)}
      <SkeletonCard
        lines={3}
        label={studio.chaptersStatus === 'generating'
          ? 'Building chapters…'
          : 'Loading chapters…'}
      />
    {:else if studio.chapters.length}
      <ol class="chapters">
        {#each studio.chapters as ch, i (ch.id ?? i)}
          {@const active = studio.activeChapterIndex === i}
          <li>
            <button
              type="button"
              class="chapter"
              class:active
              aria-current={active ? 'step' : undefined}
              onclick={() =>
                studio.seek(ch.start_timestamp_sec, { play: studio.hasAudio })}
              bind:this={
                () => (active ? activeEl : undefined),
                (el) => {
                  if (active && el) activeEl = el;
                }
              }
            >
              <span class="num" aria-hidden="true">{i + 1}</span>
              <span class="body">
                <span class="name">{ch.title}</span>
                <span class="when">
                  {rangeLabel(ch.start_timestamp_sec, ch.end_timestamp_sec)}
                </span>
                {#if active || variant === 'page'}
                  <span class="summary">{ch.summary}</span>
                {/if}
                {#if active}
                  <span
                    class="progress"
                    role="presentation"
                    style:--p="{studio.chapterProgress * 100}%"
                  ></span>
                {/if}
              </span>
            </button>
          </li>
        {/each}
      </ol>
    {:else}
      <div class="empty">
        <p>
          {studio.chaptersError ||
            'Chapters are built from the lecture transcript.'}
        </p>
        <Button
          variant="soft"
          onclick={() => studio.generateChapters()}
          disabled={studio.chaptersStatus === 'generating' ||
            studio.lines.length === 0}
        >
          <Sparkles size={14} />
          {studio.chaptersStatus === 'generating'
            ? 'Generating…'
            : 'Generate chapters'}
        </Button>
        {#if studio.lines.length === 0 && studio.transcriptStatus === 'ready'}
          <p class="needs">Needs a transcript first.</p>
        {/if}
      </div>
    {/if}

    {#if studio.bookmarks.length}
      <section class="moments" aria-label="Saved moments">
        <h3 class="sub"><Bookmark size={13} /> Saved moments</h3>
        <ul>
          {#each studio.bookmarks as b (b.t)}
            <li>
              <button
                type="button"
                class="moment"
                onclick={() => studio.seek(b.t, { play: studio.hasAudio })}
              >
                <span class="stamp">{clock(b.t)}</span>
                <span class="moment-label">{b.label}</span>
              </button>
            </li>
          {/each}
        </ul>
      </section>
    {/if}
  </div>
</nav>

<style>
  .chapter-outline {
    display: flex;
    flex-direction: column;
    min-height: 0;
    height: 100%;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 4px 10px;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    font-family: var(--font-body);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--text-tertiary);
  }
  .count {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 11px;
    color: var(--text-tertiary);
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    scrollbar-width: thin;
    scrollbar-color: var(--color-cork-border) transparent;
    padding-right: 2px;
  }
  .scroll {
    overflow-x: hidden;
  }
  .chapters {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    position: relative;
  }
  .chapter {
    box-sizing: border-box;
    min-width: 0;
    display: flex;
    align-items: flex-start;
    gap: 12px;
    width: 100%;
    min-height: 48px;
    padding: 10px 10px;
    border: 0;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-secondary);
    text-align: left;
    transition:
      background var(--motion-fast) var(--ease-out),
      color var(--motion-fast) var(--ease-out);
  }
  .chapter:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }
  .chapter:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }
  .chapter.active {
    background: var(--surface-accent);
    color: var(--text-primary);
    box-shadow: inset 2px 0 0 var(--color-ember-accent);
  }
  .num {
    flex: none;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    margin-top: 1px;
    border-radius: 50%;
    border: 1px solid var(--border-default);
    font-size: 11px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    color: var(--text-secondary);
  }
  .active .num {
    background: var(--fill-accent);
    border-color: var(--fill-accent);
    color: var(--text-on-accent);
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .name {
    overflow-wrap: anywhere;
    font-size: 14px;
    font-weight: 500;
    line-height: 1.35;
    color: var(--text-primary);
  }
  .when {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 11.5px;
    color: var(--text-tertiary);
    font-variant-numeric: tabular-nums;
  }
  .active .when {
    color: var(--text-accent);
  }
  .summary {
    margin-top: 2px;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--text-secondary);
  }
  .progress {
    display: block;
    height: 3px;
    margin-top: 8px;
    border-radius: 999px;
    background: rgba(255, 237, 215, 0.12);
    position: relative;
    overflow: hidden;
  }
  .progress::after {
    content: '';
    position: absolute;
    inset: 0 auto 0 0;
    width: var(--p);
    background: var(--color-ember-accent);
    border-radius: inherit;
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
    padding: 12px 6px;
  }
  .empty p {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-secondary);
  }
  .empty .needs {
    color: var(--text-tertiary);
  }
  .moments {
    margin-top: 14px;
    padding-top: 12px;
    border-top: 1px solid var(--border-subtle);
  }
  .sub {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 0 4px 6px;
    font-family: var(--font-body);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--text-tertiary);
  }
  .moments ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .moment {
    display: flex;
    align-items: baseline;
    gap: 10px;
    width: 100%;
    min-height: 36px;
    padding: 6px 10px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-secondary);
    text-align: left;
    font-size: 13px;
  }
  .moment:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }
  .moment:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }
  .stamp {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 11.5px;
    color: var(--text-accent);
    flex: none;
  }
  .moment-label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .page .chapter {
    padding: 14px 12px;
  }
</style>
