<script lang="ts">
  import { onMount } from 'svelte';
  import { Check, Copy, Download, Save } from '@lucide/svelte';

  let { sessionId, sessionTitle = 'Lecture' }: { sessionId: string; sessionTitle?: string } = $props();

  let notes = $state('');
  let isSaved = $state(true);
  let showCopied = $state(false);
  let saveTimeout: number | undefined;

  const storageKey = $derived(`klasync-notes-${sessionId}`);

  onMount(() => {
    try {
      const saved = localStorage.getItem(storageKey);
      notes = saved || `# Study Notes: ${sessionTitle}\n\n## Key Takeaways\n- \n\n## Action Items\n- `;
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
      <p class="eyebrow">STUDENT REVISION NOTES</p>
      <span class="status-pill" class:saved={isSaved}>
        {#if isSaved}<Check size={11} /> Saved{:else}<Save size={11} /> Saving...{/if}
      </span>
    </div>

    <div class="notes-actions">
      <button type="button" class="outline" onclick={handleCopy} title="Copy notes">
        {#if showCopied}<Check size={12} /> Copied!{:else}<Copy size={12} /> Copy{/if}
      </button>
      <button type="button" class="outline" onclick={handleDownload} title="Export markdown">
        <Download size={12} /> Export .md
      </button>
    </div>
  </div>

  <textarea
    class="notes-editor"
    bind:value={notes}
    oninput={handleInput}
    placeholder="Take lecture notes, formulas, or summaries..."
    aria-label="Student lecture notes"
    rows="10"
  ></textarea>
</div>

<style>
  .study-notes-panel { display: flex; flex-direction: column; gap: var(--spacing-8); padding: var(--spacing-12); background: rgba(16, 9, 4, 0.4); border: 1px solid var(--color-cork-border); border-radius: var(--radius-cards); }
  .notes-header { display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: var(--spacing-8); }
  .notes-title-wrap, .notes-actions { display: flex; align-items: center; gap: 6px; }
  .status-pill { display: inline-flex; align-items: center; gap: 3px; font-size: 9px; padding: 2px 6px; border-radius: 4px; background: rgba(220, 80, 0, 0.15); color: var(--color-ember-accent); }
  .status-pill.saved { background: rgba(74, 222, 128, 0.1); color: #4ade80; }
  .notes-actions button { display: inline-flex; align-items: center; gap: 4px; font-size: 10px; padding: 4px 8px; text-transform: uppercase; }
  .notes-editor { width: 100%; min-height: 200px; background: rgba(16, 9, 4, 0.6); border: 1px solid var(--color-cork-border); border-radius: 6px; color: var(--color-warm-cream); font-family: var(--font-mono, monospace); font-size: 13px; line-height: 1.6; padding: var(--spacing-10); resize: vertical; outline: none; }
  .notes-editor:focus { border-color: var(--color-warm-cream); }
  @media (max-width: 480px) {
    .notes-header { flex-direction: column; align-items: flex-start; }
    .notes-actions { width: 100%; justify-content: space-between; }
    .notes-actions button { flex: 1; justify-content: center; }
  }
</style>
