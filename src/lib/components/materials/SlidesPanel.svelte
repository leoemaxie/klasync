<script lang="ts">
  import { Presentation } from '@lucide/svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import {
    saveMaterialSlidesText,
    uploadMaterialSlides,
    type SlidesInfo,
  } from '$lib/api/materials';

  let {
    sessionCode,
    info = null,
    onChanged,
  }: {
    sessionCode: string;
    info?: SlidesInfo | null;
    /** Slides were added or cleared (the parent refreshes). */
    onChanged?: () => void;
  } = $props();

  let pasted = $state('');
  let busy = $state(false);
  let error = $state('');
  let notice = $state('');

  async function run(fn: () => Promise<SlidesInfo>, ok: string) {
    busy = true;
    error = '';
    notice = '';
    try {
      const res = await fn();
      notice = res.has_slides
        ? `${ok} (${res.chars.toLocaleString()} characters)`
        : 'Slides removed.';
      pasted = '';
      onChanged?.();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not save the slides';
    } finally {
      busy = false;
    }
  }

  async function onFile(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = '';
    if (!file) return;
    if (/\.pptx$/i.test(file.name)) {
      await run(() => uploadMaterialSlides(sessionCode, file), 'Slides read');
    } else if (/\.(txt|md|markdown)$/i.test(file.name)) {
      const text = await file.text();
      await run(
        () => saveMaterialSlidesText(sessionCode, text, file.name),
        'Notes saved'
      );
    } else {
      error = 'Use a .pptx presentation or a .txt / .md file.';
    }
  }
</script>

<details class="slides">
  <summary>
    <Presentation size={14} />
    Slides &amp; notes
    {#if info?.has_slides}
      <span class="pill"
        >{info.name ?? 'added'} · {info.chars.toLocaleString()} chars</span
      >
    {/if}
  </summary>
  <div class="body">
    <p class="muted">
      Add your slides so the handout uses your terminology and order. They are
      context only: the recording stays the source of what was taught. PDF and
      the old .ppt format are not supported; export to .pptx or paste the text.
      Applies the next time the handout is generated.
    </p>
    <div class="row">
      <label class="file">
        <input
          type="file"
          accept=".pptx,.txt,.md,.markdown"
          onchange={onFile}
          disabled={busy}
        />
      </label>
      {#if info?.has_slides}
        <Button
          size="sm"
          variant="outline"
          disabled={busy}
          onclick={() =>
            void run(() => saveMaterialSlidesText(sessionCode, ''), '')}
        >
          Remove slides
        </Button>
      {/if}
    </div>
    <label class="field">
      <span>Or paste slide text</span>
      <textarea rows="4" bind:value={pasted} placeholder="Slide 1: …"
      ></textarea>
    </label>
    <Button
      size="sm"
      variant="outline"
      disabled={busy || !pasted.trim()}
      onclick={() =>
        void run(
          () => saveMaterialSlidesText(sessionCode, pasted, 'Pasted text'),
          'Notes saved'
        )}
    >
      Save pasted text
    </Button>
    {#if error}<p class="err" role="alert">{error}</p>{/if}
    {#if notice}<p class="ok" role="status">{notice}</p>{/if}
  </div>
</details>

<style>
  .slides {
    border: 1px solid var(--border-subtle);
    border-radius: 12px;
    background: var(--surface-1);
    padding: 8px 12px;
  }
  summary {
    display: flex;
    gap: 8px;
    align-items: center;
    cursor: pointer;
    font-size: 13.5px;
    color: var(--text-secondary);
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-top: 8px;
  }
  .row {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .field span {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-tertiary);
  }
  textarea {
    padding: 7px 9px;
    border: 1px solid var(--border-default);
    border-radius: 8px;
    background: var(--surface-inset);
    color: var(--text-primary);
    font: inherit;
    font-size: 14px;
  }
  input[type='file'] {
    width: auto;
    margin: 0;
    font-size: 13px;
    letter-spacing: 0;
    color: var(--text-secondary);
  }
  .body :global(.ui-btn) {
    align-self: flex-start;
  }
  .pill {
    padding: 0 8px;
    border: 1px solid var(--border-default);
    border-radius: 999px;
    font-size: 11.5px;
  }
  .muted {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-tertiary);
  }
  .err,
  .ok {
    margin: 0;
    font-size: 13px;
    color: #ff9b7a;
  }
  .ok {
    color: #7fd3a0;
  }
</style>
