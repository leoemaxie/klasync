<script lang="ts">
  import {
    patchAssessment,
    type Assessment,
    type AssessmentSettings,
  } from '$lib/api/assessments';
  import { fromLocalInput, toLocalInput } from '$lib/assessments/questionRules';
  import Button from '$lib/components/ui/Button.svelte';

  let {
    assessment,
    settings,
    editable = true,
    onSaved,
  }: {
    assessment: Assessment;
    settings: AssessmentSettings;
    editable?: boolean;
    onSaved: () => void;
  } = $props();

  // A working copy: the form edits this, never the props. The parent re-creates
  // the panel (`{#key}`) when the server's revision changes.
  const initial = () => ({
    s: structuredClone($state.snapshot(settings)),
    title: assessment.title,
    instructions: assessment.instructions_md,
    opens: toLocalInput(assessment.opens_at),
    due: toLocalInput(assessment.due_at),
    closes: toLocalInput(assessment.closes_at),
    // Durations are typed in friendlier units than the stored seconds.
    limitMin: Math.round(settings.time_limit_secs / 60),
    cooldownMin: Math.round(settings.cooldown_secs / 60),
    gapMin: Math.round(settings.max_offline_gap_secs / 60),
  });
  const start = initial();
  let s = $state<AssessmentSettings>(start.s);
  let title = $state(start.title);
  let instructions = $state(start.instructions);
  let opens = $state(start.opens);
  let due = $state(start.due);
  let closes = $state(start.closes);
  let limitMin = $state(start.limitMin);
  let cooldownMin = $state(start.cooldownMin);
  let gapMin = $state(start.gapMin);
  let saving = $state(false);
  let message = $state('');
  let ok = $state(false);

  const strict = $derived(s.integrity === 'strict');

  async function save() {
    saving = true;
    message = '';
    ok = false;
    try {
      await patchAssessment(assessment.id, assessment.draft_rev, {
        title,
        instructions_md: instructions,
        settings: {
          ...s,
          time_limit_secs: Math.max(0, Math.round(limitMin * 60)),
          cooldown_secs: Math.max(0, Math.round(cooldownMin * 60)),
          max_offline_gap_secs: Math.max(0, Math.round(gapMin * 60)),
        },
        opens_at: fromLocalInput(opens),
        due_at: fromLocalInput(due),
        closes_at: fromLocalInput(closes),
      });
      // The parent confirms: this panel is rebuilt on save, which would erase its own message.
      onSaved();
    } catch (e) {
      message = e instanceof Error ? e.message : 'Could not save';
    } finally {
      saving = false;
    }
  }
</script>

<form
  class="settings as-scope"
  onsubmit={(e) => {
    e.preventDefault();
    void save();
  }}
>
  <fieldset disabled={!editable}>
    <legend>Basics</legend>
    <label class="wide">
      <span>Title</span><input type="text" bind:value={title} />
    </label>
    <label class="wide">
      <span>
        Instructions <em>shown on the rules screen; Markdown works</em>
      </span>
      <textarea rows="3" bind:value={instructions}></textarea>
    </label>
  </fieldset>

  <fieldset disabled={!editable}>
    <legend>Availability</legend>
    <div class="grid">
      <label>
        <span>Opens</span><input type="datetime-local" bind:value={opens} />
      </label>
      <label>
        <span>Due</span><input type="datetime-local" bind:value={due} />
      </label>
      <label>
        <span>Closes <em>hard stop</em></span><input
          type="datetime-local"
          bind:value={closes}
        />
      </label>
    </div>
    <div class="grid">
      <label>
        <span>Late work</span>
        <select bind:value={s.late_policy}>
          <option value="none">Accept, no mark</option>
          <option value="flag">Accept and flag it</option>
          <option value="penalty">Deduct a penalty per day</option>
          <option value="reject">Reject after the due time</option>
        </select>
      </label>
      {#if s.late_policy === 'penalty'}
        <label>
          <span>Penalty, % per day</span><input
            type="number"
            min="0"
            max="100"
            bind:value={s.late_penalty_pct_per_day}
          />
        </label>
      {/if}
      <label>
        <span>Grace for sending, seconds</span><input
          type="number"
          min="0"
          max="600"
          bind:value={s.grace_secs}
        />
      </label>
    </div>
  </fieldset>

  <fieldset disabled={!editable}>
    <legend>Attempts and timing</legend>
    <div class="grid">
      <label>
        <span>Attempts allowed</span><input
          type="number"
          min="1"
          max="10"
          bind:value={s.max_attempts}
        />
      </label>
      <label>
        <span>The score that counts</span>
        <select bind:value={s.keep}>
          <option value="best">Best attempt</option>
          <option value="last">Last attempt</option>
          <option value="average">Average</option>
        </select>
      </label>
      <label>
        <span>Wait between attempts, minutes</span><input
          type="number"
          min="0"
          bind:value={cooldownMin}
        />
      </label>
      <label>
        <span>Time limit, minutes <em>0 = none</em></span><input
          type="number"
          min="0"
          max="1440"
          bind:value={limitMin}
        />
      </label>
    </div>
  </fieldset>

  <fieldset disabled={!editable}>
    <legend>Delivery</legend>
    <label class="check">
      <input type="checkbox" bind:checked={s.shuffle_questions} /> Shuffle question
      order
    </label>
    <label class="check">
      <input type="checkbox" bind:checked={s.shuffle_options} /> Shuffle answer options
    </label>
    <label class="check">
      <input
        type="checkbox"
        disabled={strict}
        checked={strict || s.one_per_page}
        onchange={(e) => (s.one_per_page = e.currentTarget.checked)}
      /> One question per page
    </label>
    <label class="check">
      <input
        type="checkbox"
        disabled={strict}
        checked={!strict && s.allow_backtracking}
        onchange={(e) => (s.allow_backtracking = e.currentTarget.checked)}
      /> Let students go back to earlier questions
    </label>
    <label class="check">
      <input type="checkbox" bind:checked={s.negative_marking} /> Negative marking
      for wrong single-choice answers
    </label>
  </fieldset>

  <fieldset disabled={!editable}>
    <legend>Integrity</legend>
    <p class="note">
      A browser can't stop cheating, so this records signals for you to review.
      Nothing is penalised automatically.
    </p>
    <div class="levels" role="radiogroup" aria-label="Integrity level">
      {#each [['open', 'Open', 'No monitoring.'], ['monitored', 'Monitored', 'Records leaving the page, pasting and going offline.'], ['strict', 'Strict', 'Monitored, plus fullscreen, no pasting, one question at a time, no going back.']] as [value, label, hint] (value)}
        <label class="level" class:on={s.integrity === value}>
          <input
            type="radio"
            name="integrity"
            {value}
            bind:group={s.integrity}
          />
          <strong>{label}</strong><span>{hint}</span>
        </label>
      {/each}
    </div>
    {#if s.integrity !== 'open'}
      <label class="check">
        <input
          type="checkbox"
          disabled={strict}
          checked={strict || s.require_fullscreen}
          onchange={(e) => (s.require_fullscreen = e.currentTarget.checked)}
        /> Require fullscreen
      </label>
      <label class="check">
        <input
          type="checkbox"
          disabled={strict}
          checked={strict || s.block_paste}
          onchange={(e) => (s.block_paste = e.currentTarget.checked)}
        /> Block pasting into answers
      </label>
      <label class="inline">
        <span>Auto-submit after this many violations <em>0 = never</em></span>
        <input
          type="number"
          min="0"
          bind:value={s.auto_submit_after_violations}
        />
      </label>
    {/if}
  </fieldset>

  <fieldset disabled={!editable}>
    <legend>Offline</legend>
    <label class="check">
      <input type="checkbox" bind:checked={s.allow_offline_continue} /> Let students
      keep working and submit if their connection drops
    </label>
    <label class="check">
      <input
        type="checkbox"
        bind:checked={s.allow_offline_start}
        disabled={!s.allow_offline_continue}
      />
      Allow starting offline <em>weakens the time guarantee</em>
    </label>
    <label class="inline">
      <span>Flag offline gaps longer than, minutes <em>0 = never</em></span>
      <input type="number" min="0" bind:value={gapMin} />
    </label>
  </fieldset>

  <fieldset disabled={!editable}>
    <legend>Marking</legend>
    <div class="levels" role="radiogroup" aria-label="Scoring mode">
      {#each [['manual', 'You mark', 'You score every written answer yourself.'], ['ai_confirm', 'AI proposes, you confirm', 'AI suggests a score and feedback; nothing counts until you confirm it.'], ['ai_auto', 'AI marks', 'The AI score is final unless you override. Doubtful answers still wait for you.']] as [value, label, hint] (value)}
        <label class="level" class:on={s.scoring_mode === value}>
          <input
            type="radio"
            name="scoring"
            {value}
            bind:group={s.scoring_mode}
          />
          <strong>{label}</strong><span>{hint}</span>
        </label>
      {/each}
    </div>
    <p class="note">
      Choice and true/false questions are always scored automatically. You grade
      blind: names stay hidden until you finalise.
    </p>
    <label class="check">
      <input type="checkbox" bind:checked={s.ai_short_answer} /> Let AI mark short
      answers that don't match the accepted list
    </label>
  </fieldset>

  <fieldset disabled={!editable}>
    <legend>Results</legend>
    <div class="grid">
      <label>
        <span>Release results</span>
        <select bind:value={s.release}>
          <option value="immediate">Straight after submitting</option>
          <option value="after_due">After the due time</option>
          <option value="after_finalize">After you finalise</option>
          <option value="manual">When I release them</option>
        </select>
      </label>
      <label>
        <span>What students see</span>
        <select bind:value={s.visibility}>
          <option value="score">Score only</option>
          <option value="correctness">Score and which were right</option>
          <option value="answers">Plus the correct answers</option>
          <option value="feedback">Plus feedback and explanations</option>
        </select>
      </label>
    </div>
  </fieldset>

  <div class="savebar">
    {#if message}<span class:ok role="status">{message}</span>{/if}
    <Button variant="accent" type="submit" disabled={!editable || saving}>
      {saving ? 'Saving…' : 'Save settings'}
    </Button>
  </div>
</form>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  fieldset {
    margin: 0;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    background: var(--surface-2);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
  }
  legend {
    padding: 0 8px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--text-accent);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(190px, 1fr));
    gap: 12px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
  }
  label.check {
    flex-direction: row;
    align-items: center;
    gap: 9px;
    font-weight: 400;
    font-size: 13.5px;
  }
  label.inline {
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    font-weight: 400;
    font-size: 13.5px;
  }
  label.inline input {
    width: 90px;
  }
  em {
    font-style: normal;
    font-weight: 400;
    color: var(--text-tertiary);
    margin-left: 6px;
  }
  input[type='text'],
  input[type='number'],
  input[type='datetime-local'],
  select,
  textarea {
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
  input:focus,
  select:focus,
  textarea:focus {
    outline: none;
    border-color: var(--border-strong);
    box-shadow: var(--focus-ring);
  }
  .note {
    margin: 0;
    font-size: 13px;
    color: var(--text-tertiary);
  }
  .levels {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(210px, 1fr));
    gap: 10px;
  }
  .level {
    gap: 3px;
    padding: 12px 14px;
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
    cursor: pointer;
    font-weight: 400;
  }
  .level.on {
    border-color: var(--border-accent);
    background: var(--surface-accent);
  }
  .level input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  .level strong {
    font-size: 14px;
    color: var(--text-primary);
  }
  .level span {
    font-size: 12.5px;
    color: var(--text-tertiary);
  }
  .savebar {
    display: flex;
    gap: 14px;
    justify-content: flex-end;
    align-items: center;
  }
  .savebar .ok {
    color: #5fae7a;
    font-size: 13px;
  }
  .savebar span:not(.ok) {
    color: #d9715c;
    font-size: 13px;
  }
</style>
