<script lang="ts">
  import { onMount } from 'svelte';
  import { FileText } from '@lucide/svelte';
  import Segmented from '$lib/components/ui/Segmented.svelte';
  import EmptyState from '$lib/components/ui/EmptyState.svelte';
  import Spinner from '$lib/components/ui/Spinner.svelte';
  import LectureMaterialPanel from './LectureMaterialPanel.svelte';
  import BookBuilder from './BookBuilder.svelte';
  import CompareView from './CompareView.svelte';
  import MaterialSettingsPanel from './MaterialSettingsPanel.svelte';
  import { fetchCourseMaterials } from '$lib/api/materials';

  type Row = Awaited<
    ReturnType<typeof fetchCourseMaterials>
  >['materials'][number];
  type View = 'lectures' | 'book' | 'compare' | 'settings';

  let {
    courseId,
    courseCode,
    currentSession = null,
  }: {
    courseId: string;
    courseCode: string;
    /** The lecture currently open in the Room tab, so it can be drafted even before it has a material. */
    currentSession?: { id: string; code: string; title: string } | null;
  } = $props();

  let view = $state<View>('lectures');
  let rows = $state<Row[]>([]);
  let loading = $state(true);
  let error = $state('');
  let selected = $state<{ code: string; id: string; title: string } | null>(
    null
  );

  async function load() {
    loading = true;
    try {
      rows = (await fetchCourseMaterials(courseId)).materials;
      error = '';
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not load materials';
    } finally {
      loading = false;
    }
  }

  // Reload when the course changes.
  let loadedFor = '';
  $effect(() => {
    if (courseId && courseId !== loadedFor) {
      loadedFor = courseId;
      selected = null;
      void load();
    }
  });

  onMount(() => {
    if (courseId && courseId !== loadedFor) {
      loadedFor = courseId;
      void load();
    }
  });

  const extra = $derived(
    currentSession && !rows.some((r) => r.short_code === currentSession.code)
      ? currentSession
      : null
  );

  const statusLabel: Record<string, string> = {
    draft: 'Draft',
    in_review: 'In review',
    published: 'Published',
    archived: 'Archived',
  };
</script>

<div class="tab">
  <Segmented
    label="Materials section"
    bind:value={view}
    options={[
      { value: 'lectures', label: 'Lectures' },
      { value: 'book', label: 'Course book' },
      { value: 'compare', label: 'Compare' },
      { value: 'settings', label: 'Settings' },
    ]}
  />

  {#if !courseId}
    <EmptyState
      icon={FileText}
      title="Choose a course first"
      description="Pick a course on the Course tab to see its lecture materials."
    />
  {:else if view === 'lectures'}
    {#if selected}
      <button
        type="button"
        class="back"
        onclick={() => {
          selected = null;
          void load();
        }}>← All lectures</button
      >
      {#key selected.code}
        <LectureMaterialPanel
          sessionCode={selected.code}
          sessionId={selected.id}
          title={selected.title}
          onChanged={() => void load()}
        />
      {/key}
    {:else if loading}
      <div class="center"><Spinner /></div>
    {:else}
      {#if error}<p class="err" role="alert">{error}</p>{/if}
      {#if rows.length === 0 && !extra}
        <EmptyState
          icon={FileText}
          title="No handouts yet"
          description="After a lecture ends Klasync drafts a handout here. Turn on automatic drafting in Settings, or open a lecture's Room to generate one."
        />
      {/if}
      <ul class="list">
        {#if extra}
          <li>
            <button
              type="button"
              onclick={() =>
                (selected = {
                  code: extra.code,
                  id: extra.id,
                  title: extra.title,
                })}
            >
              <span class="t">{extra.title}</span>
              <span class="s">Current lecture · not drafted yet</span>
            </button>
          </li>
        {/if}
        {#each rows as r (r.id)}
          <li>
            <button
              type="button"
              onclick={() =>
                (selected = {
                  code: r.short_code,
                  id: r.session_id,
                  title: r.title,
                })}
            >
              <span class="t">{r.title}</span>
              <span class="s">
                {#if r.status}
                  <span class="pill" data-status={r.status}
                    >{statusLabel[r.status]}</span
                  >
                {:else}
                  <span class="pill">Not drafted</span>
                {/if}
                {#if r.generation_state === 'queued' || r.generation_state === 'generating'}
                  · drafting…
                {:else if r.generation_state === 'failed'}
                  · generation failed
                {/if}
                {#if r.started_at}· {new Date(
                    r.started_at
                  ).toLocaleDateString()}{/if}
              </span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  {:else if view === 'book'}
    {#key courseId}<BookBuilder {courseId} />{/key}
  {:else if view === 'compare'}
    {#key courseId}<CompareView {courseId} {courseCode} />{/key}
  {:else}
    {#key courseId}<MaterialSettingsPanel {courseId} />{/key}
  {/if}
</div>

<style>
  .tab {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .center {
    display: grid;
    place-items: center;
    padding: 40px;
  }
  .back {
    align-self: flex-start;
    border: 0;
    background: transparent;
    color: var(--text-secondary);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .back:hover {
    color: var(--text-primary);
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .list button {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 10px 14px;
    text-align: left;
    border: 1px solid var(--border-subtle);
    border-radius: 12px;
    background: var(--surface-1);
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .list button:hover {
    border-color: var(--border-strong);
  }
  .t {
    color: var(--text-primary);
    font-weight: 600;
  }
  .s {
    font-size: 12.5px;
    color: var(--text-tertiary);
  }
  .pill {
    padding: 0 8px;
    border: 1px solid var(--border-default);
    border-radius: 999px;
    font-size: 11.5px;
  }
  .pill[data-status='published'] {
    border-color: #4c9a68;
    color: #7fd3a0;
  }
  .err {
    margin: 0;
    padding: 8px 12px;
    border: 1px solid #ff9b7a;
    border-radius: 8px;
    color: #ff9b7a;
    font-size: 13px;
  }
</style>
