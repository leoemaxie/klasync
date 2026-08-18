<script lang="ts">
  import type { ClaimRecord } from '$lib/api';
  import {
    ArrowLeft,
    CheckCircle2,
    Share2,
    Calendar,
    Check,
  } from '@lucide/svelte';
  import { triggerHaptic } from '$lib/native/haptics';

  let { claim, onBack }: { claim: ClaimRecord; onBack?: () => void } = $props();
  let isCopied = $state(false);

  function handleCopyShare() {
    triggerHaptic('light');
    const url = `${window.location.origin}/#archive?lecture=${encodeURIComponent(claim.id)}`;
    navigator.clipboard.writeText(url).then(() => {
      isCopied = true;
      setTimeout(() => (isCopied = false), 2000);
    });
  }
</script>

<div class="studio-header">
  <div class="header-actions">
    {#if onBack}
      <button
        type="button"
        class="back-btn outline"
        onclick={onBack}
        title="Back to lectures"
      >
        <ArrowLeft size={14} /><span>Back</span>
      </button>
    {/if}

    {#if claim.course_code}
      <span class="course-code-pill">{claim.course_code}</span>
    {/if}
  </div>

  <div class="header-badges">
    <button
      type="button"
      class="action-ghost-btn outline"
      onclick={handleCopyShare}
      title="Share lecture archive link"
    >
      {#if isCopied}
        <Check size={12} color="var(--color-warm-cream)" />
        <span>Copied</span>
      {:else}
        <Share2 size={12} />
        <span>Share</span>
      {/if}
    </button>

    <div class="verified-pill">
      <CheckCircle2 size={12} /><span>Verified</span>
    </div>
  </div>
</div>

<div class="studio-title-area">
  <h1 class="session-main-title">{claim.session_title}</h1>
  <div class="session-sub-info">
    <span class="info-item"><Calendar size={12} /> Saved {claim.date}</span>
    {#if claim.session_code}
      <span class="info-divider">·</span>
      <span class="info-item"
        >Session Code: <strong>{claim.session_code}</strong></span
      >
    {/if}
  </div>
</div>

<style>
  .studio-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--spacing-10);
    padding-bottom: var(--spacing-10);
    border-bottom: 1px solid var(--color-cork-border);
  }
  .header-actions {
    display: flex;
    align-items: center;
    gap: var(--spacing-8);
  }
  .header-badges {
    display: flex;
    align-items: center;
    gap: var(--spacing-8);
  }
  .course-code-pill {
    display: inline-flex;
    align-items: center;
    font-family: var(--font-mono, monospace);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.06em;
    color: var(--color-ember-accent);
    background: rgba(220, 80, 0, 0.1);
    border: 1px solid rgba(220, 80, 0, 0.25);
    padding: 3px 9px;
    border-radius: 4px;
  }
  .back-btn,
  .action-ghost-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 5px 12px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    min-height: 30px;
    border-radius: var(--radius-controls, 4px);
    color: var(--color-warm-cream-dim);
    transition:
      background 0.15s ease,
      border-color 0.15s ease,
      color 0.15s ease;
  }
  .back-btn:hover,
  .action-ghost-btn:hover {
    color: var(--color-warm-cream);
    border-color: var(--color-warm-cream);
  }
  .verified-pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    color: #4ade80;
    background: rgba(74, 222, 128, 0.12);
    border: 1px solid rgba(74, 222, 128, 0.25);
    padding: 4px 10px;
    border-radius: 4px;
    font-weight: 600;
    letter-spacing: 0.02em;
  }
  .studio-title-area {
    margin-top: 6px;
  }
  .session-main-title {
    font-family: var(--font-display);
    font-size: 24px;
    color: var(--color-warm-cream);
    margin: 0 0 6px 0;
    word-break: break-word;
    line-height: 1.25;
    font-weight: 700;
  }
  .session-sub-info {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--color-warm-cream-dim);
    flex-wrap: wrap;
  }
  .info-item {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .info-item strong {
    color: var(--color-warm-cream);
    font-family: var(--font-mono, monospace);
  }
  .info-divider {
    color: var(--color-cork-border);
  }
  @media (max-width: 640px) {
    .studio-header {
      align-items: flex-start;
      flex-direction: column;
      gap: 8px;
    }
    .header-badges {
      width: 100%;
      justify-content: space-between;
    }
    .session-main-title {
      font-size: 19px;
    }
  }
</style>
