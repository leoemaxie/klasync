<script lang="ts">
  import { onMount } from 'svelte';
  import { FileText, Save, Check, Copy, Download } from '@lucide/svelte';

  let { sessionId, sessionTitle = 'Lecture' }: { sessionId: string; sessionTitle?: string } = $props();

  let notes = $state('');
  let isSaved = $state(true);
  let showCopied = $state(false);
  let saveTimeout: number | undefined;

  const storageKey = $derived(`klasync-notes-${sessionId}`);

  onMount(() => {
    try {
      const saved = localStorage.getItem(storageKey);
      if (saved) {
        notes = saved;
      } else {
        notes = `# Study Notes: ${sessionTitle}\n\n## Key Takeaways\n- \n\n## Action Items & Formulas\n- `;
      }
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

<div class="panel study-notes-panel">
  <div class="notes-header">
    <div class="notes-title-wrap">
      <p class="eyebrow">STUDENT REVISION & ACTIVE RECALL NOTES</p>
      <div class="save-status-indicator">
        {#if isSaved}
          <span class="status-pill saved"><Check size={12} /> Auto-saved</span>
        {:else}
          <span class="status-pill saving"><Save size={12} /> Saving...</span>
        {/if}
      </div>
    </div>

    <div class="notes-actions">
      <button type="button" class="outline" onclick={handleCopy} title="Copy notes to clipboard">
        {#if showCopied}
          <Check size={13} color="var(--color-warm-cream)" /> Copied!
        {:else}
          <Copy size={13} /> Copy Markdown
        {/if}
      </button>
      <button type="button" class="outline" onclick={handleDownload} title="Download notes as markdown file">
        <Download size={13} /> Export .md
      </button>
    </div>
  </div>

  <textarea
    class="notes-editor"
    bind:value={notes}
    oninput={handleInput}
    placeholder="Take lecture notes, formulate questions, or write summaries..."
    aria-label="Student lecture notes"
    rows="14"
  ></textarea>

  <div class="notes-footer">
    <span class="hint">Supports standard Markdown formatting. Automatically stored in your local student workspace cache.</span>
  </div>
</div>

<style>
  .study-notes-panel {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-14);
    padding: var(--spacing-18);
    background: rgba(16, 9, 4, 0.4);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards);
  }
  .notes-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--spacing-12);
  }
  .notes-title-wrap {
    display: flex;
    align-items: center;
    gap: var(--spacing-12);
  }
  .save-status-indicator {
    display: flex;
    align-items: center;
  }
  .status-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 10px;
    padding: 2px 8px;
    border-radius: 4px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .status-pill.saved {
    background: rgba(74, 222, 128, 0.1);
    color: #4ade80;
    border: 1px solid rgba(74, 222, 128, 0.2);
  }
  .status-pill.saving {
    background: rgba(220, 80, 0, 0.15);
    color: var(--color-ember-accent);
    border: 1px solid rgba(220, 80, 0, 0.3);
  }
  .notes-actions {
    display: flex;
    gap: var(--spacing-8);
  }
  .notes-actions button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    padding: 5px 10px;
    text-transform: uppercase;
  }
  .notes-editor {
    width: 100%;
    min-height: 280px;
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: 6px;
    color: var(--color-warm-cream);
    font-family: var(--font-mono, monospace);
    font-size: 13px;
    line-height: 1.6;
    padding: var(--spacing-14);
    resize: vertical;
    outline: none;
    box-sizing: border-box;
  }
  .notes-editor:focus {
    border-color: var(--color-warm-cream);
  }
  .notes-footer {
    display: flex;
    justify-content: flex-end;
  }
  .notes-footer .hint {
    font-size: 11px;
    color: var(--color-driftwood);
  }
</style>
