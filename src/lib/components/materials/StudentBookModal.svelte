<script lang="ts">
  import { onMount } from 'svelte';
  import { BookOpenText, Download, X } from '@lucide/svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import EmptyState from '$lib/components/ui/EmptyState.svelte';
  import Spinner from '$lib/components/ui/Spinner.svelte';
  import MaterialReader from './MaterialReader.svelte';
  import {
    downloadStudentBook,
    fetchStudentBook,
    type ExportFormat,
  } from '$lib/api/materials';
  import { exportFilename, saveBlob } from '$lib/materials/download';

  let {
    courseId,
    courseTitle,
    onClose,
  }: { courseId: string; courseTitle: string; onClose: () => void } = $props();

  type Book = Awaited<ReturnType<typeof fetchStudentBook>>;
  let book = $state<Book | null>(null);
  let status = $state<'loading' | 'ready' | 'none' | 'error'>('loading');
  let error = $state('');
  let exporting = $state('');
  let dialog = $state<HTMLDivElement>();

  async function load() {
    status = 'loading';
    try {
      book = await fetchStudentBook(courseId);
      status = 'ready';
    } catch (e) {
      const msg = e instanceof Error ? e.message : '';
      if (
        /not found|no published|not published|not available|no book/i.test(msg)
      ) {
        status = 'none';
      } else {
        error = msg || 'Unable to load the course book.';
        status = 'error';
      }
    }
  }

  async function save(format: ExportFormat) {
    exporting = format;
    try {
      saveBlob(
        await downloadStudentBook(courseId, format),
        exportFilename(book?.title ?? courseTitle, format)
      );
    } catch (e) {
      error = e instanceof Error ? e.message : 'Export failed';
    } finally {
      exporting = '';
    }
  }

  onMount(() => {
    void load();
    const previous = document.activeElement as HTMLElement | null;
    dialog?.focus();
    return () => previous?.focus?.();
  });

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose();
  }
</script>

<svelte:window onkeydown={onKey} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="scrim" onclick={(e) => e.target === e.currentTarget && onClose()}>
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-label={`Course book: ${courseTitle}`}
    tabindex="-1"
    bind:this={dialog}
  >
    <header>
      <div>
        <p class="mat-eyebrow">Course book</p>
        <h2>{book?.title ?? courseTitle}</h2>
      </div>
      <div class="mat-actions">
        {#if book}
          {#each book.formats as f}
            <Button
              size="sm"
              variant="outline"
              disabled={!!exporting}
              onclick={() => void save(f.id as ExportFormat)}
            >
              <Download size={13} />
              {f.label}
            </Button>
          {/each}
        {/if}
        <Button icon size="sm" aria-label="Close" onclick={onClose}>
          <X size={16} />
        </Button>
      </div>
    </header>
    {#if error && status !== 'error'}<p class="err" role="alert">
        {error}
      </p>{/if}
    <div class="body">
      {#if status === 'loading'}
        <div class="center"><Spinner /></div>
      {:else if status === 'none'}
        <EmptyState
          icon={BookOpenText}
          title="No course book yet"
          description="Your lecturer hasn’t published a course book for this course."
        />
      {:else if status === 'error'}
        <EmptyState
          tone="error"
          icon={BookOpenText}
          title="Couldn’t load the course book"
          description={error}
        >
          <Button variant="outline" onclick={() => void load()}>
            Try again
          </Button>
        </EmptyState>
      {:else if book}
        <MaterialReader doc={book.doc} />
      {/if}
    </div>
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 200;
    display: grid;
    place-items: center;
    padding: 16px;
    background: rgba(8, 4, 2, 0.7);
  }
  .dialog {
    display: flex;
    flex-direction: column;
    width: min(900px, 100%);
    max-height: 92dvh;
    border: 1px solid var(--border-default);
    border-radius: 16px;
    background: var(--surface-page);
    box-shadow: var(--elev-3);
    outline: none;
  }
  header {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
    align-items: flex-start;
    padding: 14px 18px;
    border-bottom: 1px solid var(--border-subtle);
  }
  .mat-eyebrow {
    margin: 0;
    font-size: 11px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--text-accent);
  }
  h2 {
    margin: 2px 0 0;
    font-size: 20px;
    color: var(--text-primary);
  }
  .mat-actions {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    align-items: center;
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 18px;
  }
  .center {
    display: grid;
    place-items: center;
    padding: 40px;
  }
  .err {
    margin: 8px 18px 0;
    color: #ff9b7a;
    font-size: 13px;
  }
</style>
