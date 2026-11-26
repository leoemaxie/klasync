<script lang="ts">
  import { onMount } from 'svelte';
  import { ArrowLeft, Plus, Sparkles, TriangleAlert } from '@lucide/svelte';
  import {
    addQuestion,
    archiveAssessment,
    closeAssessment,
    deleteQuestion,
    finalizeAssessment,
    getAssessment,
    publishAssessment,
    QUESTION_TYPE_LABEL,
    releaseResults,
    reorderQuestions,
    type AssessmentDetail,
    type Question,
    type QuestionType,
  } from '$lib/api/assessments';
  import { publishProblems } from '$lib/assessments/questionRules';
  import Button from '$lib/components/ui/Button.svelte';
  import Segmented from '$lib/components/ui/Segmented.svelte';
  import Spinner from '$lib/components/ui/Spinner.svelte';
  import AssessmentReportView from './AssessmentReportView.svelte';
  import GenerateDialog from './GenerateDialog.svelte';
  import GradingWorkbench from './GradingWorkbench.svelte';
  import QuestionEditor from './QuestionEditor.svelte';
  import SettingsPanel from './SettingsPanel.svelte';

  let { id, onBack }: { id: string; onBack: () => void } = $props();

  type Tab = 'questions' | 'settings' | 'grading' | 'report';
  let tab = $state<Tab>('questions');
  let detail = $state<AssessmentDetail | null>(null);
  let loadError = $state('');
  let message = $state('');
  let messageKind = $state<'ok' | 'error'>('ok');
  let busy = $state(false);
  let generating = $state(false);
  let reportKey = $state(0);

  const a = $derived(detail?.assessment ?? null);
  const status = $derived(a?.status ?? 'draft');
  const editable = $derived(status !== 'finalized' && status !== 'archived');
  const problems = $derived(detail ? publishProblems(detail.questions) : []);
  const released = $derived(!!a?.results_released_at);
  const tabs = $derived([
    { value: 'questions' as Tab, label: 'Questions' },
    { value: 'settings' as Tab, label: 'Settings' },
    ...(status !== 'draft'
      ? [
          { value: 'grading' as Tab, label: 'Marking' },
          { value: 'report' as Tab, label: 'Report' },
        ]
      : []),
  ]);

  async function load() {
    try {
      detail = await getAssessment(id);
      loadError = '';
    } catch (e) {
      loadError =
        e instanceof Error ? e.message : 'Could not load the assessment';
    }
  }

  onMount(load);

  // Question edits bump the revision; settings need the latest one.
  $effect(() => {
    if (tab === 'settings' || tab === 'report' || tab === 'grading')
      void load();
  });

  function say(text: string, kind: 'ok' | 'error' = 'ok') {
    message = text;
    messageKind = kind;
  }

  async function run(action: () => Promise<unknown>, success: string) {
    busy = true;
    try {
      await action();
      say(success);
      await load();
      reportKey++;
    } catch (e) {
      say(e instanceof Error ? e.message : 'That did not work', 'error');
    } finally {
      busy = false;
    }
  }

  const publish = () =>
    run(async () => {
      const r = await publishAssessment(id);
      if (!r.new_version)
        throw new Error('Nothing has changed since the last publish.');
    }, 'Published. Enrolled students can now see it.');

  function close() {
    if (
      !confirm(
        'Close this assessment? Students still taking it are submitted now, and no new attempts can start.'
      )
    )
      return;
    void run(() => closeAssessment(id), 'Closed.');
  }

  function finalize() {
    if (
      !confirm(
        'Finalise? This locks every grade and shows you student names. It cannot be undone.'
      )
    )
      return;
    void run(
      () => finalizeAssessment(id),
      'Finalised. Names are now visible in the report.'
    );
  }

  const toggleRelease = () =>
    run(
      () => releaseResults(id, !released),
      released
        ? 'Results hidden from students.'
        : 'Results released to students.'
    );

  function archive() {
    if (!confirm('Archive this assessment?')) return;
    void run(() => archiveAssessment(id), 'Archived.');
  }

  async function add(kind: QuestionType) {
    if (!detail) return;
    try {
      const q = await addQuestion(id, {
        question_type: kind,
        prompt_md: '',
        points: 1,
      });
      detail.questions = [...detail.questions, q];
    } catch (e) {
      say(
        e instanceof Error ? e.message : 'Could not add the question',
        'error'
      );
    }
  }

  async function remove(q: Question) {
    if (!confirm('Delete this question?')) return;
    try {
      await deleteQuestion(id, q.id);
      if (detail)
        detail.questions = detail.questions.filter((x) => x.id !== q.id);
    } catch (e) {
      say(e instanceof Error ? e.message : 'Could not delete', 'error');
    }
  }

  async function move(index: number, direction: -1 | 1) {
    if (!detail) return;
    const list = [...detail.questions];
    const target = index + direction;
    if (target < 0 || target >= list.length) return;
    [list[index], list[target]] = [list[target], list[index]];
    detail.questions = list;
    try {
      await reorderQuestions(
        id,
        list.map((q) => q.id)
      );
    } catch (e) {
      say(e instanceof Error ? e.message : 'Could not reorder', 'error');
      await load();
    }
  }

  function saved(updated: Question) {
    if (!detail) return;
    detail.questions = detail.questions.map((q) =>
      q.id === updated.id ? updated : q
    );
  }

  const STATUS_LABEL = {
    draft: 'Draft',
    published: 'Published',
    closed: 'Closed',
    finalized: 'Finalised',
    archived: 'Archived',
  };
  const TYPES: QuestionType[] = [
    'mcq_single',
    'mcq_multi',
    'true_false',
    'short_answer',
    'long_answer',
  ];
</script>

<div class="ws">
  <button class="back" type="button" onclick={onBack}
    ><ArrowLeft size={15} /> All assessments</button
  >

  {#if loadError}
    <p class="oops" role="alert">{loadError}</p>
  {:else if !detail || !a}
    <Spinner />
  {:else}
    <header class="head">
      <div class="title">
        <h2>{a.title}</h2>
        <span class="status" data-s={status}>{STATUS_LABEL[status]}</span>
      </div>
      <div class="actions">
        {#if status === 'draft'}
          <Button
            variant="accent"
            disabled={busy || problems.length > 0}
            onclick={publish}>Publish</Button
          >
        {:else if status === 'published'}
          {#if detail.unpublished_changes}
            <Button
              variant="accent"
              disabled={busy || problems.length > 0}
              onclick={publish}>Republish changes</Button
            >
          {/if}
          <Button variant="outline" disabled={busy} onclick={close}
            >Close</Button
          >
        {:else if status === 'closed'}
          <Button
            variant="accent"
            disabled={busy || detail.pending_grades > 0}
            onclick={finalize}>Finalise</Button
          >
        {/if}
        {#if status === 'published' || status === 'closed' || status === 'finalized'}
          <Button variant="outline" disabled={busy} onclick={toggleRelease}>
            {released ? 'Hide results' : 'Release results'}
          </Button>
        {/if}
        {#if status === 'draft' || status === 'finalized'}
          <Button variant="ghost" disabled={busy} onclick={archive}
            >Archive</Button
          >
        {/if}
      </div>
    </header>

    <p class="summary">
      {detail.questions.length} question{detail.questions.length === 1
        ? ''
        : 's'} ·
      {detail.attempts} attempt{detail.attempts === 1 ? '' : 's'} ({detail.submitted}
      submitted)
      {#if detail.version}· Version {detail.version.version_no}{/if}
      {#if status === 'closed' && detail.pending_grades > 0}· {detail.pending_grades}
        answers to mark before you can finalise{/if}
    </p>

    {#if message}
      <p class="msg" class:err={messageKind === 'error'} role="status">
        {message}
      </p>
    {/if}
    {#if status === 'draft' && problems.length > 0 && detail.questions.length > 0}
      <div class="blockers" role="status">
        <TriangleAlert size={15} />
        <div>
          <strong>Before you can publish</strong>
          <ul>
            {#each problems.slice(0, 4) as p (p)}<li>{p}</li>{/each}
          </ul>
        </div>
      </div>
    {/if}
    {#if status === 'published' && detail.unpublished_changes}
      <p class="msg">
        You've edited this since publishing. Students still see the last
        published version until you republish.
      </p>
    {/if}

    <Segmented label="Section" options={tabs} bind:value={tab} />

    {#if tab === 'questions'}
      <div class="qs">
        {#each detail.questions as q, i (q.id)}
          <QuestionEditor
            assessmentId={id}
            question={q}
            index={i}
            count={detail.questions.length}
            {editable}
            onSaved={saved}
            onDelete={() => remove(q)}
            onMove={(d) => move(i, d)}
          />
        {/each}
        {#if detail.questions.length === 0}
          <p class="empty">
            No questions yet. Write one, or let AI draft some from the lecture.
          </p>
        {/if}
        {#if editable}
          <div class="adders">
            <Button variant="soft" onclick={() => (generating = true)}
              ><Sparkles size={14} /> Generate with AI</Button
            >
            <span class="or">or add</span>
            {#each TYPES as t (t)}
              <Button variant="outline" size="sm" onclick={() => add(t)}
                ><Plus size={13} /> {QUESTION_TYPE_LABEL[t]}</Button
              >
            {/each}
          </div>
        {/if}
      </div>
    {:else if tab === 'settings'}
      {#key a.draft_rev}
        <SettingsPanel
          assessment={a}
          settings={detail.settings}
          {editable}
          onSaved={async () => {
            await load();
            say('Settings saved.');
          }}
        />
      {/key}
    {:else if tab === 'grading'}
      <GradingWorkbench
        assessmentId={id}
        locked={status === 'finalized'}
        aiMode={detail.settings.scoring_mode !== 'manual'}
        onChanged={load}
      />
    {:else}
      <AssessmentReportView
        assessmentId={id}
        revealed={!!a.identity_revealed_at}
        refreshKey={reportKey}
      />
    {/if}

    {#if generating}
      <GenerateDialog
        assessmentId={id}
        onDone={load}
        onClose={() => {
          generating = false;
          void load();
        }}
      />
    {/if}
  {/if}
</div>

<style>
  .ws {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .back {
    align-self: flex-start;
    display: inline-flex;
    gap: 6px;
    align-items: center;
    background: none;
    border: 0;
    color: var(--text-secondary);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
    padding: 0;
  }
  .head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
  }
  .title {
    display: flex;
    gap: 12px;
    align-items: center;
    flex-wrap: wrap;
  }
  h2 {
    margin: 0;
    font-size: 26px;
    font-weight: 500;
  }
  .status {
    font-size: 12px;
    padding: 3px 11px;
    border-radius: 99px;
    background: var(--surface-inset);
    color: var(--text-secondary);
  }
  .status[data-s='published'] {
    background: var(--surface-accent);
    color: var(--text-accent);
  }
  .status[data-s='finalized'] {
    color: #5fae7a;
  }
  .actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .summary {
    margin: 0;
    font-size: 13px;
    color: var(--text-tertiary);
  }
  .msg {
    margin: 0;
    padding: 9px 14px;
    border-radius: var(--radius-md);
    background: var(--surface-accent);
    font-size: 13px;
    color: var(--text-secondary);
  }
  .msg.err {
    color: #d9715c;
  }
  .blockers {
    display: flex;
    gap: 10px;
    padding: 12px 14px;
    border-radius: var(--radius-md);
    border: 1px solid color-mix(in srgb, #e3a23c 50%, transparent);
    color: #e3a23c;
    font-size: 13px;
  }
  .blockers ul {
    margin: 4px 0 0;
    padding-left: 18px;
    color: var(--text-secondary);
  }
  .qs {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .adders {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;
    padding: 6px 0;
  }
  .or {
    font-size: 12.5px;
    color: var(--text-tertiary);
  }
  .empty {
    color: var(--text-tertiary);
    font-size: 14px;
  }
  .oops {
    color: #d9715c;
  }
</style>
