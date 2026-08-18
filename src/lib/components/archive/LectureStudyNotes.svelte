<script lang="ts">
  import { onMount } from 'svelte';
  import { Check, Copy, Download, Save, Eye, Edit3 } from '@lucide/svelte';
  import NotesMarkdownPreview from './NotesMarkdownPreview.svelte';

  let { sessionId, sessionTitle = 'Lecture' }: { sessionId: string; sessionTitle?: string } = $props();

  let notes = $state('');
  let isSaved = $state(true);
  let showCopied = $state(false);
  let activeMode = $state<'edit' | 'preview'>('edit');
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

<div class="study-notes-panel">
  <div class="notes-header">
    <div class="notes-title-group">
      <h2 class="section-title">Lecture Notes</h2>
      <span class="status-pill" class:saved={isSaved}>
        {#if isSaved}<Check size={11} /> Saved{:else}<Save size={11} /> Saving...{/if}
      </span>
    </div>

    <div class="notes-actions">
      <div class="mode-toggle" role="group" aria-label="Editor view mode">
        <button type="button" class="mode-btn" class:active={activeMode === 'edit'} onclick={() => (activeMode = 'edit')}>
          <Edit3 size={11} /> Edit
        </button>
        <button type="button" class="mode-btn" class:active={activeMode === 'preview'} onclick={() => (activeMode = 'preview')}>
          <Eye size={11} /> Preview
        </button>
      </div>

      <button type="button" class="outline notes-btn" onclick={handleCopy} title="Copy notes">
        {#if showCopied}<Check size={11} /> Copied{:else}<Copy size={11} /> Copy{/if}
      </button>
      <button type="button" class="outline notes-btn" onclick={handleDownload} title="Export markdown notes">
        <Download size={11} /> Export
      </button>
    </div>
  </div>

  {#if activeMode === 'edit'}
    <div class="editor-wrapper">
      <textarea class="notes-editor" bind:value={notes} oninput={handleInput} placeholder="Write markdown notes..." aria-label="Study notes" rows="12"></textarea>
    </div>
  {:else}
    <NotesMarkdownPreview content={notes} />
  {/if}
</div>

<style>
  .study-notes-panel { display: flex; flex-direction: column; gap: var(--spacing-10); width: 100%; }
  .notes-header { display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: var(--spacing-8); }
  .notes-title-group, .notes-actions { display: flex; align-items: center; gap: 6px; }
  .section-title { font-size: 15px; font-weight: 700; color: var(--color-warm-cream); margin: 0; }
  .status-pill { display: inline-flex; align-items: center; gap: 3px; font-size: 10px; padding: 2px 6px; border-radius: 4px; background: rgba(220, 80, 0, 0.15); color: var(--color-ember-accent); }
  .status-pill.saved { background: rgba(74, 222, 128, 0.1); color: #4ade80; }
  .mode-toggle { display: inline-flex; background: rgba(16, 9, 4, 0.6); border: 1px solid var(--color-cork-border); border-radius: 4px; padding: 2px; gap: 2px; }
  .mode-btn { display: inline-flex; align-items: center; gap: 4px; font-size: 10px; padding: 3px 8px; border: none; background: transparent; color: var(--color-driftwood); border-radius: 2px; cursor: pointer; text-transform: uppercase; font-weight: 600; }
  .mode-btn.active { background: var(--color-ember-accent); color: var(--color-warm-cream); font-weight: 700; }
  .notes-btn { display: inline-flex; align-items: center; gap: 4px; font-size: 10px; padding: 4px 8px; text-transform: uppercase; min-height: 28px; height: 28px; }
  .editor-wrapper { display: flex; flex-direction: column; gap: 6px; }
  .notes-editor { width: 100%; min-height: 280px; background: rgba(16, 9, 4, 0.6); border: 1px solid var(--color-cork-border); border-radius: var(--radius-cards, 8px); color: var(--color-warm-cream); font-family: var(--font-mono, monospace); font-size: 13.5px; line-height: 1.65; padding: var(--spacing-14); resize: vertical; outline: none; box-sizing: border-box; }
  .notes-editor:focus { border-color: var(--color-warm-cream); }
  @media (max-width: 540px) { .notes-header { flex-direction: column; align-items: stretch; gap: 8px; } .notes-actions { justify-content: space-between; width: 100%; } }
</style>
