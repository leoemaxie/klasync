<script lang="ts">
  import { onMount } from 'svelte';
  import {
    Shuffle,
    RotateCcw,
    Sparkles,
    ChevronLeft,
    ChevronRight,
    Layers,
    PartyPopper,
    ChevronDown,
  } from '@lucide/svelte';
  import SkeletonCard from '$lib/components/shared/SkeletonCard.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import EmptyState from '$lib/components/ui/EmptyState.svelte';
  import Segmented from '$lib/components/ui/Segmented.svelte';
  import { triggerHaptic } from '$lib/native/haptics';
  import { useStudio } from '$lib/studio/lectureStudio.svelte';
  import { FlashcardDeckState } from '$lib/studio/flashcardDeck.svelte';
  import FlashCard from './FlashCard.svelte';

  const studio = useStudio();
  const deck = new FlashcardDeckState(studio.sessionKey);
  onMount(() => void deck.load());

  const QUICK = ['Key formulas', 'Core definitions', 'Exam review'];
  let topic = $state('');
  let makeOpen = $state(false);

  // Text selected in the transcript arrives as a ready-to-run topic.
  $effect(() => {
    if (!studio.flashcardSeed) return;
    topic = studio.flashcardSeed;
    makeOpen = true;
    studio.flashcardSeed = '';
  });

  const showMake = $derived(
    makeOpen || (deck.status === 'ready' && !deck.deck.length)
  );

  async function generate(value = topic) {
    triggerHaptic('medium');
    await deck.generate(value.trim() || undefined);
    if (!deck.error) {
      topic = '';
      makeOpen = false;
    }
  }

  function press(fn: () => void) {
    triggerHaptic('light');
    fn();
  }

  // Arrow keys move through the deck while focus is inside this pane; the
  // workspace's seek shortcuts only run when nothing here has handled them.
  function onKeydown(event: KeyboardEvent) {
    const t = event.target as HTMLElement;
    if (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA') return;
    if (event.key === 'ArrowRight') {
      event.preventDefault();
      deck.go(1);
    } else if (event.key === 'ArrowLeft') {
      event.preventDefault();
      deck.go(-1);
    }
  }

  const position = $derived(
    deck.current ? `${deck.index + 1} of ${deck.visible.length}` : ''
  );
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="cards" role="region" aria-label="Flashcards" onkeydown={onKeydown}>
  <div class="deck-bar">
    <Segmented
      label="Cards to show"
      value={deck.filter}
      options={[
        { value: 'all', label: `All ${deck.deck.length}` },
        {
          value: 'learning',
          label: `To learn ${deck.deck.length - deck.masteredCount}`,
        },
      ]}
      onchange={(v) => deck.setFilter(v)}
    />
    <div class="tools">
      <Button
        icon
        title="Shuffle"
        aria-label="Shuffle cards"
        disabled={deck.deck.length < 2}
        onclick={() => press(() => deck.shuffle())}
        ><Shuffle size={16} /></Button
      >
      <Button
        icon
        title="Reset progress"
        aria-label="Reset progress"
        disabled={deck.masteredCount === 0}
        onclick={() => press(() => deck.resetProgress())}
        ><RotateCcw size={16} /></Button
      >
      <Button
        variant={showMake ? 'soft' : 'outline'}
        aria-expanded={showMake}
        onclick={() => (makeOpen = !makeOpen)}
        disabled={!deck.deck.length && deck.status === 'ready'}
      >
        <Sparkles size={14} /> More cards
        <ChevronDown size={14} />
      </Button>
    </div>
  </div>

  {#if showMake}
    <form
      class="make"
      onsubmit={(e) => {
        e.preventDefault();
        void generate();
      }}
    >
      <div class="row">
        <input
          type="text"
          bind:value={topic}
          placeholder="Optional topic, e.g. the Calvin cycle"
          aria-label="Topic for new flashcards"
          maxlength="200"
        />
        <Button variant="accent" type="submit" disabled={deck.generating}>
          <Sparkles size={14} />
          {deck.generating ? 'Generating…' : 'Generate'}
        </Button>
      </div>
      <div class="quick" role="group" aria-label="Suggested topics">
        {#each QUICK as q}
          <Button
            size="sm"
            variant="outline"
            disabled={deck.generating}
            onclick={() => void generate(q)}>{q}</Button
          >
        {/each}
      </div>
    </form>
  {/if}

  {#if deck.generating}
    <p class="status" role="status">
      Generating flashcards from the lecture. This can take up to a minute…
    </p>
  {/if}
  {#if deck.error}
    <p class="status err" role="alert">{deck.error}</p>
  {/if}

  {#if deck.status === 'loading'}
    <SkeletonCard lines={3} label="Loading flashcards…" />
  {:else if !deck.deck.length}
    <EmptyState
      icon={Layers}
      title="No flashcards yet"
      description="Generate a deck from the whole lecture, or add a topic to focus it."
    >
      {#if !deck.generating}
        <Button variant="accent" onclick={() => void generate('')}>
          <Sparkles size={14} /> Generate from lecture
        </Button>
      {/if}
    </EmptyState>
  {:else if !deck.current}
    <EmptyState
      icon={PartyPopper}
      title="All caught up"
      description="You’ve marked every card as known. Review the full deck any time."
    >
      <Button variant="outline" onclick={() => deck.setFilter('all')}
        >Review all cards</Button
      >
    </EmptyState>
  {:else}
    <div class="progress" aria-hidden="true">
      <span style:width="{((deck.index + 1) / deck.visible.length) * 100}%"
      ></span>
    </div>
    <p class="count" aria-live="polite">
      Card {position} · {deck.masteredCount} of {deck.deck.length} known
    </p>

    <FlashCard
      card={deck.current}
      flipped={deck.flipped}
      {position}
      onFlip={() => press(() => deck.flip())}
      onSwipe={(d) => press(() => deck.go(d))}
    />

    <div class="deck-actions">
      {#if deck.flipped}
        <Button
          variant="outline"
          class="grow"
          onclick={() => press(() => deck.rate(false))}>Still learning</Button
        >
        <Button
          variant="accent"
          class="grow"
          onclick={() => press(() => deck.rate(true))}>Got it</Button
        >
      {:else}
        <Button
          variant="outline"
          icon
          aria-label="Previous card"
          disabled={deck.visible.length < 2}
          onclick={() => press(() => deck.go(-1))}
          ><ChevronLeft size={18} /></Button
        >
        <Button
          variant="solid"
          class="grow reveal"
          onclick={() => press(() => deck.flip())}>Show answer</Button
        >
        <Button
          variant="outline"
          icon
          aria-label="Next card"
          disabled={deck.visible.length < 2}
          onclick={() => press(() => deck.go(1))}
          ><ChevronRight size={18} /></Button
        >
      {/if}
    </div>
  {/if}
</div>

<style>
  .cards {
    display: flex;
    flex-direction: column;
    gap: 12px;
    height: 100%;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    scrollbar-width: thin;
    scrollbar-color: var(--color-cork-border) transparent;
  }
  .deck-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 8px;
    flex: none;
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .make {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    background: var(--surface-inset);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-lg);
    animation: open var(--motion-base) var(--ease-out);
  }
  .row {
    display: flex;
    gap: 8px;
  }
  .row input {
    flex: 1;
    min-width: 0;
    height: var(--control-h);
    min-height: var(--control-h);
    margin: 0;
    padding: 0 16px;
    border: 1px solid var(--border-default);
    border-radius: 999px;
    background: var(--surface-1);
    color: var(--text-primary);
    font-family: var(--font-body);
    font-size: 13.5px;
    letter-spacing: 0;
    text-transform: none;
  }
  .row input::placeholder {
    color: var(--text-tertiary);
    font-style: normal;
  }
  .row input:focus {
    outline: none;
    border-color: var(--border-strong);
    box-shadow: var(--focus-ring);
  }
  .quick {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .status {
    flex: none;
    margin: 0;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .status.err {
    color: var(--color-error);
  }
  .progress {
    flex: none;
    height: 4px;
    border-radius: 999px;
    background: rgba(255, 237, 215, 0.12);
    overflow: hidden;
  }
  .progress span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--color-ember-accent);
    transition: width var(--motion-slow) var(--ease-out);
  }
  .count {
    flex: none;
    margin: -4px 0 0;
    font-size: 12.5px;
    color: var(--text-tertiary);
    font-variant-numeric: tabular-nums;
  }
  .deck-actions {
    flex: none;
    display: flex;
    gap: 8px;
  }
  .cards :global(.grow) {
    flex: 1;
  }
  .cards :global(.reveal) {
    height: 44px;
    min-height: 44px;
  }
  @keyframes open {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
  }
</style>
