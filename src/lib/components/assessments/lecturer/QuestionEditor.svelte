<script lang="ts">
  import {
    ArrowDown,
    ArrowUp,
    Lock,
    Plus,
    Sparkles,
    Trash2,
    TriangleAlert,
    Unlock,
    X,
  } from '@lucide/svelte';
  import {
    patchQuestion,
    QUESTION_TYPE_LABEL,
    type Difficulty,
    type Question,
    type RubricCriterion,
  } from '$lib/api/assessments';
  import {
    nextOptionId,
    questionProblem,
    rubricTotal,
  } from '$lib/assessments/questionRules';

  let {
    assessmentId,
    question,
    index,
    count,
    editable = true,
    onSaved,
    onDelete,
    onMove,
  }: {
    assessmentId: string;
    question: Question;
    index: number;
    count: number;
    editable?: boolean;
    onSaved: (q: Question) => void;
    onDelete: () => void;
    onMove: (direction: -1 | 1) => void;
  } = $props();

  // A working copy, so typing never fights the server round trip.
  const copy = () => structuredClone($state.snapshot(question));
  const acceptedOf = () => (question.key_payload.accepted ?? []).join('\n');
  let draft = $state<Question>(copy());
  let acceptedText = $state(acceptedOf());
  let saving = $state(false);
  let saved = $state(false);
  let error = $state('');
  let dirty = false;

  // Re-sync when a different question (or a server-side change) arrives.
  let lastId = $state.raw('');
  let lastStamp = '';
  $effect(() => {
    const stamp = JSON.stringify(question);
    if (question.id !== lastId || (!dirty && stamp !== lastStamp)) {
      lastId = question.id;
      draft = copy();
      acceptedText = acceptedOf();
    }
    lastStamp = stamp;
  });

  const problem = $derived(questionProblem(draft));
  const type = $derived(draft.question_type);
  const options = $derived(draft.public_payload.options ?? []);
  const correctIds = $derived(
    Array.isArray(draft.key_payload.correct) ? draft.key_payload.correct : []
  );
  const rubric = $derived(draft.key_payload.rubric ?? []);

  async function save() {
    if (!editable || !dirty) return;
    saving = true;
    error = '';
    try {
      const updated = await patchQuestion(assessmentId, draft.id, {
        prompt_md: draft.prompt_md,
        points: Number(draft.points),
        difficulty: draft.difficulty,
        topic_tag: draft.topic_tag?.trim() ? draft.topic_tag.trim() : null,
        public_payload: draft.public_payload,
        key_payload: draft.key_payload,
        explanation_md: draft.explanation_md,
        locked: draft.locked,
      });
      dirty = false;
      saved = true;
      setTimeout(() => (saved = false), 1800);
      onSaved(updated);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not save';
    } finally {
      saving = false;
    }
  }

  function touch() {
    dirty = true;
    saved = false;
  }

  function setCorrect(id: string, on: boolean) {
    touch();
    if (type === 'mcq_single') {
      draft.key_payload.correct = [id];
    } else {
      const set = new Set(correctIds);
      if (on) set.add(id);
      else set.delete(id);
      draft.key_payload.correct = [...set];
    }
    void save();
  }

  function addOption() {
    if (options.length >= 10) return;
    touch();
    draft.public_payload.options = [
      ...options,
      { id: nextOptionId(options), text: '' },
    ];
  }

  function removeOption(id: string) {
    touch();
    draft.public_payload.options = options.filter((o) => o.id !== id);
    draft.key_payload.correct = correctIds.filter((c) => c !== id);
    void save();
  }

  function setAccepted() {
    touch();
    draft.key_payload.accepted = acceptedText
      .split('\n')
      .map((s) => s.trim())
      .filter(Boolean);
  }

  function addCriterion() {
    touch();
    const next: RubricCriterion = {
      id: `r${rubric.length + 1}`,
      criterion: '',
      points: 1,
      description: '',
    };
    draft.key_payload.rubric = [...rubric, next];
  }

  function removeCriterion(id: string) {
    touch();
    draft.key_payload.rubric = rubric.filter((c) => c.id !== id);
    void save();
  }

  const stamp = (sec: number) =>
    `${Math.floor(sec / 60)}:${String(sec % 60).padStart(2, '0')}`;
</script>

<article class="q as-scope" class:invalid={!!problem}>
  <header>
    <span class="num">{index + 1}</span>
    <span class="type">{QUESTION_TYPE_LABEL[type]}</span>
    {#if question.origin === 'ai'}
      <span class="badge ai"><Sparkles size={12} /> AI draft</span>
    {:else if question.origin === 'ai_edited'}
      <span class="badge"><Sparkles size={12} /> AI, edited</span>
    {/if}
    {#if question.low_confidence}
      <span
        class="badge warn"
        title="The model was unsure, or this has no matching moment in the lecture. Check it."
        ><TriangleAlert size={12} /> Check this</span
      >
    {/if}
    {#if question.source_ranges.length > 0}
      <span class="badge muted"
        >From {stamp(question.source_ranges[0].start_sec)}</span
      >
    {/if}
    <span class="spacer"></span>
    {#if saving}<span class="state">Saving…</span>
    {:else if saved}<span class="state ok">Saved</span>{/if}
    {#if editable}
      <button
        type="button"
        class="icon"
        aria-label="Move up"
        disabled={index === 0}
        onclick={() => onMove(-1)}><ArrowUp size={15} /></button
      >
      <button
        type="button"
        class="icon"
        aria-label="Move down"
        disabled={index === count - 1}
        onclick={() => onMove(1)}><ArrowDown size={15} /></button
      >
      <button
        type="button"
        class="icon"
        aria-pressed={draft.locked}
        aria-label={draft.locked ? 'Unlock' : 'Lock from AI changes'}
        title={draft.locked
          ? 'Locked: AI will not change this'
          : 'Lock so AI never changes it'}
        onclick={() => {
          touch();
          draft.locked = !draft.locked;
          void save();
        }}
        >{#if draft.locked}<Lock size={15} />{:else}<Unlock
            size={15}
          />{/if}</button
      >
      <button
        type="button"
        class="icon is-danger"
        aria-label="Delete question"
        onclick={onDelete}><Trash2 size={15} /></button
      >
    {/if}
  </header>

  <label class="field">
    <span>Question</span>
    <textarea
      rows="3"
      bind:value={draft.prompt_md}
      disabled={!editable}
      oninput={touch}
      onblur={save}
      placeholder="Write the question. Markdown works: **bold**, `code`."
    ></textarea>
  </label>

  {#if type === 'mcq_single' || type === 'mcq_multi'}
    <fieldset class="field">
      <legend>
        Options <em
          >{type === 'mcq_single'
            ? 'Mark the one correct answer'
            : 'Mark every correct answer'}</em
        >
      </legend>
      {#each options as option (option.id)}
        <div class="opt">
          <input
            type={type === 'mcq_single' ? 'radio' : 'checkbox'}
            name={`correct-${draft.id}`}
            checked={correctIds.includes(option.id)}
            disabled={!editable}
            aria-label={`Option ${option.id} is correct`}
            onchange={(e) => setCorrect(option.id, e.currentTarget.checked)}
          />
          <input
            class="inp"
            type="text"
            bind:value={option.text}
            disabled={!editable}
            oninput={touch}
            onblur={save}
            placeholder="Option text"
          />
          {#if editable && options.length > 2}
            <button
              type="button"
              class="icon"
              aria-label="Remove option"
              onclick={() => removeOption(option.id)}><X size={14} /></button
            >
          {/if}
        </div>
      {/each}
      {#if editable && options.length < 10}
        <button type="button" class="add" onclick={addOption}
          ><Plus size={14} /> Add option</button
        >
      {/if}
      {#if type === 'mcq_multi'}
        <label class="inline">
          <input
            type="checkbox"
            checked={draft.key_payload.partial === 'proportional'}
            disabled={!editable}
            onchange={(e) => {
              touch();
              draft.key_payload.partial = e.currentTarget.checked
                ? 'proportional'
                : 'all_or_nothing';
              void save();
            }}
          />
          Give partial credit for some correct options
        </label>
      {/if}
    </fieldset>
  {:else if type === 'true_false'}
    <fieldset class="field">
      <legend>Correct answer</legend>
      <div class="seg">
        {#each [true, false] as value (value)}
          <label class:on={draft.key_payload.correct === value}>
            <input
              type="radio"
              name={`tf-${draft.id}`}
              checked={draft.key_payload.correct === value}
              disabled={!editable}
              onchange={() => {
                touch();
                draft.key_payload.correct = value;
                void save();
              }}
            />
            {value ? 'True' : 'False'}
          </label>
        {/each}
      </div>
    </fieldset>
  {:else if type === 'short_answer'}
    <label class="field">
      <span
        >Accepted answers <em>one per line; spacing and capitals are ignored</em
        ></span
      >
      <textarea
        rows="3"
        bind:value={acceptedText}
        disabled={!editable}
        oninput={setAccepted}
        onblur={save}
        placeholder="k&#10;thermal conductivity"></textarea>
    </label>
    <label class="inline">
      <input
        type="checkbox"
        checked={draft.key_payload.ai_fallback ?? false}
        disabled={!editable}
        onchange={(e) => {
          touch();
          draft.key_payload.ai_fallback = e.currentTarget.checked;
          void save();
        }}
      />
      Let AI mark answers that don't match (you confirm them)
    </label>
  {:else}
    <fieldset class="field">
      <legend>
        Rubric
        <em
          class:bad={rubric.length > 0 &&
            Math.abs(rubricTotal(rubric) - draft.points) > 0.01}
          >{rubricTotal(rubric)} of {draft.points} points</em
        >
      </legend>
      {#each rubric as criterion (criterion.id)}
        <div class="opt">
          <input
            class="inp"
            type="text"
            bind:value={criterion.criterion}
            disabled={!editable}
            oninput={touch}
            onblur={save}
            placeholder="What earns these marks"
          />
          <input
            class="num-in"
            type="number"
            min="0"
            step="0.5"
            bind:value={criterion.points}
            disabled={!editable}
            oninput={touch}
            onblur={save}
            aria-label="Points"
          />
          {#if editable}
            <button
              type="button"
              class="icon"
              aria-label="Remove criterion"
              onclick={() => removeCriterion(criterion.id)}
              ><X size={14} /></button
            >
          {/if}
        </div>
      {/each}
      {#if editable}
        <button type="button" class="add" onclick={addCriterion}
          ><Plus size={14} /> Add criterion</button
        >
      {/if}
    </fieldset>
    <label class="field">
      <span
        >Reference answer <em
          >for you and the AI marker; students never see it</em
        ></span
      >
      <textarea
        rows="3"
        value={draft.key_payload.reference ?? ''}
        disabled={!editable}
        oninput={(e) => {
          touch();
          draft.key_payload.reference = e.currentTarget.value;
        }}
        onblur={save}></textarea>
    </label>
  {/if}

  <div class="row">
    <label class="field narrow">
      <span>Points</span>
      <input
        type="number"
        min="0"
        step="0.5"
        bind:value={draft.points}
        disabled={!editable}
        oninput={touch}
        onblur={save}
      />
    </label>
    <label class="field narrow">
      <span>Difficulty</span>
      <select
        bind:value={draft.difficulty}
        disabled={!editable}
        onchange={() => {
          touch();
          void save();
        }}
      >
        {#each ['easy', 'medium', 'hard'] as d (d)}
          <option value={d as Difficulty}>{d}</option>
        {/each}
      </select>
    </label>
    <label class="field grow">
      <span>Topic <em>groups results in reports</em></span>
      <input
        type="text"
        bind:value={draft.topic_tag}
        disabled={!editable}
        oninput={touch}
        onblur={save}
        placeholder="e.g. conduction"
      />
    </label>
  </div>

  <label class="field">
    <span
      >Explanation <em>shown to students after results are released</em></span
    >
    <textarea
      rows="2"
      bind:value={draft.explanation_md}
      disabled={!editable}
      oninput={touch}
      onblur={save}></textarea>
  </label>

  {#if problem}
    <p class="problem"><TriangleAlert size={14} /> {problem}</p>
  {/if}
  {#if error}<p class="problem" role="alert">{error}</p>{/if}
</article>

<style>
  .q {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    background: var(--surface-2);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
  }
  .q.invalid {
    border-color: color-mix(in srgb, #e3a23c 55%, transparent);
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .num {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: 8px;
    background: var(--surface-accent);
    color: var(--text-accent);
    font-weight: 600;
    font-size: 13px;
  }
  .type {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-tertiary);
  }
  .badge {
    display: inline-flex;
    gap: 4px;
    align-items: center;
    font-size: 11.5px;
    padding: 2px 8px;
    border-radius: 99px;
    background: var(--surface-inset);
    color: var(--text-secondary);
  }
  .badge.ai {
    background: var(--surface-accent);
    color: var(--text-accent);
  }
  .badge.warn {
    color: #e3a23c;
  }
  .badge.muted {
    color: var(--text-tertiary);
  }
  .spacer {
    flex: 1;
  }
  .state {
    font-size: 12px;
    color: var(--text-tertiary);
  }
  .state.ok {
    color: #5fae7a;
  }
  .icon {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 8px;
    border: 0;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
  }
  .icon:hover:not(:disabled) {
    background: var(--surface-hover);
  }
  .icon:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .icon.is-danger:hover {
    color: #d9715c;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
    margin: 0;
    padding: 0;
    border: 0;
    min-width: 0;
  }
  .field > span,
  legend {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    padding: 0;
  }
  em {
    font-weight: 400;
    font-style: normal;
    color: var(--text-tertiary);
    margin-left: 6px;
  }
  em.bad {
    color: #e3a23c;
  }
  textarea,
  input.inp,
  input[type='number'],
  input[type='text'],
  select {
    width: 100%;
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
  textarea:focus,
  input:focus,
  select:focus {
    outline: none;
    border-color: var(--border-strong);
    box-shadow: var(--focus-ring);
  }
  .opt {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .opt .inp {
    flex: 1;
  }
  .num-in {
    width: 84px !important;
    flex: none;
  }
  .add {
    align-self: flex-start;
    display: inline-flex;
    gap: 5px;
    align-items: center;
    background: none;
    border: 0;
    color: var(--text-accent);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
    padding: 4px 0;
  }
  .inline {
    display: flex;
    gap: 8px;
    align-items: center;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .seg {
    display: inline-flex;
    gap: 6px;
  }
  .seg label {
    padding: 7px 16px;
    border: 1px solid var(--border-default);
    border-radius: 99px;
    cursor: pointer;
    font-size: 13px;
  }
  .seg label.on {
    background: var(--surface-accent);
    border-color: var(--border-accent);
  }
  .seg input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  .row {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
  }
  .narrow {
    width: 110px;
  }
  .grow {
    flex: 1;
    min-width: 160px;
  }
  .problem {
    margin: 0;
    display: flex;
    gap: 6px;
    align-items: center;
    font-size: 13px;
    color: #e3a23c;
  }
</style>
