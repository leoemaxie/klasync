<script lang="ts">
  import type { ClaimRecord } from '$lib/api';
  import { ArrowLeft, CheckCircle2, Share2, Calendar, Check } from '@lucide/svelte';
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
  <div class="header-left">
    {#if onBack}
      <button type="button" class="back-btn outline" onclick={onBack} title="Back to explorer">
        <ArrowLeft size={14} /><span>Explorer</span>
      </button>
    {/if}
    <div class="meta-tags-row">
      <span class="course-code-badge">{claim.course_code}</span>
      {#if claim.course_title && claim.course_title !== claim.course_code}
        <span class="course-title-sub">{claim.course_title}</span>
      {/if}
      {#if claim.academic_session}<span class="meta-pill">{claim.academic_session}</span>{/if}
    </div>
  </div>

  <div class="header-right">
    <button type="button" class="action-ghost-btn outline" onclick={handleCopyShare}>
      {#if isCopied}<Check size={12} color="var(--color-warm-cream)" /> <span>Copied</span>{:else}<Share2 size={12} /> <span>Share</span>{/if}
    </button>
    <div class="verified-pill"><CheckCircle2 size={12} /><span>Verified</span></div>
  </div>
</div>

<div class="studio-title-area">
  <h1 class="session-main-title">{claim.session_title}</h1>
  <div class="session-sub-info">
    <span class="info-item"><Calendar size={12} /> Claimed on {claim.date}</span>
    {#if claim.session_code}
      <span class="info-divider">·</span>
      <span class="info-item">Session Code: <strong>{claim.session_code}</strong></span>
    {/if}
  </div>
</div>

<style>
  .studio-header { display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 8px; padding-bottom: 8px; border-bottom: 1px solid var(--color-cork-border); }
  .header-left, .meta-tags-row, .header-right { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
  .back-btn { display: inline-flex; align-items: center; gap: 4px; padding: 4px 8px; font-size: 11px; text-transform: uppercase; min-height: 32px; }
  .course-code-badge { font-family: var(--font-mono, monospace); font-size: 12px; font-weight: 700; color: var(--color-ember-accent); }
  .course-title-sub { font-size: 11px; color: var(--color-driftwood); text-transform: uppercase; }
  .meta-pill { font-size: 9px; background: rgba(255, 237, 215, 0.08); color: var(--color-warm-cream-dim); padding: 2px 6px; border-radius: 3px; }
  .action-ghost-btn { display: inline-flex; align-items: center; gap: 4px; font-size: 10px; padding: 4px 8px; text-transform: uppercase; min-height: 32px; }
  .verified-pill { display: inline-flex; align-items: center; gap: 4px; font-size: 10px; color: #4ade80; background: rgba(74, 222, 128, 0.1); padding: 3px 8px; border-radius: 4px; }
  .studio-title-area { margin-top: 4px; }
  .session-main-title { font-family: var(--font-display); font-size: 22px; color: var(--color-warm-cream); margin: 0 0 4px 0; word-break: break-word; }
  .session-sub-info { display: flex; align-items: center; gap: 6px; font-size: 11px; color: var(--color-driftwood); flex-wrap: wrap; }
  .info-item { display: inline-flex; align-items: center; gap: 4px; }
  .info-divider { color: var(--color-cork-border); }
  @media (max-width: 640px) {
    .session-main-title { font-size: 18px; }
    .header-right { width: 100%; justify-content: space-between; }
  }
</style>
