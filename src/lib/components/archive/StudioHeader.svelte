<script lang="ts">
  import type { ClaimRecord } from '$lib/api';
  import {
    ArrowLeft,
    CheckCircle2,
    Share2,
    Check,
    Keyboard,
    PanelLeft,
    PanelLeftClose,
    Focus,
    Shrink,
  } from '@lucide/svelte';
  import { triggerHaptic } from '$lib/native/haptics';
  import { clock } from '$lib/studio/time';
  import Button from '$lib/components/ui/Button.svelte';

  let {
    claim,
    onBack,
    duration = 0,
    focus = false,
    listOpen = false,
    showListToggle = false,
    onToggleFocus,
    onToggleList,
    onShortcuts,
  }: {
    claim: ClaimRecord;
    onBack?: () => void;
    duration?: number;
    focus?: boolean;
    listOpen?: boolean;
    showListToggle?: boolean;
    onToggleFocus?: () => void;
    onToggleList?: () => void;
    onShortcuts?: () => void;
  } = $props();

  let isCopied = $state(false);

  function handleCopyShare() {
    triggerHaptic('light');
    const url = `${window.location.origin}/#/archive?lecture=${encodeURIComponent(claim.id)}`;
    navigator.clipboard
      .writeText(url)
      .then(() => {
        isCopied = true;
        setTimeout(() => (isCopied = false), 2000);
      })
      .catch(() => {});
  }
</script>

<header class="studio-header" class:focus>
  <div class="lead">
    {#if onBack}
      <Button
        variant="outline"
        class="back"
        onclick={onBack}
        aria-label="Back to lectures"
      >
        <ArrowLeft size={15} /><span>Lectures</span>
      </Button>
    {/if}
    {#if showListToggle}
      <Button
        variant="outline"
        class="list-toggle"
        onclick={onToggleList}
        pressed={listOpen}
        title={listOpen ? 'Hide lecture list' : 'Show lecture list'}
      >
        {#if listOpen}<PanelLeftClose size={15} />{:else}<PanelLeft
            size={15}
          />{/if}<span>Lectures</span>
      </Button>
    {/if}

    <div class="title-block">
      {#if !focus}
        <p class="eyebrow-row">
          <span class="course-code">{claim.course_code}</span>
          {#if claim.course_title}<span class="dot" aria-hidden="true">·</span
            ><span class="course">{claim.course_title}</span>{/if}
          <span class="dot" aria-hidden="true">·</span>
          <time>{claim.date}</time>
          {#if duration > 0}<span class="dot" aria-hidden="true">·</span><span
              >{clock(duration)} long</span
            >{/if}
        </p>
      {/if}
      <h1 class="title">{claim.session_title}</h1>
      {#if !focus}
        <div class="meta">
          {#if claim.session_code}<span class="pill mono" title="Session code"
              >{claim.session_code}</span
            >{/if}
          {#if claim.attended !== false}
            <span class="pill ok"><CheckCircle2 size={12} /> Verified</span>
          {/if}
        </div>
      {/if}
    </div>
  </div>

  <div class="header-actions">
    <Button
      icon
      onclick={handleCopyShare}
      aria-label="Copy link to this lecture"
      title="Copy link"
    >
      {#if isCopied}<Check size={17} />{:else}<Share2 size={17} />{/if}
    </Button>
    <Button
      icon
      class="kbd-btn"
      onclick={onShortcuts}
      aria-label="Keyboard shortcuts"
      title="Keyboard shortcuts (?)"
    >
      <Keyboard size={17} />
    </Button>
    <Button
      icon
      class="focus-btn"
      pressed={focus}
      onclick={onToggleFocus}
      aria-label={focus ? 'Exit focus mode' : 'Enter focus mode'}
      title="Focus mode (F)"
    >
      {#if focus}<Shrink size={17} />{:else}<Focus size={17} />{/if}
    </Button>
  </div>
</header>

<span class="sr-only" role="status">{isCopied ? 'Link copied' : ''}</span>

<style>
  .studio-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    min-width: 0;
  }
  .studio-header.focus {
    align-items: center;
  }
  .lead {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    min-width: 0;
    flex: 1;
  }
  .focus .lead {
    align-items: center;
  }
  .title-block {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .eyebrow-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    margin: 0;
    font-size: 12.5px;
    color: var(--text-secondary);
  }
  .course-code {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.08em;
    color: var(--text-accent);
  }
  .dot {
    color: var(--text-tertiary);
  }
  .title {
    margin: 0;
    font-family: var(--font-display);
    font-size: clamp(22px, 2.4vw, 30px);
    font-weight: 500;
    line-height: 1.15;
    letter-spacing: -0.015em;
    color: var(--text-primary);
    text-wrap: balance;
    overflow-wrap: anywhere;
  }
  .focus .title {
    font-size: 19px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 4px;
  }
  .pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 24px;
    padding: 0 10px;
    border: 1px solid var(--border-default);
    border-radius: 999px;
    font-size: 12px;
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }
  .pill.mono {
    font-family: var(--font-mono, ui-monospace, monospace);
    letter-spacing: 0.04em;
  }
  .pill.ok {
    color: #7ddc9f;
    border-color: rgba(125, 220, 159, 0.3);
    background: rgba(125, 220, 159, 0.08);
  }
  .header-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
  }
  .studio-header :global(.back) {
    display: none;
  }
  @media (max-width: 960px) {
    .studio-header :global(.back) {
      display: inline-flex;
    }
    .studio-header :global(.list-toggle),
    .studio-header :global(.focus-btn) {
      display: none;
    }
  }
  @media (max-width: 720px) {
    .title {
      font-size: 20px;
    }
    .studio-header {
      gap: 8px;
    }
    .lead {
      gap: 8px;
    }
    .studio-header {
      position: relative;
      flex-direction: column;
      gap: 6px;
    }
    .lead {
      width: 100%;
      flex-direction: column;
      align-items: stretch;
      gap: 10px;
    }
    .studio-header :global(.back) {
      align-self: flex-start;
    }
    .header-actions {
      position: absolute;
      top: 0;
      right: 0;
    }
    .studio-header :global(.kbd-btn) {
      display: none;
    }
    .meta {
      display: none;
    }
  }
</style>
