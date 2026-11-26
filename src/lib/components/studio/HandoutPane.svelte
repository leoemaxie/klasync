<script lang="ts">
  import { onMount } from 'svelte';
  import { Download, FileText } from '@lucide/svelte';
  import SkeletonCard from '$lib/components/shared/SkeletonCard.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import EmptyState from '$lib/components/ui/EmptyState.svelte';
  import MaterialReader from '$lib/components/materials/MaterialReader.svelte';
  import {
    downloadStudentMaterial,
    fetchStudentMaterial,
    type ExportFormat,
    type PublishedMaterial,
  } from '$lib/api/materials';
  import { resolveSessionUuid } from '$lib/api/sessions';
  import { exportFilename, saveBlob } from '$lib/materials/download';
  import { useStudio } from '$lib/studio/lectureStudio.svelte';

  const studio = useStudio();

  let material = $state<PublishedMaterial | null>(null);
  let status = $state<'loading' | 'ready' | 'none' | 'error'>('loading');
  let error = $state('');
  let exporting = $state('');
  let sessionId = '';

  async function load() {
    status = 'loading';
    error = '';
    try {
      const id = await resolveSessionUuid(studio.sessionKey);
      if (!id) throw new Error('Unknown lecture');
      sessionId = id;
      material = await fetchStudentMaterial(id);
      status = 'ready';
    } catch (e) {
      const msg = e instanceof Error ? e.message : '';
      // The API answers 404 until the lecturer publishes.
      if (/not found|no published|not published|not available/i.test(msg)) {
        status = 'none';
      } else {
        error = msg || 'Unable to load the handout.';
        status = 'error';
      }
    }
  }
  onMount(() => void load());

  async function save(format: ExportFormat) {
    exporting = format;
    try {
      const blob = await downloadStudentMaterial(sessionId, format);
      saveBlob(blob, exportFilename(material?.doc.title ?? 'handout', format));
      studio.announce(`Downloaded ${format.toUpperCase()}`);
    } catch (e) {
      studio.announce(e instanceof Error ? e.message : 'Export failed');
    } finally {
      exporting = '';
    }
  }
</script>

<div class="handout">
  {#if status === 'loading'}
    <SkeletonCard lines={5} label="Loading handout…" />
  {:else if status === 'error'}
    <EmptyState
      tone="error"
      icon={FileText}
      title="Couldn’t load the handout"
      description={error}
    >
      <Button variant="outline" onclick={() => void load()}>Try again</Button>
    </EmptyState>
  {:else if status === 'none' || !material}
    <EmptyState
      icon={FileText}
      title="No handout yet"
      description="Your lecturer hasn’t published a handout for this lecture. It will appear here when they do."
    />
  {:else}
    <div class="bar">
      <span class="meta"
        >Version {material.version_no}{#if material.published_at}
          · {new Date(material.published_at).toLocaleDateString()}{/if}</span
      >
      <div class="exports">
        {#each material.formats as f}
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
      </div>
    </div>
    <div class="body"><MaterialReader doc={material.doc} /></div>
  {/if}
</div>

<style>
  .handout {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  .bar {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border-subtle);
  }
  .meta {
    font-size: 12.5px;
    color: var(--text-tertiary);
  }
  .exports {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 16px;
  }
</style>
