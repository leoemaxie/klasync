<script lang="ts">
  import { ClipboardCheck, Plus } from '@lucide/svelte';
  import {
    createAssessment,
    listCourseAssessments,
    listCourseLectures,
    type Assessment,
    type AssessmentKind,
    type LectureOption,
  } from '$lib/api/assessments';
  import type { Course } from '$lib/types';
  import Button from '$lib/components/ui/Button.svelte';
  import EmptyState from '$lib/components/ui/EmptyState.svelte';
  import SkeletonCard from '$lib/components/shared/SkeletonCard.svelte';
  import AssessmentWorkspace from './lecturer/AssessmentWorkspace.svelte';

  let { course }: { course: Course | null } = $props();

  let assessments = $state<Assessment[]>([]);
  let lectures = $state<LectureOption[]>([]);
  let loading = $state(true);
  let error = $state('');
  let openId = $state<string | null>(null);
  let creating = $state(false);
  let lectureId = $state('');
  let title = $state('');
  let kind = $state<AssessmentKind>('quiz');
  let busy = $state(false);

  async function load(courseId: string) {
    loading = true;
    error = '';
    try {
      [assessments, lectures] = await Promise.all([
        listCourseAssessments(courseId),
        listCourseLectures(courseId),
      ]);
      if (!lectureId && lectures.length > 0) lectureId = lectures[0].id;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not load assessments';
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    const id = course?.id;
    if (id) void load(id);
    else loading = false;
  });

  const lectureById = $derived(new Map(lectures.map((l) => [l.id, l])));

  async function create() {
    const lecture = lectures.find((l) => l.id === lectureId);
    if (!lecture || !title.trim()) {
      error = 'Choose a lecture and give the assessment a title.';
      return;
    }
    busy = true;
    error = '';
    try {
      const row = await createAssessment(lecture.short_code, {
        title: title.trim(),
        kind,
      });
      title = '';
      creating = false;
      openId = row.id;
      if (course) await load(course.id);
    } catch (e) {
      error =
        e instanceof Error ? e.message : 'Could not create the assessment';
    } finally {
      busy = false;
    }
  }

  const STATUS = {
    draft: 'Draft',
    published: 'Published',
    closed: 'Closed',
    finalized: 'Finalised',
    archived: 'Archived',
  };
  const when = (iso: string | null) =>
    iso
      ? new Date(iso).toLocaleDateString(undefined, { dateStyle: 'medium' })
      : '';
</script>

{#if openId}
  <AssessmentWorkspace
    id={openId}
    onBack={() => {
      openId = null;
      if (course) void load(course.id);
    }}
  />
{:else}
  <div class="tab as-scope">
    <header class="head">
      <div>
        <p class="eyebrow-line">Quizzes and homework</p>
        <h2>Assessments</h2>
        {#if course}<p class="sub">{course.code} · {course.title}</p>{/if}
      </div>
      {#if course}
        <Button variant="accent" onclick={() => (creating = !creating)}
          ><Plus size={15} /> New assessment</Button
        >
      {/if}
    </header>

    {#if !course}
      <EmptyState
        icon={ClipboardCheck}
        title="Choose a course first"
        description="Pick a course in the Course tab, then create assessments from its lectures."
      />
    {:else}
      {#if creating}
        <form
          class="create"
          onsubmit={(e) => {
            e.preventDefault();
            void create();
          }}
        >
          <label>
            <span>From which lecture?</span>
            <select bind:value={lectureId}>
              {#each lectures as l (l.id)}
                <option value={l.id}>
                  {l.title}{l.started_at
                    ? ` · ${when(l.started_at)}`
                    : ''}{l.has_transcript ? '' : ' (no transcript)'}
                </option>
              {/each}
            </select>
          </label>
          <label>
            <span>Title</span>
            <input
              type="text"
              bind:value={title}
              placeholder="e.g. Week 3 quiz: conduction"
            />
          </label>
          <label>
            <span>Type</span>
            <select bind:value={kind}>
              <option value="quiz">Quiz</option>
              <option value="homework">Homework</option>
              <option value="practice">Practice</option>
            </select>
          </label>
          <div class="row">
            <Button variant="ghost" onclick={() => (creating = false)}
              >Cancel</Button
            >
            <Button
              variant="accent"
              type="submit"
              disabled={busy || lectures.length === 0}>Create</Button
            >
          </div>
          {#if lectures.length === 0}
            <p class="note">
              This course has no lectures yet. Record one first.
            </p>
          {/if}
        </form>
      {/if}

      {#if error}<p class="oops" role="alert">{error}</p>{/if}

      {#if loading}
        <SkeletonCard lines={3} label="Loading assessments…" />
      {:else if assessments.length === 0}
        <EmptyState
          icon={ClipboardCheck}
          title="No assessments yet"
          description="Create one from a lecture. You can write the questions, have AI draft them, or both."
        />
      {:else}
        <ul class="list">
          {#each assessments as a (a.id)}
            <li>
              <button
                type="button"
                class="card"
                onclick={() => (openId = a.id)}
              >
                <span class="main">
                  <strong>{a.title}</strong>
                  <span class="meta">
                    {lectureById.get(a.session_id)?.title ?? 'Lecture'} · {a.kind}
                    {#if a.due_at}· due {when(a.due_at)}{/if}
                  </span>
                </span>
                <span class="status" data-s={a.status}>{STATUS[a.status]}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    {/if}
  </div>
{/if}

<style>
  .tab {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .head {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    gap: 12px;
    flex-wrap: wrap;
    padding-bottom: 14px;
    border-bottom: 1px solid var(--border-subtle);
  }
  .eyebrow-line {
    margin: 0 0 2px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--text-accent);
  }
  h2 {
    margin: 0;
    font-size: 28px;
    font-weight: 500;
  }
  .sub {
    margin: 2px 0 0;
    font-size: 13px;
    color: var(--text-tertiary);
  }
  .create {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 12px;
    padding: 16px;
    background: var(--surface-2);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
  }
  input,
  select {
    padding: 9px 11px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-default);
    background: var(--surface-inset);
    color: var(--text-primary);
    font: inherit;
    font-size: 14px;
    text-transform: none;
    letter-spacing: 0;
  }
  .row {
    grid-column: 1 / -1;
    display: flex;
    gap: 10px;
    justify-content: flex-end;
  }
  .note,
  .oops {
    grid-column: 1 / -1;
    margin: 0;
    font-size: 13px;
  }
  .note {
    color: var(--text-tertiary);
  }
  .oops {
    color: #d9715c;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .card {
    width: 100%;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    padding: 14px 16px;
    background: var(--surface-2);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    color: var(--text-primary);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .card:hover {
    border-color: var(--border-default);
    background: var(--surface-hover);
  }
  .main {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .meta {
    font-size: 12.5px;
    color: var(--text-tertiary);
  }
  .status {
    font-size: 12px;
    padding: 3px 11px;
    border-radius: 99px;
    background: var(--surface-inset);
    color: var(--text-secondary);
    white-space: nowrap;
  }
  .status[data-s='published'] {
    background: var(--surface-accent);
    color: var(--text-accent);
  }
  .status[data-s='finalized'] {
    color: #5fae7a;
  }
</style>
