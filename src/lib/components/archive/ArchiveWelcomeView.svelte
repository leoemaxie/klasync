<script lang="ts">
  import type { ClaimRecord } from '$lib/api';
  import {
    Play,
    RotateCcw,
    BookOpen,
    FileText,
    Layers,
    Headphones,
  } from '@lucide/svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import EmptyState from '$lib/components/ui/EmptyState.svelte';
  import { clock } from '$lib/studio/time';
  import { lastResumed, progressOf } from '$lib/studio/resume';

  let {
    claims = [],
    resumeVersion = 0,
    onOpenLatest,
    onOpen,
  }: {
    claims: ClaimRecord[];
    resumeVersion?: number;
    /** Open a lecture from the start (or resuming where it was left). */
    onOpenLatest: (claim: ClaimRecord) => void;
    /** Open at an exact moment, or at 0 to start over. */
    onOpen?: (claim: ClaimRecord, startAt: number) => void;
  } = $props();

  const resumed = $derived.by(() => {
    void resumeVersion;
    const r = lastResumed();
    const claim = r ? claims.find((c) => c.id === r.id) : undefined;
    return r && claim ? { ...r, claim } : null;
  });
  const recent = $derived(
    claims.filter((c) => c.id !== resumed?.claim.id).slice(0, 4)
  );

  const tiles = [
    {
      icon: FileText,
      title: 'Follow along',
      text: 'A transcript that scrolls with the recording.',
    },
    {
      icon: Layers,
      title: 'Study',
      text: 'Flashcards, summaries and a concept map.',
    },
    {
      icon: Headphones,
      title: 'Replay',
      text: 'Jump to any moment, at your speed.',
    },
  ];
</script>

<div class="welcome">
  {#if claims.length === 0}
    <EmptyState
      icon={BookOpen}
      title="Your archive is empty"
      description="Join a live lecture and it will be saved here, with its transcript, notes and study tools."
    />
  {:else}
    {#if resumed}
      {@const progress = progressOf(resumed)}
      <section class="continue" aria-label="Continue studying">
        <p class="eyebrow-line">Continue where you left off</p>
        <p class="course">{resumed.claim.course_code}</p>
        <h2>{resumed.claim.session_title}</h2>
        {#if progress != null}
          <div class="bar-track" aria-hidden="true">
            <i style:width="{progress * 100}%"></i>
          </div>
        {/if}
        <p class="where">
          Stopped at <strong>{clock(resumed.t)}</strong>{resumed.d > 0
            ? ` of ${clock(resumed.d)}`
            : ''}
        </p>
        <div class="buttons">
          <Button variant="accent" onclick={() => onOpenLatest(resumed.claim)}>
            <Play size={14} fill="currentColor" /> Continue from {clock(
              resumed.t
            )}
          </Button>
          <Button variant="outline" onclick={() => onOpen?.(resumed.claim, 0)}>
            <RotateCcw size={14} /> Start over
          </Button>
        </div>
      </section>
    {:else}
      <section class="continue" aria-label="Start studying">
        <p class="eyebrow-line">Study studio</p>
        <h2>Pick a lecture to start</h2>
        <p class="where">
          Choose one from the list, or jump straight into your latest.
        </p>
        <div class="buttons">
          <Button variant="accent" onclick={() => onOpenLatest(claims[0])}>
            <BookOpen size={14} /> Open latest · {claims[0].course_code}
          </Button>
        </div>
      </section>
    {/if}

    {#if recent.length}
      <section aria-label="Recent lectures">
        <h3 class="section">Recent</h3>
        <ul class="recent">
          {#each recent as claim (claim.id)}
            <li>
              <button type="button" onclick={() => onOpenLatest(claim)}>
                <span class="r-course">{claim.course_code}</span>
                <span class="r-title">{claim.session_title}</span>
                <span class="r-date">{claim.date}</span>
              </button>
            </li>
          {/each}
        </ul>
      </section>
    {/if}
  {/if}

  <ul class="tiles" aria-label="What you can do in a lecture">
    {#each tiles as t}
      {@const Icon = t.icon}
      <li>
        <span class="t-icon"><Icon size={17} /></span>
        <strong>{t.title}</strong>
        <span>{t.text}</span>
      </li>
    {/each}
  </ul>
</div>

<style>
  .welcome {
    display: flex;
    flex-direction: column;
    gap: 22px;
    max-width: 760px;
    margin: 0 auto;
    padding: 8px 0 24px;
  }
  .continue {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 24px 26px;
    border: 1px solid var(--border-accent);
    border-radius: var(--radius-xl);
    background:
      radial-gradient(
        120% 140% at 0% 0%,
        rgba(220, 80, 0, 0.2),
        transparent 60%
      ),
      var(--surface-2);
    box-shadow: var(--elev-2);
  }
  .eyebrow-line {
    margin: 0;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--text-accent);
  }
  .course {
    margin: 0;
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 12.5px;
    letter-spacing: 0.08em;
    color: var(--text-secondary);
  }
  .continue h2 {
    margin: 0;
    font-size: clamp(24px, 3vw, 32px);
    line-height: 1.15;
    text-wrap: balance;
  }
  .bar-track {
    height: 4px;
    margin-top: 6px;
    border-radius: 999px;
    background: rgba(255, 237, 215, 0.12);
    overflow: hidden;
  }
  .bar-track i {
    display: block;
    height: 100%;
    background: var(--color-ember-accent);
  }
  .where {
    margin: 0;
    font-size: 14px;
    color: var(--text-secondary);
  }
  .where strong {
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
  }
  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 8px;
  }
  .section {
    margin: 0 0 8px;
    font-family: var(--font-body);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--text-tertiary);
  }
  .recent {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 8px;
  }
  .recent button {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: baseline;
    gap: 10px;
    width: 100%;
    padding: 12px 14px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    background: var(--surface-inset);
    color: var(--text-primary);
    text-align: left;
    font-family: var(--font-body);
    font-size: 14px;
    letter-spacing: 0;
    text-transform: none;
    white-space: normal;
    transition:
      border-color var(--motion-fast) var(--ease-out),
      background var(--motion-fast) var(--ease-out);
  }
  .recent button:hover {
    border-color: var(--border-strong);
    background: var(--surface-1);
  }
  .recent button:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }
  .r-course {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 12px;
    color: var(--text-accent);
  }
  .r-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .r-date {
    font-size: 12px;
    color: var(--text-secondary);
  }
  .tiles {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(190px, 1fr));
    gap: 10px;
  }
  .tiles li {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 14px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-secondary);
  }
  .tiles strong {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }
  .t-icon {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    margin-bottom: 4px;
    border-radius: 50%;
    background: var(--surface-accent);
    color: var(--text-accent);
  }
</style>
