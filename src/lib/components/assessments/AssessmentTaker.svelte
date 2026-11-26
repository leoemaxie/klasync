<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import {
    Flag,
    CloudOff,
    Cloud,
    CheckCircle2,
    Clock,
    ShieldAlert,
    ArrowLeft,
    ArrowRight,
    Maximize,
  } from '@lucide/svelte';
  import {
    startAttempt,
    syncAttempt,
    type AnswerResponse,
    type StudentAssessment,
    type StudentQuestion,
  } from '$lib/api/assessments';
  import {
    AttemptEngine,
    isNetworkError,
    type EngineState,
  } from '$lib/assessments/attemptEngine';
  import { watchIntegrity } from '$lib/assessments/integrityWatch';
  import { attemptStore } from '$lib/storage/attemptStore';
  import { renderMarkdown } from '$lib/utils/markdown';
  import Button from '$lib/components/ui/Button.svelte';
  import Spinner from '$lib/components/ui/Spinner.svelte';

  let {
    assessment,
    onExit,
  }: {
    assessment: StudentAssessment;
    /** `refresh` asks the list to reload (an attempt started or finished). */
    onExit: (refresh?: boolean) => void;
  } = $props();

  type Stage = 'rules' | 'starting' | 'taking' | 'error';
  let stage = $state<Stage>('rules');
  let errorMessage = $state('');
  let engine = $state<AttemptEngine | null>(null);
  let es = $state<EngineState | null>(null);
  let responses = $state<Record<string, AnswerResponse>>({});
  let flags = $state<Record<string, boolean>>({});
  let current = $state(0);
  let remaining = $state<number | null>(null);
  let confirming = $state(false);
  let pasteNotice = $state(false);
  let fullscreenOk = $state(true);
  let resumedOffline = $state(false);

  let area = $state<HTMLElement | null>(null);
  let tick: ReturnType<typeof setInterval> | undefined;
  let unsubscribe: (() => void) | undefined;
  let stopWatching: (() => void) | undefined;
  const touchedAt: Record<string, number> = {};

  const rules = $derived(assessment.rules);
  const questions = $derived<StudentQuestion[]>(engine?.view.questions ?? []);
  const settings = $derived(engine?.view.settings ?? rules);
  const unanswered = $derived(
    questions.filter((q) => !isAnswered(q, responses[q.id])).length
  );
  const flaggedCount = $derived(Object.values(flags).filter(Boolean).length);
  const finished = $derived(
    es?.phase === 'submitted' || es?.phase === 'submitted_local'
  );

  function deviceId(): string {
    try {
      let id = localStorage.getItem('klasync_device_id');
      if (!id) {
        id = crypto.randomUUID();
        localStorage.setItem('klasync_device_id', id);
      }
      return id;
    } catch {
      return 'device';
    }
  }

  const engineDeps = () => ({
    store: attemptStore,
    sync: syncAttempt,
    deviceId: deviceId(),
  });

  function isAnswered(q: StudentQuestion, r: AnswerResponse | undefined) {
    if (!r) return false;
    if (q.question_type === 'true_false') return typeof r.value === 'boolean';
    if (q.question_type === 'mcq_single' || q.question_type === 'mcq_multi') {
      return (r.selected?.length ?? 0) > 0;
    }
    return (r.text ?? '').trim().length > 0;
  }

  function attach(e: AttemptEngine) {
    engine = e;
    for (const q of e.view.questions) {
      const saved = e.answerFor(q.id);
      if (saved) {
        responses[q.id] = saved.response;
        flags[q.id] = saved.flagged;
      }
    }
    unsubscribe = e.subscribe((s) => (es = s));
    stage = 'taking';
    updateRemaining();
    tick = setInterval(updateRemaining, 1000);
    startWatching();
  }

  function startWatching() {
    stopWatching?.();
    if (!engine) return;
    const s = engine.view.settings;
    fullscreenOk = !s.require_fullscreen || !!document.fullscreenElement;
    stopWatching = watchIntegrity(engine, {
      settings: s,
      area,
      onBlockedPaste: () => {
        pasteNotice = true;
        setTimeout(() => (pasteNotice = false), 3500);
      },
    });
  }

  $effect(() => {
    // Re-attach paste handlers once the answer area exists.
    if (area && engine) startWatching();
  });

  function updateRemaining() {
    if (!engine) return;
    remaining = engine.remainingMs();
    void engine.autoSubmitIfExpired();
  }

  async function begin() {
    stage = 'starting';
    errorMessage = '';
    try {
      const view = await startAttempt(assessment.id, deviceId());
      attach(await AttemptEngine.open(view, engineDeps()));
    } catch (error) {
      // No connection but an attempt is saved on this device: carry on from it.
      const id = assessment.in_progress_attempt_id;
      if (id && isNetworkError(error)) {
        const restored = await AttemptEngine.restore(id, engineDeps());
        if (restored) {
          resumedOffline = true;
          attach(restored);
          return;
        }
      }
      stage = 'error';
      errorMessage = isNetworkError(error)
        ? 'You need a connection to start. Once started, you can keep working offline.'
        : error instanceof Error
          ? error.message
          : 'Could not start the assessment';
    }
  }

  onMount(async () => {
    // An attempt already under way and saved here can be reopened offline.
    const id = assessment.in_progress_attempt_id;
    if (id && !navigator.onLine) {
      const restored = await AttemptEngine.restore(id, engineDeps());
      if (restored) {
        resumedOffline = true;
        attach(restored);
      }
    }
  });

  onDestroy(() => {
    clearInterval(tick);
    unsubscribe?.();
    stopWatching?.();
    engine?.dispose();
  });

  async function answer(q: StudentQuestion, response: AnswerResponse) {
    if (!engine || es?.phase !== 'in_progress') return;
    responses[q.id] = response;
    const now = Date.now();
    const spent = Math.min(120_000, now - (touchedAt[q.id] ?? now));
    touchedAt[q.id] = now;
    await engine.setAnswer(q.id, response, spent);
  }

  function touch(q: StudentQuestion) {
    touchedAt[q.id] ??= Date.now();
  }

  function toggleMulti(q: StudentQuestion, id: string) {
    const selected = new Set(responses[q.id]?.selected ?? []);
    if (selected.has(id)) selected.delete(id);
    else selected.add(id);
    void answer(q, { selected: [...selected].sort() });
  }

  async function toggleFlag(q: StudentQuestion) {
    flags[q.id] = !flags[q.id];
    await engine?.setFlag(q.id, flags[q.id]);
  }

  async function submit() {
    confirming = false;
    await engine?.submit();
  }

  async function enterFullscreen() {
    try {
      await document.documentElement.requestFullscreen();
      fullscreenOk = true;
    } catch {
      fullscreenOk = true; // not available here: do not lock the student out
    }
  }

  function words(text: string | undefined) {
    return (text ?? '').trim().split(/\s+/).filter(Boolean).length;
  }

  function clock(ms: number | null) {
    if (ms === null) return '';
    const total = Math.max(0, Math.ceil(ms / 1000));
    const h = Math.floor(total / 3600);
    const m = Math.floor((total % 3600) / 60);
    const s = total % 60;
    const pad = (n: number) => String(n).padStart(2, '0');
    return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`;
  }

  function limitText(secs: number) {
    if (!secs) return 'No time limit';
    const m = Math.round(secs / 60);
    return m >= 60
      ? `${Math.floor(m / 60)} h ${m % 60 ? `${m % 60} min` : ''}`.trim()
      : `${m} minutes`;
  }

  function when(iso: string | null) {
    return iso
      ? new Date(iso).toLocaleString(undefined, {
          dateStyle: 'medium',
          timeStyle: 'short',
        })
      : null;
  }

  const saveLabel = $derived.by(() => {
    switch (es?.save) {
      case 'synced':
        return 'All answers saved';
      case 'saving':
        return 'Saving…';
      case 'offline':
        return 'Offline — saved on this device';
      case 'error':
        return 'Saved on this device — retrying';
      default:
        return '';
    }
  });
</script>

<svelte:window
  ononline={() => engine?.setOnline(true)}
  onoffline={() => engine?.setOnline(false)}
  onbeforeunload={(e) => {
    if (es && es.phase === 'in_progress' && es.pending > 0) e.preventDefault();
  }}
/>

<div class="taker as-scope" style="--ok:#5fae7a;--bad:#d9715c;--warn:#e3a23c;">
  {#if stage === 'rules' || stage === 'starting' || stage === 'error'}
    <div class="surface rules">
      <button class="back" type="button" onclick={() => onExit()}>
        <ArrowLeft size={15} /> Assessments
      </button>
      <p class="eyebrow-line">
        {assessment.course_code} · {assessment.kind}
      </p>
      <h2>{assessment.title}</h2>
      <p class="sub">From “{assessment.lecture_title}”</p>

      {#if assessment.instructions_md}
        <div class="md">{@html renderMarkdown(assessment.instructions_md)}</div>
      {/if}

      <dl class="facts">
        <div>
          <dt>Time</dt>
          <dd>{limitText(assessment.time_limit_secs)}</dd>
        </div>
        <div>
          <dt>Attempts</dt>
          <dd>
            {assessment.attempts_used} of {assessment.max_attempts} used
          </dd>
        </div>
        {#if assessment.due_at}
          <div>
            <dt>Due</dt>
            <dd>{when(assessment.due_at)}</dd>
          </div>
        {/if}
        <div>
          <dt>Questions</dt>
          <dd>
            {rules.one_per_page
              ? 'One at a time'
              : 'All on one page'}{rules.allow_backtracking
              ? ''
              : ', no going back'}
          </dd>
        </div>
      </dl>

      <ul class="rule-list">
        <li>
          <Cloud size={15} />
          Your answers save on this device as you go. If your connection drops, keep
          working: they are sent when you are back online.
        </li>
        {#if assessment.integrity !== 'open'}
          <li>
            <ShieldAlert size={15} />
            This is
            {assessment.integrity === 'strict' ? 'a strict' : 'a monitored'}
            assessment. Leaving the page, switching tabs
            {rules.block_paste ? ', pasting' : ''}
            {rules.require_fullscreen ? ' and leaving fullscreen' : ''} are recorded
            for your lecturer to review. Nothing is penalised automatically.
          </li>
        {/if}
        {#if assessment.time_limit_secs}
          <li>
            <Clock size={15} />
            The timer is kept by the server, so changing your device clock makes no
            difference. The attempt submits itself when time is up.
          </li>
        {/if}
      </ul>

      {#if stage === 'error'}
        <p class="oops" role="alert">{errorMessage}</p>
      {/if}

      <div class="actions">
        <Button variant="ghost" onclick={() => onExit()}>Not now</Button>
        <Button
          variant="accent"
          disabled={stage === 'starting' || !assessment.can_start}
          onclick={begin}
        >
          {#if stage === 'starting'}<Spinner />{/if}
          {assessment.in_progress_attempt_id ? 'Continue' : 'Start'}
        </Button>
      </div>
      {#if !assessment.can_start}
        <p class="muted">This assessment can't be started right now.</p>
      {/if}
    </div>
  {:else if engine && es}
    {#if finished}
      <div class="surface done">
        <span class="done-icon" data-state={es.phase}>
          {#if es.phase === 'submitted'}<CheckCircle2
              size={34}
            />{:else}<CloudOff size={34} />{/if}
        </span>
        {#if es.phase === 'submitted'}
          <h2>Submitted</h2>
          <p>
            {#if es.autoSubmitted}Your time ran out, so your answers were
              submitted for you.{:else}Your answers are in. Well done.{/if}
          </p>
          {#if es.receipt}
            <p class="receipt">Receipt <code>{es.receipt}</code></p>
          {/if}
          {#if es.flagged}
            <p class="muted">
              This was sent after the deadline from an offline device, so your
              lecturer will see it was submitted late from offline.
            </p>
          {/if}
        {:else}
          <h2>Saved on this device</h2>
          <p>
            You're offline. Your submission is stored here and will be sent
            automatically as soon as you reconnect. Keep this page or the app
            open, or come back to it later.
          </p>
          <p class="muted">
            {es.pending} item{es.pending === 1 ? '' : 's'} waiting to send.
            {#if es.lastError}{es.lastError}{/if}
          </p>
          <Button variant="soft" onclick={() => engine?.flush()}
            >Try to send now</Button
          >
        {/if}
        <Button variant="accent" onclick={() => onExit(true)}
          >Back to assessments</Button
        >
      </div>
    {:else}
      <header class="topbar">
        <div class="bar-left">
          <strong>{engine.view.title}</strong>
          <span class="muted">Attempt {engine.view.attempt_no}</span>
        </div>
        <div class="bar-right">
          <span
            class="save"
            data-save={es.save}
            role="status"
            aria-live="polite"
          >
            {#if es.online}<Cloud size={14} />{:else}<CloudOff size={14} />{/if}
            {saveLabel}
          </span>
          {#if remaining !== null}
            <span
              class="timer"
              class:low={remaining < 300_000}
              class:critical={remaining < 60_000}
              role="timer"
              aria-label="Time remaining"
            >
              <Clock size={14} />
              {clock(remaining)}
            </span>
          {/if}
        </div>
      </header>

      {#if resumedOffline}
        <p class="banner">
          Continuing from this device while offline. Your earlier answers are
          here.
        </p>
      {/if}
      {#if !es.online && !resumedOffline}
        <p class="banner" role="status">
          You're offline. Keep answering; everything is saved on this device and
          sent when you reconnect.
        </p>
      {/if}
      {#if pasteNotice}
        <p class="banner warn" role="alert">
          Pasting is turned off for this assessment.
        </p>
      {/if}

      {#if settings.require_fullscreen && !fullscreenOk}
        <div class="surface gate">
          <Maximize size={22} />
          <h3>Fullscreen is required</h3>
          <p>
            Your lecturer asked for this assessment to be taken in fullscreen.
          </p>
          <Button variant="accent" onclick={enterFullscreen}
            >Enter fullscreen</Button
          >
        </div>
      {:else}
        <div class="layout">
          {#if settings.allow_backtracking}
            <nav class="nav" aria-label="Questions">
              {#each questions as q, i (q.id)}
                <button
                  type="button"
                  class="dot"
                  class:on={settings.one_per_page && i === current}
                  class:answered={isAnswered(q, responses[q.id])}
                  class:flagged={flags[q.id]}
                  aria-label={`Question ${i + 1}${isAnswered(q, responses[q.id]) ? ', answered' : ''}${flags[q.id] ? ', flagged' : ''}`}
                  onclick={() => {
                    current = i;
                    if (!settings.one_per_page)
                      document.getElementById(`q-${q.id}`)?.scrollIntoView({
                        behavior: 'smooth',
                        block: 'start',
                      });
                  }}>{i + 1}</button
                >
              {/each}
            </nav>
          {/if}

          <div class="questions" bind:this={area}>
            {#each questions as q, i (q.id)}
              {#if !settings.one_per_page || i === current}
                <fieldset
                  class="q surface"
                  id={`q-${q.id}`}
                  onfocusin={() => touch(q)}
                >
                  <legend>
                    <span class="num"
                      >Question {i + 1} of {questions.length}</span
                    >
                    <span class="pts"
                      >{q.points} point{q.points === 1 ? '' : 's'}</span
                    >
                  </legend>
                  <div class="prompt md">
                    {@html renderMarkdown(q.prompt_md)}
                  </div>

                  {#if q.question_type === 'mcq_single' || q.question_type === 'true_false'}
                    <div
                      class="options"
                      role="radiogroup"
                      aria-label="Choose one"
                    >
                      {#each q.options as option (option.id)}
                        {@const selected =
                          q.question_type === 'true_false'
                            ? responses[q.id]?.value ===
                                (option.id === 'true') &&
                              typeof responses[q.id]?.value === 'boolean'
                            : responses[q.id]?.selected?.[0] === option.id}
                        <label class="option" class:selected>
                          <input
                            type="radio"
                            name={`q-${q.id}`}
                            checked={selected}
                            onchange={() =>
                              answer(
                                q,
                                q.question_type === 'true_false'
                                  ? { value: option.id === 'true' }
                                  : { selected: [option.id] }
                              )}
                          />
                          <span>{option.text}</span>
                        </label>
                      {/each}
                    </div>
                  {:else if q.question_type === 'mcq_multi'}
                    <p class="help">Select all that apply.</p>
                    <div class="options">
                      {#each q.options as option (option.id)}
                        {@const selected =
                          responses[q.id]?.selected?.includes(option.id) ??
                          false}
                        <label class="option" class:selected>
                          <input
                            type="checkbox"
                            checked={selected}
                            onchange={() => toggleMulti(q, option.id)}
                          />
                          <span>{option.text}</span>
                        </label>
                      {/each}
                    </div>
                  {:else if q.question_type === 'short_answer'}
                    <input
                      class="inp"
                      type="text"
                      maxlength={q.max_chars ?? 300}
                      value={responses[q.id]?.text ?? ''}
                      placeholder="Your answer"
                      autocomplete="off"
                      oninput={(e) =>
                        answer(q, { text: e.currentTarget.value })}
                    />
                  {:else}
                    <textarea
                      class="inp long"
                      rows="8"
                      value={responses[q.id]?.text ?? ''}
                      placeholder="Write your answer"
                      oninput={(e) =>
                        answer(q, { text: e.currentTarget.value })}></textarea>
                    <p
                      class="help"
                      class:over={q.max_words &&
                        words(responses[q.id]?.text) > q.max_words}
                    >
                      {words(responses[q.id]?.text)} words{q.max_words
                        ? ` · suggested limit ${q.max_words}`
                        : ''}
                    </p>
                  {/if}

                  <div class="q-actions">
                    <button
                      type="button"
                      class="flag"
                      class:on={flags[q.id]}
                      aria-pressed={!!flags[q.id]}
                      onclick={() => toggleFlag(q)}
                    >
                      <Flag size={14} />
                      {flags[q.id] ? 'Flagged for review' : 'Flag for review'}
                    </button>
                  </div>
                </fieldset>
              {/if}
            {/each}

            <div class="pager">
              {#if settings.one_per_page}
                <Button
                  variant="outline"
                  disabled={current === 0 || !settings.allow_backtracking}
                  onclick={() => (current = Math.max(0, current - 1))}
                  ><ArrowLeft size={15} /> Previous</Button
                >
                {#if current < questions.length - 1}
                  <Button
                    variant="outline"
                    onclick={() => (current = current + 1)}
                    >Next <ArrowRight size={15} /></Button
                  >
                {/if}
              {/if}
              <span class="spacer"></span>
              <Button variant="accent" onclick={() => (confirming = true)}
                >Submit</Button
              >
            </div>
          </div>
        </div>
      {/if}

      {#if confirming}
        <div class="scrim" role="presentation">
          <div
            class="dialog surface"
            role="alertdialog"
            aria-modal="true"
            aria-labelledby="sub-title"
          >
            <h3 id="sub-title">Submit your answers?</h3>
            <p>
              {#if unanswered > 0}
                You have {unanswered} unanswered question{unanswered === 1
                  ? ''
                  : 's'}.
              {:else}
                You have answered every question.
              {/if}
              {#if flaggedCount > 0}
                {flaggedCount} flagged for review.
              {/if}
              You can't change them after submitting.
            </p>
            <div class="actions">
              <Button variant="ghost" onclick={() => (confirming = false)}
                >Keep working</Button
              >
              <Button variant="accent" onclick={submit}>Submit now</Button>
            </div>
          </div>
        </div>
      {/if}
    {/if}
  {/if}
</div>

<style>
  .taker {
    display: flex;
    flex-direction: column;
    gap: 14px;
    max-width: 980px;
    margin: 0 auto;
    width: 100%;
  }
  .surface {
    background: var(--surface-2);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    padding: 20px;
  }
  .rules {
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
  .eyebrow-line {
    margin: 0;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--text-accent);
  }
  h2 {
    margin: 0;
    font-size: 26px;
    font-weight: 500;
  }
  .sub,
  .muted {
    margin: 0;
    color: var(--text-tertiary);
    font-size: 13px;
  }
  .facts {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    gap: 10px;
    margin: 0;
  }
  .facts div {
    background: var(--surface-inset);
    border-radius: var(--radius-md);
    padding: 10px 12px;
  }
  .facts dt {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-tertiary);
  }
  .facts dd {
    margin: 2px 0 0;
    font-size: 14px;
  }
  .rule-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 9px;
    font-size: 13.5px;
    color: var(--text-secondary);
  }
  .rule-list li {
    display: flex;
    gap: 9px;
    align-items: flex-start;
  }
  .rule-list :global(svg) {
    flex: none;
    margin-top: 3px;
    color: var(--text-accent);
  }
  .oops {
    color: var(--bad);
    margin: 0;
  }
  .actions {
    display: flex;
    gap: 10px;
    justify-content: flex-end;
    flex-wrap: wrap;
  }
  .topbar {
    position: sticky;
    top: calc(var(--app-top-offset, 0px) + 4px);
    z-index: 5;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
    padding: 10px 14px;
    border-radius: var(--radius-lg);
    background: var(--surface-3);
    border: 1px solid var(--border-default);
  }
  .bar-left {
    display: flex;
    gap: 10px;
    align-items: baseline;
  }
  .bar-right {
    display: flex;
    gap: 10px;
    align-items: center;
  }
  .save,
  .timer {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
    padding: 4px 10px;
    border-radius: 999px;
    background: var(--surface-inset);
    color: var(--text-secondary);
  }
  .save[data-save='offline'],
  .save[data-save='error'] {
    color: var(--warn);
  }
  .save[data-save='synced'] {
    color: var(--ok);
  }
  .timer {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    color: var(--text-primary);
  }
  .timer.low {
    color: var(--warn);
  }
  .timer.critical {
    color: var(--bad);
  }
  .banner {
    margin: 0;
    padding: 9px 14px;
    border-radius: var(--radius-md);
    background: var(--surface-accent);
    font-size: 13px;
    color: var(--text-secondary);
  }
  .banner.warn {
    color: var(--warn);
  }
  .layout {
    display: grid;
    grid-template-columns: 56px 1fr;
    gap: 14px;
    align-items: start;
  }
  .layout:has(.nav:empty),
  .layout:not(:has(.nav)) {
    grid-template-columns: 1fr;
  }
  .nav {
    position: sticky;
    top: 80px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .dot {
    width: 40px;
    height: 40px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-default);
    background: var(--surface-2);
    color: var(--text-secondary);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .dot.answered {
    border-color: var(--ok);
    color: var(--text-primary);
  }
  .dot.flagged {
    box-shadow: inset 0 -3px 0 var(--warn);
  }
  .dot.on {
    background: var(--fill-accent);
    color: var(--text-on-accent);
    border-color: transparent;
  }
  .questions {
    display: flex;
    flex-direction: column;
    gap: 14px;
    min-width: 0;
  }
  fieldset.q {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-inline-size: 0;
  }
  legend {
    display: flex;
    gap: 12px;
    align-items: baseline;
    padding: 0 6px;
    font-size: 12px;
    color: var(--text-tertiary);
  }
  .num {
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-accent);
  }
  .prompt {
    font-size: 16px;
    line-height: 1.55;
  }
  .options {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .option {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    padding: 12px 14px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-default);
    background: var(--surface-inset);
    cursor: pointer;
    min-height: 44px;
  }
  .option.selected {
    border-color: var(--border-accent);
    background: var(--surface-accent);
  }
  .option input {
    margin-top: 3px;
    accent-color: var(--fill-accent);
  }
  .inp {
    width: 100%;
    padding: 11px 13px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-default);
    background: var(--surface-inset);
    color: var(--text-primary);
    font: inherit;
    font-size: 15px;
    text-transform: none;
    letter-spacing: 0;
  }
  .inp.long {
    resize: vertical;
    min-height: 160px;
    line-height: 1.5;
  }
  .inp:focus {
    outline: none;
    border-color: var(--border-strong);
    box-shadow: var(--focus-ring);
  }
  .help {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-tertiary);
  }
  .help.over {
    color: var(--warn);
  }
  .q-actions {
    display: flex;
    justify-content: flex-end;
  }
  .flag {
    display: inline-flex;
    gap: 6px;
    align-items: center;
    background: none;
    border: 0;
    color: var(--text-tertiary);
    font: inherit;
    font-size: 12.5px;
    cursor: pointer;
    padding: 6px 8px;
    border-radius: var(--radius-sm);
  }
  .flag.on {
    color: var(--warn);
  }
  .pager {
    display: flex;
    gap: 10px;
    align-items: center;
    flex-wrap: wrap;
  }
  .spacer {
    flex: 1;
  }
  .gate,
  .done {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    text-align: center;
    padding: 36px 20px;
  }
  .done p {
    margin: 0;
    max-width: 52ch;
    color: var(--text-secondary);
  }
  .done-icon[data-state='submitted'] {
    color: var(--ok);
  }
  .done-icon[data-state='submitted_local'] {
    color: var(--warn);
  }
  .receipt code {
    font-size: 15px;
    letter-spacing: 0.08em;
  }
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 60;
    background: rgb(0 0 0 / 0.55);
    display: grid;
    place-items: center;
    padding: 16px;
  }
  .dialog {
    max-width: 420px;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .dialog h3,
  .dialog p {
    margin: 0;
  }
  @media (max-width: 640px) {
    .layout {
      grid-template-columns: 1fr;
    }
    .nav {
      position: static;
      flex-direction: row;
      flex-wrap: wrap;
    }
  }
</style>
