<script lang="ts">
  import { onMount } from 'svelte';
  import { ClipboardCheck, CloudOff, ChartNoAxesColumn } from '@lucide/svelte';
  import {
    getStudentAssessments,
    type StudentAssessment,
    type StudentAssessmentState,
  } from '$lib/api/assessments';
  import {
    isNetworkError,
    type StoredAttempt,
  } from '$lib/assessments/attemptEngine';
  import { listUnfinishedAttempts } from '$lib/storage/attemptStore';
  import SkeletonCard from '$lib/components/shared/SkeletonCard.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import EmptyState from '$lib/components/ui/EmptyState.svelte';
  import AssessmentTaker from './AssessmentTaker.svelte';
  import AssessmentResult from './AssessmentResult.svelte';
  import StudentProgressCard from './StudentProgressCard.svelte';

  type View =
    | { kind: 'list' }
    | { kind: 'take'; item: StudentAssessment }
    | { kind: 'result'; attemptId: string; title: string };

  let view = $state<View>({ kind: 'list' });
  let items = $state<StudentAssessment[]>([]);
  let local = $state<StoredAttempt[]>([]);
  let loading = $state(true);
  let offline = $state(false);
  let openProgress = $state<Record<string, boolean>>({});

  async function load() {
    loading = true;
    offline = false;
    local = await listUnfinishedAttempts();
    try {
      items = await getStudentAssessments();
    } catch (error) {
      items = [];
      offline = isNetworkError(error);
    } finally {
      loading = false;
    }
  }

  onMount(load);

  const STATE_LABEL: Record<StudentAssessmentState, string> = {
    upcoming: 'Opens soon',
    open: 'Open',
    in_progress: 'In progress',
    submitted: 'Submitted',
    closed: 'Closed',
  };

  const groups = $derived.by(() => {
    const byCourse = new Map<
      string,
      { id: string; code: string; title: string; items: StudentAssessment[] }
    >();
    for (const item of items) {
      const group = byCourse.get(item.course_id) ?? {
        id: item.course_id,
        code: item.course_code,
        title: item.course_title,
        items: [],
      };
      group.items.push(item);
      byCourse.set(item.course_id, group);
    }
    return [...byCourse.values()];
  });

  /** Local attempts the server list does not already show as in progress. */
  const savedHere = $derived(
    local.filter(
      (a) => !items.some((i) => i.in_progress_attempt_id === a.id) || offline
    )
  );

  function fromStored(a: StoredAttempt): StudentAssessment {
    const v = a.view;
    return {
      id: v.assessment_id,
      title: v.title,
      kind: v.kind,
      status: 'published',
      course_id: '',
      course_code: 'Saved on this device',
      course_title: '',
      session_id: '',
      lecture_title: '',
      instructions_md: v.instructions_md,
      opens_at: null,
      due_at: null,
      closes_at: null,
      time_limit_secs: v.settings.time_limit_secs,
      rules: v.settings,
      integrity: v.settings.records_integrity ? 'monitored' : 'open',
      max_attempts: v.settings.max_attempts,
      attempts_used: v.attempt_no,
      in_progress_attempt_id: a.id,
      latest_attempt_id: a.id,
      state: 'in_progress',
      can_start: true,
      results_available: false,
    };
  }

  function open(item: StudentAssessment) {
    if (item.in_progress_attempt_id || item.can_start) {
      view = { kind: 'take', item };
    } else if (item.results_available && item.latest_attempt_id) {
      view = {
        kind: 'result',
        attemptId: item.latest_attempt_id,
        title: item.title,
      };
    }
  }

  function back(refresh = false) {
    view = { kind: 'list' };
    if (refresh) void load();
  }

  const due = (iso: string | null) =>
    iso
      ? new Date(iso).toLocaleString(undefined, {
          dateStyle: 'medium',
          timeStyle: 'short',
        })
      : null;
</script>

{#if view.kind === 'take'}
  {#key view.item.id + (view.item.in_progress_attempt_id ?? '')}
    <AssessmentTaker assessment={view.item} onExit={back} />
  {/key}
{:else if view.kind === 'result'}
  <AssessmentResult
    attemptId={view.attemptId}
    title={view.title}
    onBack={() => back()}
  />
{:else}
  <div class="hub">
    <header class="head">
      <div>
        <p class="eyebrow-line">Quizzes and homework</p>
        <h2>Assessments</h2>
      </div>
    </header>

    {#if offline}
      <p class="banner">
        <CloudOff size={15} /> You're offline. Work saved on this device is below.
      </p>
    {/if}

    {#if savedHere.length > 0}
      <section class="saved">
        <h3>Saved on this device</h3>
        <ul class="list">
          {#each savedHere as a (a.id)}
            <li class="row surface">
              <div class="main">
                <strong>{a.view.title}</strong>
                <span class="meta">
                  {a.phase === 'in_progress'
                    ? 'Attempt in progress'
                    : 'Submitted here, waiting to send'}
                  · {a.pendingOps.length} change{a.pendingOps.length === 1
                    ? ''
                    : 's'} not yet sent
                </span>
              </div>
              <Button
                variant="soft"
                onclick={() => (view = { kind: 'take', item: fromStored(a) })}
                >Continue</Button
              >
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if loading}
      <SkeletonCard lines={3} label="Loading assessments…" />
    {:else if groups.length === 0 && !offline}
      <EmptyState
        icon={ClipboardCheck}
        title="Nothing to take yet"
        description="When a lecturer publishes a quiz or homework for one of your courses, it appears here."
      />
    {:else}
      {#each groups as group (group.id)}
        <section class="course">
          <div class="course-head">
            <h3><span class="course-code">{group.code}</span> {group.title}</h3>
            <button
              type="button"
              class="link"
              aria-expanded={!!openProgress[group.id]}
              onclick={() => (openProgress[group.id] = !openProgress[group.id])}
            >
              <ChartNoAxesColumn size={14} />
              {openProgress[group.id] ? 'Hide progress' : 'My progress'}
            </button>
          </div>
          {#if openProgress[group.id]}
            <StudentProgressCard courseId={group.id} />
          {/if}
          <ul class="list">
            {#each group.items as item (item.id)}
              <li class="row surface">
                <div class="main">
                  <strong>{item.title}</strong>
                  <span class="meta">
                    {item.lecture_title}
                    {#if item.due_at}· Due {due(item.due_at)}{/if}
                    {#if item.time_limit_secs}· {Math.round(
                        item.time_limit_secs / 60
                      )} min{/if}
                    · {item.attempts_used}/{item.max_attempts} attempts
                  </span>
                </div>
                <span class="chip" data-state={item.state}
                  >{STATE_LABEL[item.state]}</span
                >
                {#if item.in_progress_attempt_id || item.can_start}
                  <Button variant="accent" onclick={() => open(item)}>
                    {item.in_progress_attempt_id ? 'Continue' : 'Open'}
                  </Button>
                {/if}
                {#if item.results_available && item.latest_attempt_id}
                  <Button
                    variant="outline"
                    onclick={() =>
                      (view = {
                        kind: 'result',
                        attemptId: item.latest_attempt_id!,
                        title: item.title,
                      })}>Results</Button
                  >
                {/if}
              </li>
            {/each}
          </ul>
        </section>
      {/each}
    {/if}
  </div>
{/if}

<style>
  .hub {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .head {
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
  .head h2 {
    margin: 0;
    font-size: 28px;
    font-weight: 500;
  }
  .banner {
    margin: 0;
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 9px 14px;
    border-radius: var(--radius-md);
    background: var(--surface-accent);
    font-size: 13px;
    color: var(--text-secondary);
  }
  h3 {
    margin: 0;
    font-size: 15px;
    font-weight: 500;
  }
  .course-code {
    color: var(--text-accent);
    font-weight: 600;
    margin-right: 6px;
  }
  .course,
  .saved {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .course-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
  .link {
    display: inline-flex;
    gap: 6px;
    align-items: center;
    background: none;
    border: 0;
    color: var(--text-secondary);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .surface {
    background: var(--surface-2);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 14px;
    flex-wrap: wrap;
  }
  .main {
    flex: 1;
    min-width: 200px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .meta {
    font-size: 12.5px;
    color: var(--text-tertiary);
  }
  .chip {
    font-size: 11.5px;
    padding: 3px 10px;
    border-radius: 999px;
    background: var(--surface-inset);
    color: var(--text-secondary);
  }
  .chip[data-state='open'],
  .chip[data-state='in_progress'] {
    background: var(--surface-accent);
    color: var(--text-accent);
  }
</style>
