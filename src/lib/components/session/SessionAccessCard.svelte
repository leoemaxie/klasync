<script lang="ts">
  import QrCodeSvg from '$lib/components/shared/QrCodeSvg.svelte';
  import { Copy, Check, Link as LinkIcon } from '@lucide/svelte';

  let {
    code = '',
    inviteUrl = '',
    copied = false,
    onCopyInvite
  }: {
    code: string;
    inviteUrl: string;
    copied?: boolean;
    onCopyInvite: () => void;
  } = $props();

  let copiedCode = $state(false);

  function handleCopyCode() {
    if (!code) return;
    try {
      navigator.clipboard.writeText(code);
      copiedCode = true;
      setTimeout(() => (copiedCode = false), 2000);
    } catch {}
  }
</script>

<div class="access-grid">
  <div class="access-card code-link-card">
    <div class="card-section">
      <p class="section-label">STUDENT ACCESS CODE</p>
      <div class="code-box">
        <span class="code-display">{code}</span>
        <button type="button" class="copy-code-btn" onclick={handleCopyCode}>
          {#if copiedCode}<Check size={13} class="success-icon" /> Copied{:else}<Copy size={13} /> Copy Code{/if}
        </button>
      </div>
    </div>
    <div class="card-divider"></div>
    <div class="card-section">
      <p class="section-label">DIRECT INVITE LINK</p>
      <div class="url-input-wrap">
        <LinkIcon size={14} aria-hidden="true" class="link-icon" />
        <input readonly value={inviteUrl} aria-label="Direct invite URL" class="url-input" />
        <button type="button" class="outline copy-link-btn" onclick={onCopyInvite}>
          {#if copied}<Check size={13} aria-hidden="true" class="success-icon" /> Copied{:else}<Copy size={13} aria-hidden="true" /> Copy Link{/if}
        </button>
      </div>
    </div>
    <p class="access-hint">Students can join by entering this 8-character code, clicking the link, or scanning the QR code.</p>
  </div>

  <div class="access-card qr-card">
    <p class="section-label">CLASSROOM QR INVITE</p>
    <div class="qr-frame">
      <QrCodeSvg value={inviteUrl || code} size={145} />
    </div>
    <p class="qr-hint">Display on projector for quick mobile scan</p>
  </div>
</div>

<style>
  .access-grid { display: grid; grid-template-columns: 1.6fr 1fr; gap: var(--spacing-16); }
  .access-card { background: rgba(16, 9, 4, 0.6); border: 1px solid var(--color-cork-border); border-radius: 8px; padding: var(--spacing-18); display: flex; flex-direction: column; gap: var(--spacing-14); }
  .section-label { font-size: 11px; letter-spacing: 0.1em; text-transform: uppercase; color: var(--color-driftwood); font-weight: 600; margin: 0 0 6px 0; }
  .code-box { display: flex; align-items: center; gap: var(--spacing-12); flex-wrap: wrap; }
  .code-display { font-family: monospace; font-size: 26px; font-weight: 700; letter-spacing: 0.18em; color: var(--color-warm-cream); background: rgba(10, 5, 2, 0.8); border: 1px solid var(--color-cork-border); padding: 8px 18px; border-radius: 6px; }
  .copy-code-btn { background: var(--color-bark-brown); border: 1px solid var(--color-cork-border); color: var(--color-warm-cream); padding: 8px 14px; border-radius: 6px; font-size: 11px; text-transform: uppercase; cursor: pointer; display: inline-flex; align-items: center; gap: 6px; }
  .card-divider { height: 1px; background: var(--color-cork-border); opacity: 0.5; }
  .url-input-wrap { position: relative; display: flex; align-items: center; gap: 8px; }
  .url-input { width: 100%; padding: 8px 12px 8px 32px; background: rgba(10, 5, 2, 0.6); border: 1px solid var(--color-cork-border); border-radius: 6px; color: var(--color-warm-cream); font-size: 11px; font-family: monospace; }
  .copy-link-btn { font-size: 11px; padding: 7px 14px; text-transform: uppercase; display: inline-flex; align-items: center; gap: 6px; }
  .access-hint { font-size: 11px; color: var(--color-driftwood); margin: 0; }
  .qr-card { align-items: center; text-align: center; justify-content: center; }
  .qr-frame { padding: 10px; background: var(--color-warm-cream); border-radius: 8px; display: inline-flex; box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4); }
  .qr-hint { font-size: 11px; color: var(--color-driftwood); margin: 0; }
  @media (max-width: 768px) { .access-grid { grid-template-columns: 1fr; } }
</style>
