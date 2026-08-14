<script lang="ts">
  import QrCodeSvg from '$lib/components/shared/QrCodeSvg.svelte';
  import { Copy, Check, Link as LinkIcon, Share2, Maximize2, X } from '@lucide/svelte';
  import { triggerHaptic } from '$lib/native/haptics';

  let {
    code = '',
    inviteUrl = '',
    copied = false,
    onCopyInvite,
  }: {
    code: string;
    inviteUrl: string;
    copied?: boolean;
    onCopyInvite: () => void;
  } = $props();

  let copiedCode = $state(false);
  let isQrEnlarged = $state(false);
  const canNativeShare = typeof navigator !== 'undefined' && !!navigator.share;

  function handleCopyCode() {
    if (!code) return;
    try {
      navigator.clipboard.writeText(code);
      triggerHaptic('success');
      copiedCode = true;
      setTimeout(() => (copiedCode = false), 2000);
    } catch {}
  }

  async function handleShareInvite() {
    triggerHaptic('light');
    if (canNativeShare) {
      try {
        await navigator.share({
          title: `Klasync Live Session`,
          text: `Join the live lecture using code: ${code}\n`,
          url: inviteUrl || undefined,
        });
        return;
      } catch {
        // Fallback to copy if user cancels or share fails
      }
    }
    onCopyInvite();
  }
</script>

<div class="access-grid">
  <div class="access-card code-link-card">
    <div class="card-section">
      <p class="section-label">STUDENT ACCESS CODE</p>
      <div class="code-box">
        <span class="code-display">{code}</span>
        <div class="code-actions">
          <button type="button" class="copy-code-btn" onclick={handleCopyCode}>
            {#if copiedCode}<Check size={13} class="success-icon" /> Copied{:else}<Copy
                size={13}
              /> Copy Code{/if}
          </button>
          {#if canNativeShare}
            <button type="button" class="share-btn" onclick={handleShareInvite} aria-label="Share session invite">
              <Share2 size={13} /> Share
            </button>
          {/if}
        </div>
      </div>
    </div>
    <div class="card-divider"></div>
    <div class="card-section">
      <p class="section-label">DIRECT INVITE LINK</p>
      <div class="url-input-wrap">
        <LinkIcon size={14} aria-hidden="true" class="link-icon" />
        <input
          readonly
          value={inviteUrl}
          aria-label="Direct invite URL"
          class="url-input"
        />
        <button
          type="button"
          class="outline copy-link-btn"
          onclick={() => {
            triggerHaptic('success');
            onCopyInvite();
          }}
        >
          {#if copied}<Check
              size={13}
              aria-hidden="true"
              class="success-icon"
            /> Copied{:else}<Copy size={13} aria-hidden="true" /> Copy{/if}
        </button>
      </div>
    </div>
    <p class="access-hint">
      Students can join by entering this 8-character code, clicking the link, or
      scanning the QR code.
    </p>
  </div>

  <div class="access-card qr-card">
    <div class="qr-header">
      <p class="section-label">CLASSROOM QR INVITE</p>
      <button
        type="button"
        class="enlarge-qr-btn"
        onclick={() => (isQrEnlarged = true)}
        aria-label="Enlarge QR Code"
        title="Enlarge QR Code"
      >
        <Maximize2 size={13} />
      </button>
    </div>
    <button
      type="button"
      class="qr-frame-btn"
      onclick={() => (isQrEnlarged = true)}
      aria-label="Tap to enlarge QR Code"
    >
      <div class="qr-frame">
        <QrCodeSvg value={inviteUrl || code} size={140} />
      </div>
    </button>
    <p class="qr-hint">Tap to enlarge or show on projector</p>
  </div>
</div>

{#if isQrEnlarged}
  <div
    class="qr-modal-backdrop"
    onclick={(e) => e.target === e.currentTarget && (isQrEnlarged = false)}
    role="dialog"
    aria-modal="true"
    aria-label="Enlarged QR Code"
  >
    <div class="qr-modal-card">
      <div class="qr-modal-top">
        <div>
          <span class="qr-modal-code">{code}</span>
          <p class="qr-modal-sub">Scan to join live lecture</p>
        </div>
        <button
          type="button"
          class="close-qr-btn"
          onclick={() => (isQrEnlarged = false)}
          aria-label="Close QR modal"
        >
          <X size={18} />
        </button>
      </div>
      <div class="qr-modal-frame">
        <QrCodeSvg value={inviteUrl || code} size={260} />
      </div>
      <p class="qr-modal-hint">Point phone camera at the screen to join directly</p>
    </div>
  </div>
{/if}

<style>
  .access-grid {
    display: grid;
    grid-template-columns: 1.6fr 1fr;
    gap: var(--spacing-16, 16px);
  }
  .access-card {
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border, #40372e);
    border-radius: 8px;
    padding: var(--spacing-18, 18px);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-14, 14px);
  }
  .section-label {
    font-size: 11px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--color-driftwood, #b8a794);
    font-weight: 600;
    margin: 0 0 6px 0;
  }
  .code-box {
    display: flex;
    align-items: center;
    gap: var(--spacing-12, 12px);
    flex-wrap: wrap;
  }
  .code-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .code-display {
    font-family: var(--font-mono, monospace);
    font-size: 26px;
    font-weight: 700;
    letter-spacing: 0.18em;
    color: var(--color-warm-cream, #ffedd7);
    background: rgba(10, 5, 2, 0.8);
    border: 1px solid var(--color-cork-border, #40372e);
    padding: 8px 18px;
    border-radius: 6px;
  }
  .copy-code-btn,
  .share-btn {
    background: var(--color-bark-brown, #382416);
    border: 1px solid var(--color-cork-border, #40372e);
    color: var(--color-warm-cream, #ffedd7);
    padding: 8px 14px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    transition: all 0.15s ease;
  }
  .share-btn {
    background: var(--color-ember-accent, #dc5000);
    border-color: var(--color-ember-accent, #dc5000);
  }
  .copy-code-btn:hover {
    background: var(--color-bark-glow, #4a3020);
    color: #ffffff;
  }
  .share-btn:hover {
    filter: brightness(1.1);
  }
  .card-divider {
    height: 1px;
    background: var(--color-cork-border, #40372e);
    opacity: 0.5;
  }
  .url-input-wrap {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .url-input {
    width: 100%;
    padding: 8px 12px 8px 32px;
    background: rgba(10, 5, 2, 0.6);
    border: 1px solid var(--color-cork-border, #40372e);
    border-radius: 6px;
    color: var(--color-warm-cream, #ffedd7);
    font-size: 11px;
    font-family: var(--font-mono, monospace);
  }
  .copy-link-btn {
    font-size: 11px;
    padding: 7px 14px;
    text-transform: uppercase;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .access-hint {
    font-size: 11px;
    color: var(--color-driftwood, #b8a794);
    margin: 0;
  }
  .qr-card {
    align-items: center;
    text-align: center;
    justify-content: center;
  }
  .qr-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
  }
  .enlarge-qr-btn {
    background: transparent;
    border: none;
    color: var(--color-driftwood, #b8a794);
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
    display: inline-flex;
    align-items: center;
  }
  .enlarge-qr-btn:hover {
    color: var(--color-warm-cream, #ffedd7);
  }
  .qr-frame-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 0;
    display: inline-flex;
  }
  .qr-frame {
    padding: 10px;
    background: var(--color-warm-cream, #ffedd7);
    border-radius: 8px;
    display: inline-flex;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
    transition: transform 0.15s ease;
  }
  .qr-frame-btn:hover .qr-frame {
    transform: scale(1.03);
  }
  .qr-hint {
    font-size: 11px;
    color: var(--color-driftwood, #b8a794);
    margin: 0;
  }

  /* Enlarged QR Modal */
  .qr-modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 210;
    background: rgba(8, 4, 2, 0.9);
    backdrop-filter: blur(10px);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 20px;
  }
  .qr-modal-card {
    background: #140b05;
    border: 1px solid var(--color-cork-border, #40372e);
    border-radius: 12px;
    padding: 24px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    max-width: 360px;
    width: 100%;
    box-shadow: 0 10px 40px rgba(0, 0, 0, 0.8);
  }
  .qr-modal-top {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    width: 100%;
  }
  .qr-modal-code {
    font-family: var(--font-mono, monospace);
    font-size: 24px;
    font-weight: 700;
    letter-spacing: 0.16em;
    color: var(--color-ember-accent, #dc5000);
  }
  .qr-modal-sub {
    font-size: 11px;
    color: var(--color-driftwood, #b8a794);
    margin: 2px 0 0 0;
  }
  .close-qr-btn {
    background: transparent;
    border: none;
    color: var(--color-driftwood, #b8a794);
    cursor: pointer;
    padding: 4px;
  }
  .close-qr-btn:hover {
    color: #ffffff;
  }
  .qr-modal-frame {
    padding: 14px;
    background: #ffffff;
    border-radius: 8px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
  }
  .qr-modal-hint {
    font-size: 11px;
    color: var(--color-driftwood, #b8a794);
    text-align: center;
    margin: 0;
  }

  @media (max-width: 768px) {
    .access-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
