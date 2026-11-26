<script lang="ts">
  import {
    Check,
    Copy,
    Download,
    Eye,
    PenLine,
    Clock,
    LoaderCircle,
  } from '@lucide/svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Segmented from '$lib/components/ui/Segmented.svelte';
  import NotesMarkdownPreview from '$lib/components/archive/NotesMarkdownPreview.svelte';
  import { useStudio } from '$lib/studio/lectureStudio.svelte';
  import { clock } from '$lib/studio/time';

  const studio = useStudio();
  let mode = $state<'write' | 'preview'>('write');
  let editor: HTMLTextAreaElement | undefined = $state();
  let copied = $state(false);

  /** Inserts a [mm:ss] stamp for the current moment at the caret. */
  function stamp() {
    mode = 'write';
    const el = editor;
    const token = `[${clock(studio.time)}] `;
    if (!el) {
      studio.addNote('', studio.time);
      return;
    }
    const start = el.selectionStart ?? studio.notes.length;
    const end = el.selectionEnd ?? start;
    const before = studio.notes.slice(0, start);
    const needsBreak = before.length > 0 && !before.endsWith('\n');
    const insert = `${needsBreak ? '\n' : ''}- ${token}`;
    studio.setNotes(before + insert + studio.notes.slice(end));
    const caret = start + insert.length;
    queueMicrotask(() => {
      el.focus();
      el.setSelectionRange(caret, caret);
    });
  }

  // `N` anywhere in the workspace asks for a stamped note; the request waits
  // here until this pane is mounted, then is consumed once.
  $effect(() => {
    if (studio.pendingStamp) {
      studio.pendingStamp = false;
      stamp();
    }
  });

  async function copy() {
    try {
      await navigator.clipboard.writeText(studio.notes);
      copied = true;
      setTimeout(() => (copied = false), 1800);
    } catch {
      studio.announce('Copy is not available here');
    }
  }

  function download() {
    const blob = new Blob([studio.notes], { type: 'text/markdown' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${studio.title.replace(/[^a-zA-Z0-9_-]/g, '_')}_Notes.md`;
    a.click();
    URL.revokeObjectURL(url);
  }
</script>

<div class="notes">
  <div class="toolbar">
    <Segmented
      label="Notes view"
      bind:value={mode}
      options={[
        { value: 'write', label: 'Write', icon: PenLine },
        { value: 'preview', label: 'Preview', icon: Eye },
      ]}
    />

    <Button
      variant="soft"
      class="stamp-btn"
      onclick={stamp}
      title="Add a note at the current time (N)"
    >
      <Clock size={14} /><span>Stamp {clock(studio.time)}</span>
    </Button>

    <div class="end">
      <span
        class="status"
        class:saving={!studio.notesSaved}
        role="status"
        title={studio.notesSaved ? 'All changes saved' : 'Saving…'}
      >
        {#if studio.notesSaved}<Check size={14} /><span class="sr-only"
            >All changes saved</span
          >{:else}<LoaderCircle size={14} /><span class="sr-only">Saving…</span
          >{/if}
      </span>
      <Button icon aria-label="Copy notes" title="Copy notes" onclick={copy}>
        {#if copied}<Check size={15} />{:else}<Copy size={15} />{/if}
      </Button>
      <Button
        icon
        aria-label="Export notes as Markdown"
        title="Export as Markdown"
        onclick={download}
        disabled={!studio.notes.trim()}
      >
        <Download size={15} />
      </Button>
    </div>
  </div>

  <div class="canvas">
    {#if mode === 'write'}
      <textarea
        bind:this={editor}
        class="editor"
        value={studio.notes}
        oninput={(e) => studio.setNotes(e.currentTarget.value)}
        placeholder="Write notes here.&#10;&#10;Tip: select text in the transcript and choose “Add to notes”, or press N to stamp the current moment."
        aria-label="Lecture notes"
        spellcheck="true"></textarea>
    {:else}
      <NotesMarkdownPreview
        content={studio.notes}
        fill
        onSeek={(s) => studio.seek(s, { play: studio.hasAudio })}
      />
    {/if}
  </div>
</div>

<style>
  .toolbar :global(.stamp-btn) {
    padding: 0 12px;
    font-variant-numeric: tabular-nums;
  }
  .notes {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    gap: 10px;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .end {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-left: auto;
  }
  .status {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    margin-right: 2px;
    color: var(--text-tertiary);
  }
  .status.saving :global(svg) {
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .canvas {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .editor {
    flex: 1;
    width: 100%;
    min-height: 0;
    resize: none;
    padding: 16px 18px;
    background: var(--surface-inset);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    color: var(--text-primary);
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 14px;
    line-height: 1.75;
    letter-spacing: 0;
    text-transform: none;
    box-sizing: border-box;
    transition: border-color var(--motion-fast) var(--ease-out);
  }
  .editor::placeholder {
    color: var(--text-tertiary);
    font-style: normal;
    font-family: var(--font-body);
    line-height: 1.6;
  }
  .editor:focus {
    outline: none;
    border-color: var(--border-strong);
    box-shadow: var(--focus-ring);
  }
  @media (prefers-reduced-motion: reduce) {
    .status.saving :global(svg) {
      animation-duration: 1.6s !important;
      animation-iteration-count: infinite !important;
    }
  }
</style>
