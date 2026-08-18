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
  </div>

  <div class="header-badges">
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
      <span class="info-item">Code: <strong>{claim.session_code}</strong></span>
    {/if}
  </div>
</div>

<style>
  .studio-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--color-cork-border);
  }
  .header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .header-badges {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .back-btn,
  .action-ghost-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 6px 12px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    min-height: 32px;
    border-radius: var(--radius-controls, 4px);
    transition: background 0.15s ease, border-color 0.15s ease;
  }
  .action-ghost-btn {
    font-size: 11px;
  }
  .verified-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 10px;
    color: #4ade80;
    background: rgba(74, 222, 128, 0.1);
    padding: 4px 8px;
    border-radius: 4px;
    font-weight: 500;
  }
  .studio-title-area {
    margin-top: 4px;
  }
  .session-main-title {
    font-family: var(--font-display);
    font-size: 22px;
    color: var(--color-warm-cream);
    margin: 0 0 4px 0;
    word-break: break-word;
    line-height: 1.25;
  }
  .session-sub-info {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--color-driftwood);
    flex-wrap: wrap;
  }
  .info-item {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .info-divider {
    color: var(--color-cork-border);
  }
  @media (max-width: 640px) {
    .studio-header {
      align-items: flex-start;
    }
    .header-actions {
      flex-direction: column;
      align-items: stretch;
      gap: 6px;
      width: fit-content;
    }
    .back-btn,
    .action-ghost-btn {
      width: 100%;
      justify-content: flex-start;
      padding: 6px 12px;
    }
    .session-main-title {
      font-size: 18px;
    }
  }
</style>
