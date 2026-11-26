<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import {
    Download,
    FileText,
    RefreshCw,
    Send,
    Sparkles,
  } from '@lucide/svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import EmptyState from '$lib/components/ui/EmptyState.svelte';
  import Spinner from '$lib/components/ui/Spinner.svelte';
  import MaterialEditor from './MaterialEditor.svelte';
  import SlidesPanel from './SlidesPanel.svelte';
  import {
    downloadMaterial,
    fetchMaterial,
    fetchMaterialCapabilities,
    fetchPluginSettings,
    fetchMaterialTranscript,
    generateMaterial,
    publishMaterial,
    unpublishMaterial,
    updateMaterial,
    type ExportFormat,
    type MaterialCapabilities,
    type MaterialResponse,
  } from '$lib/api/materials';
  import { exportFilename, saveBlob } from '$lib/materials/download';

  let {
    courseId,
    sessionCode,
    sessionId,
    title,
    onChanged,
  }: {
    courseId: string;
    sessionCode: string;
    sessionId: string;
    title: string;
    /** Status changed (publish, generate…): the parent refreshes its list. */
    onChanged?: () => void;
  } = $props();

  let data = $state<MaterialResponse | null>(null);
  let caps = $state<MaterialCapabilities | null>(null);
  let loading = $state(true);
  let error = $state('');
  let notice = $state('');
  let busy = $state('');
  let transcript = $state<{ offset_sec: number | null; text: string }[]>([]);
  let editor = $state<MaterialEditor>();
  let editorKey = $state(0);
  let sendEmail = $state(false);
  let pushLms = $state(false);
  let lmsConfigured = $state(false);
  let message = $state('');
  let showPublish = $state(false);

  const material = $derived(data?.material ?? null);
  const working = $derived(
    material?.generation_state === 'queued' ||
      material?.generation_state === 'generating'
  );

  let poll: ReturnType<typeof setTimeout> | undefined;

  async function load(opts: { remount?: boolean } = {}) {
    try {
      data = await fetchMaterial(sessionCode);
      error = '';
      if (opts.remount) editorKey++;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not load the material';
    } finally {
      loading = false;
    }
    schedulePoll();
  }

  function schedulePoll() {
    clearTimeout(poll);
    if (working) {
      poll = setTimeout(async () => {
        const wasWorking = true;
        await load({ remount: false });
        // A generation finished: show the fresh draft in the editor.
        if (wasWorking && !working) editorKey++;
      }, 3000);
    }
  }

  onMount(() => {
    void load();
    fetchMaterialCapabilities()
      .then((c) => (caps = c))
      .catch(() => {});
    fetchPluginSettings(courseId)
      .then((r) => {
        const w = r.plugins.find(
          (p) => p.kind === 'delivery' && p.id === 'webhook'
        );
        lmsConfigured = !!w?.enabled && !!w.config.url;
      })
      .catch(() => {});
    fetchMaterialTranscript(sessionCode)
      .then((t) => (transcript = t.lines))
      .catch(() => {});
  });
  onDestroy(() => clearTimeout(poll));

  async function run(label: string, fn: () => Promise<unknown>, ok = '') {
    busy = label;
    error = '';
    notice = '';
    try {
      await fn();
      if (ok) notice = ok;
      await load({ remount: true });
      onChanged?.();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Something went wrong';
    } finally {
      busy = '';
    }
  }

  const generate = () => {
    if (
      material?.draft_doc &&
      !confirm(
        'Regenerate this material? Blocks you locked or edited are kept; the rest is rewritten.'
      )
    )
      return;
    return run('generate', () => generateMaterial(sessionCode));
  };

  const toReview = (status: 'draft' | 'in_review') =>
    run('status', () => updateMaterial(sessionCode, { status }));

  async function publish() {
    if (!(await editor?.flush())) {
      error = 'Fix the save problem before publishing.';
      return;
    }
    if (editor && !editor.isValid()) {
      error = 'Fix the highlighted problems before publishing.';
      return;
    }
    await run(
      'publish',
      () =>
        publishMaterial(sessionCode, {
          deliver: [
            ...(sendEmail ? ['email'] : []),
            ...(pushLms ? ['webhook'] : []),
          ],
          message: message.trim() || undefined,
        }),
      sendEmail || pushLms
        ? `Published. ${[sendEmail && 'Email is being sent', pushLms && 'the LMS is being notified'].filter(Boolean).join(' and ')}.`
        : 'Published. Students can read it now.'
    );
    showPublish = false;
  }

  const unpublish = (archive: boolean) =>
    run(
      'unpublish',
      () => unpublishMaterial(sessionCode, archive),
      archive ? 'Archived.' : 'Unpublished.'
    );

  async function exportAs(format: ExportFormat, version?: string) {
    busy = `export-${format}`;
    error = '';
    try {
      if (!version) await editor?.flush();
      const blob = await downloadMaterial(
        sessionCode,
        format,
        version ?? 'draft'
      );
      saveBlob(
        blob,
        exportFilename(material?.draft_doc?.title ?? title, format)
      );
    } catch (e) {
      error = e instanceof Error ? e.message : 'Export failed';
    } finally {
      busy = '';
    }
  }

  const formats = $derived(caps?.formats.filter((f) => f.available) ?? []);
  const emailChannel = $derived(
    caps?.delivery_channels.find((c) => c.id === 'email')
  );
  const statusLabel: Record<string, string> = {
    draft: 'Draft',
    in_review: 'In review',
    published: 'Published',
    archived: 'Archived',
  };
</script>

<div class="mat-panel">
  {#if loading}
    <div class="center"><Spinner /></div>
  {:else if !material}
    <EmptyState
      icon={Sparkles}
      title="No handout yet"
      description="Klasync drafts a structured handout from the lecture recording. You review, edit and publish it."
    >
      {#if error}<p class="err" role="alert">{error}</p>{/if}
      <SlidesPanel
        {sessionCode}
        info={data?.slides ?? null}
        onChanged={() => void load()}
      />
      <Button
        variant="accent"
        disabled={!!busy}
        onclick={() =>
          void run('generate', () => generateMaterial(sessionCode))}
      >
        {busy ? 'Starting…' : 'Generate handout'}
      </Button>
    </EmptyState>
  {:else}
    <header class="head">
      <div>
        <h3>{material.draft_doc?.title ?? title}</h3>
        <p class="meta">
          <span class="pill" data-status={material.status}
            >{statusLabel[material.status]}</span
          >
          {#if data?.versions?.length}· v{data.versions[0].version_no}{/if}
        </p>
      </div>
      <div class="mat-actions">
        <Button
          size="sm"
          variant="outline"
          disabled={working || !!busy}
          onclick={() => void generate()}
        >
          <RefreshCw size={14} /> Regenerate
        </Button>
        {#if material.status === 'draft'}
          <Button
            size="sm"
            variant="outline"
            disabled={!material.draft_doc || !!busy}
            onclick={() => void toReview('in_review')}
            >Mark ready for review</Button
          >
        {:else if material.status === 'in_review'}
          <Button
            size="sm"
            variant="outline"
            disabled={!!busy}
            onclick={() => void toReview('draft')}>Back to draft</Button
          >
        {/if}
        {#if material.status === 'published'}
          <Button
            size="sm"
            variant="outline"
            disabled={!!busy}
            onclick={() => void unpublish(false)}>Unpublish</Button
          >
          <Button
            size="sm"
            variant="outline"
            disabled={!!busy}
            onclick={() => void unpublish(true)}>Archive</Button
          >
        {/if}
        <Button
          size="sm"
          variant="accent"
          disabled={!material.draft_doc || working || !!busy}
          onclick={() => (showPublish = !showPublish)}
        >
          <Send size={14} />
          {material.status === 'published' ? 'Publish update' : 'Publish'}
        </Button>
      </div>
    </header>

    {#if showPublish}
      <div class="publish" role="group" aria-label="Publish options">
        <p class="muted">
          Students in the course can read this on Klasync and export it.
          {#if material.status === 'published'}This publishes a new version.{/if}
        </p>
        {#if emailChannel}
          <label class="check">
            <input type="checkbox" bind:checked={sendEmail} />
            Also email students a link
          </label>
          {#if sendEmail}
            <label class="field">
              <span>Message (optional)</span>
              <textarea rows="2" bind:value={message} maxlength="1000"
              ></textarea>
            </label>
          {/if}
        {/if}
        {#if lmsConfigured}
          <label class="check">
            <input type="checkbox" bind:checked={pushLms} />
            Also push to the LMS (webhook)
          </label>
        {/if}
        <div class="row">
          <Button
            size="sm"
            variant="accent"
            disabled={!!busy}
            onclick={() => void publish()}
          >
            {busy === 'publish' ? 'Publishing…' : 'Confirm publish'}
          </Button>
          <Button size="sm" onclick={() => (showPublish = false)}>Cancel</Button
          >
        </div>
      </div>
    {/if}

    {#if error}<p class="err" role="alert">{error}</p>{/if}
    {#if notice}<p class="ok" role="status">{notice}</p>{/if}

    {#if working}
      <div class="working" role="status">
        <Spinner />
        <span>
          {material.generation_state === 'queued'
            ? 'Waiting for the lecture to finish processing…'
            : 'Drafting the handout…'}
          This page updates itself.
        </span>
      </div>
    {:else if material.generation_state === 'failed'}
      <p class="err" role="alert">
        Generation failed{material.generation_error
          ? `: ${material.generation_error}`
          : ''}. Try regenerating.
      </p>
    {/if}

    <SlidesPanel
      {sessionCode}
      info={data?.slides ?? null}
      onChanged={() => void load()}
    />

    {#if formats.length && material.draft_doc}
      <div class="exports">
        <span class="muted">Export draft:</span>
        {#each formats as f}
          <Button
            size="sm"
            variant="outline"
            disabled={!!busy}
            onclick={() => void exportAs(f.id as ExportFormat)}
          >
            <Download size={13} />
            {f.label}
          </Button>
        {/each}
      </div>
    {/if}

    {#if material.draft_doc}
      {#key editorKey}
        <MaterialEditor
          bind:this={editor}
          {sessionCode}
          initialDoc={material.draft_doc}
          initialRev={material.draft_rev}
          {transcript}
          onRegenerating={() => void load()}
          onConflict={() =>
            (error =
              'This draft changed elsewhere. Reload the page to continue editing.')}
        />
      {/key}
    {:else if !working}
      <EmptyState
        icon={FileText}
        title="Nothing drafted yet"
        description="Generate the handout to start."
      />
    {/if}

    {#if data?.versions?.length}
      <section class="versions">
        <h4>Versions</h4>
        <ul>
          {#each data.versions as v}
            <li>
              <span
                >v{v.version_no} · {v.origin === 'ai'
                  ? 'AI draft'
                  : 'Your edit'}</span
              >
              <span class="muted"
                >{new Date(v.created_at).toLocaleString()}</span
              >
              {#if v.published_at}<span class="pill" data-status="published"
                  >published</span
                >{/if}
              {#each formats.filter((f) => ['pdf', 'docx'].includes(f.id)) as f}
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={!!busy}
                  onclick={() => void exportAs(f.id as ExportFormat, v.id)}
                  >{f.label}</Button
                >
              {/each}
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if data?.deliveries?.length}
      <section class="versions">
        <h4>Deliveries</h4>
        <ul>
          {#each data.deliveries as d}
            <li>
              <span>{d.channel}</span>
              <span class="pill" data-status={d.status}>{d.status}</span>
              <span class="muted">{d.delivered}/{d.recipients} delivered</span>
              {#if d.error}<span class="err-inline">{d.error}</span>{/if}
            </li>
          {/each}
        </ul>
      </section>
    {/if}
  {/if}
</div>

<style>
  .mat-panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .center {
    display: grid;
    place-items: center;
    padding: 40px;
  }
  .head {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
    align-items: flex-start;
  }
  h3 {
    margin: 0;
    color: var(--text-primary);
    font-size: 18px;
  }
  h4 {
    margin: 0 0 6px;
    font-size: 12px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-tertiary);
  }
  .meta {
    margin: 2px 0 0;
    font-size: 12.5px;
    color: var(--text-tertiary);
  }
  .mat-actions,
  .exports,
  .row {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    align-items: center;
  }
  .pill {
    display: inline-block;
    padding: 0 8px;
    border: 1px solid var(--border-default);
    border-radius: 999px;
    font-size: 11.5px;
    color: var(--text-secondary);
  }
  .pill[data-status='published'],
  .pill[data-status='sent'] {
    border-color: #4c9a68;
    color: #7fd3a0;
  }
  .pill[data-status='failed'] {
    border-color: #ff9b7a;
    color: #ff9b7a;
  }
  .publish {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    border: 1px solid var(--border-accent);
    border-radius: 12px;
    background: var(--surface-1);
  }
  .check {
    display: flex;
    gap: 8px;
    align-items: center;
    font-size: 14px;
    color: var(--text-secondary);
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
  }
  .muted {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-tertiary);
  }
  .err,
  .ok {
    margin: 0;
    padding: 8px 12px;
    border-radius: 8px;
    font-size: 13px;
    border: 1px solid #ff9b7a;
    color: #ff9b7a;
  }
  .ok {
    border-color: #4c9a68;
    color: #7fd3a0;
  }
  .err-inline {
    color: #ff9b7a;
    font-size: 12.5px;
  }
  .working {
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 10px 12px;
    border: 1px solid var(--border-default);
    border-radius: 10px;
    font-size: 13.5px;
    color: var(--text-secondary);
  }
  .versions ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .versions li {
    display: flex;
    gap: 10px;
    align-items: center;
    flex-wrap: wrap;
    font-size: 13.5px;
    color: var(--text-secondary);
  }
  /* The global label/input styles are for the sign-in forms; undo them here. */
  .check {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 8px;
    margin: 0;
    font-size: 14px;
    font-weight: 400;
    letter-spacing: 0;
    text-transform: none;
    color: var(--text-secondary);
  }
  .check input[type='checkbox'] {
    width: auto;
    margin: 0;
    flex: none;
  }
</style>
