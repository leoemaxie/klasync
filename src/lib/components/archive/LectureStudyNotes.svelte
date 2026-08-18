<script lang="ts">
  import { onMount } from 'svelte';
  import { Check, Copy, Download, Save } from '@lucide/svelte';

  let {
    sessionId,
    sessionTitle = 'Lecture',
  }: { sessionId: string; sessionTitle?: string } = $props();

  let notes = $state('');
  let isSaved = $state(true);
  let showCopied = $state(false);
  let saveTimeout: number | undefined;

  const storageKey = $derived(`klasync-notes-${sessionId}`);
  const wordCount = $derived(
    notes.trim() ? notes.trim().split(/\s+/).length : 0
  );

  onMount(() => {
    try {
      const saved = localStorage.getItem(storageKey);
      notes =
        saved ||
        `# Study Notes: ${sessionTitle}\n\n## Key Takeaways\n- \n\n## Action Items\n- `;
    } catch {}
  });

  function handleInput() {
    isSaved = false;
    if (saveTimeout) clearTimeout(saveTimeout);
    saveTimeout = window.setTimeout(() => {
      try {
        localStorage.setItem(storageKey, notes);
        isSaved = true;
      } catch {}
    }, 600);
  }

  function handleCopy() {
    navigator.clipboard.writeText(notes).then(() => {
      showCopied = true;
      setTimeout(() => (showCopied = false), 2000);
    });
  }

  function handleDownload() {
    const blob = new Blob([notes], { type: 'text/markdown' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${sessionTitle.replace(/[^a-zA-Z0-9_-]/g, '_')}_Notes.md`;
    a.click();
    URL.revokeObjectURL(url);
  }
</script>

<div class="study-notes-panel">
  <div class="notes-header">
    <div class="notes-title-group">
      <h2 class="section-title">Lecture Notes</h2>
      <span class="status-pill" class:saved={isSaved}>
        {#if isSaved}
          <Check size={12} />
          <span>Saved</span>
        {:else}
          <Save size={12} />
          <span>Saving...</span>
        {/if}
      </span>
    </div>

    <div class="notes-actions">
      <button
        type="button"
        class="outline notes-btn"
        onclick={handleCopy}
        title="Copy notes to clipboard"
      >
        {#if showCopied}
          <Check size={13} />
          <span>Copied</span>
        {:else}
          <Copy size={13} />
          <span>Copy</span>
        {/if}
      </button>
      <button
        type="button"
        class="outline notes-btn"
        onclick={handleDownload}
        title="Export notes as Markdown"
      >
        <Download size={13} />
        <span>Export</span>
      </button>
    </div>
  </div>

  <div class="editor-wrapper">
    <textarea
      class="notes-editor"
      bind:value={notes}
      oninput={handleInput}
      placeholder="Write lecture summary, study notes, or questions here..."
      aria-label="Study notes"
      rows="12"></textarea>
    <div class="editor-footer">
      <span class="word-counter"
        >{wordCount} {wordCount === 1 ? 'Word' : 'Words'}</span
      >
      <span class="format-hint">Markdown Supported</span>
    </div>
  </div>
</div>

<style>
  .study-notes-panel {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-10);
    width: 100%;
  }
  .notes-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--spacing-8);
  }
  .notes-title-group {
    display: flex;
    align-items: center;
    gap: var(--spacing-8);
  }
  .section-title {
    font-size: 15px;
    font-weight: 700;
    color: var(--color-warm-cream);
    margin: 0;
    letter-spacing: -0.01em;
  }
  .status-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    font-weight: 600;
    padding: 3px 8px;
    border-radius: 999px;
    background: rgba(220, 80, 0, 0.12);
    border: 1px solid rgba(220, 80, 0, 0.25);
    color: var(--color-ember-accent);
  }
  .status-pill.saved {
    background: rgba(74, 222, 128, 0.1);
    border-color: rgba(74, 222, 128, 0.2);
    color: #4ade80;
  }
  .notes-actions {
    display: flex;
    align-items: center;
    gap: var(--spacing-6);
  }
  .notes-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 0 12px;
    height: 32px;
    min-height: 32px;
    border-radius: var(--radius-controls, 4px);
    box-sizing: border-box;
  }
  .editor-wrapper {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }
  .notes-editor {
    width: 100%;
    min-height: 280px;
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards, 8px);
    color: var(--color-warm-cream);
    font-family: var(--font-mono, monospace);
    font-size: 13.5px;
    line-height: 1.65;
    padding: var(--spacing-14);
    resize: vertical;
    outline: none;
    box-sizing: border-box;
    transition:
      border-color 0.15s ease,
      background 0.15s ease;
  }
  .notes-editor::placeholder {
    color: rgba(255, 237, 215, 0.45);
  }
  .notes-editor:focus {
    border-color: var(--color-warm-cream);
    background: rgba(255, 237, 215, 0.02);
  }
  .editor-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 11px;
    color: var(--color-warm-cream-dim);
    font-family: var(--font-mono, monospace);
    padding: 0 var(--spacing-4);
  }
  .word-counter {
    font-weight: 600;
  }
  .format-hint {
    color: var(--color-warm-cream-dim);
  }
  @media (max-width: 520px) {
    .notes-header {
      flex-direction: column;
      align-items: stretch;
    }
    .notes-actions {
      width: 100%;
      justify-content: space-between;
    }
    .notes-actions button {
      flex: 1;
      justify-content: center;
    }
  }
</style>
