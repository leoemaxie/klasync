import { describe, expect, it } from 'vitest';
import type {
  AnswerOp,
  AttemptView,
  SyncRequest,
  SyncResponse,
} from '$lib/api/assessments';
import {
  AttemptEngine,
  AttemptTimer,
  MemoryAttemptStore,
  type Clock,
  type EngineDeps,
} from './attemptEngine';

const START = Date.parse('2026-03-01T10:00:00.000Z');

function view(overrides: Partial<AttemptView> = {}): AttemptView {
  return {
    attempt_id: 'att-1',
    assessment_id: 'as-1',
    title: 'Quiz',
    instructions_md: '',
    kind: 'quiz',
    attempt_no: 1,
    status: 'in_progress',
    started_at: new Date(START).toISOString(),
    deadline_at: new Date(START + 600_000).toISOString(),
    server_now: new Date(START).toISOString(),
    settings: {
      time_limit_secs: 600,
      grace_secs: 30,
      one_per_page: false,
      allow_backtracking: true,
      require_fullscreen: false,
      block_paste: false,
      records_integrity: false,
      allow_offline_continue: true,
      allow_offline_start: false,
      max_offline_gap_secs: 0,
      auto_submit_after_violations: 0,
      max_attempts: 1,
    },
    questions: [],
    answers: [],
    receipt: null,
    submitted_at: null,
    auto_submitted: false,
    ...overrides,
  };
}

/** A manual clock: `wall` can be wound back by the "user", `mono` cannot. */
class FakeClock implements Clock {
  wallMs = START;
  monoMs = 1000;
  wall() {
    return this.wallMs;
  }
  mono() {
    return this.monoMs;
  }
  advance(ms: number) {
    this.wallMs += ms;
    this.monoMs += ms;
  }
}

/** Behaves like the real server: ops are applied once per op id. */
class FakeServer {
  online = true;
  /** Apply the request, then lose the response (the client sees a network error). */
  dropResponses = false;
  applied = new Map<string, AnswerOp>();
  answers = new Map<string, { response: unknown; revision: number }>();
  requests: SyncRequest[] = [];
  submitted = false;
  /** Make the server answer a submit without confirming it. */
  refuseToConfirm = false;
  status: SyncResponse['status'] = 'in_progress';
  clock: FakeClock;

  constructor(clock: FakeClock) {
    this.clock = clock;
  }

  sync = async (_id: string, request: SyncRequest): Promise<SyncResponse> => {
    if (!this.online) throw new TypeError('Failed to fetch');
    this.requests.push(structuredClone(request));
    let applied = 0;
    let duplicates = 0;
    for (const op of request.ops ?? []) {
      if (this.applied.has(op.op_id)) {
        duplicates++;
        continue;
      }
      this.applied.set(op.op_id, op);
      const existing = this.answers.get(op.question_id);
      if (!existing || existing.revision <= op.revision) {
        this.answers.set(op.question_id, {
          response: op.response,
          revision: op.revision,
        });
      }
      applied++;
    }
    if (request.submit && !this.refuseToConfirm) {
      this.submitted = true;
      this.status = 'submitted';
    }
    const response: SyncResponse = {
      applied,
      duplicates,
      rejected: 0,
      status: this.status,
      deadline_at: new Date(START + 600_000).toISOString(),
      server_now: new Date(this.clock.wall()).toISOString(),
      receipt: this.submitted ? 'ABCD-1234-EF56' : null,
      flagged: false,
      auto_submitted: false,
    };
    if (this.dropResponses) throw new TypeError('Failed to fetch');
    return response;
  };
}

function setup(
  opts: { settings?: Partial<AttemptView['settings']>; batchSize?: number } = {}
) {
  const clock = new FakeClock();
  const server = new FakeServer(clock);
  const store = new MemoryAttemptStore();
  const scheduled: { fn: () => void; ms: number; cancelled: boolean }[] = [];
  const deps: EngineDeps = {
    store,
    sync: server.sync,
    clock,
    schedule: (fn, ms) => {
      const entry = { fn, ms, cancelled: false };
      scheduled.push(entry);
      return () => {
        entry.cancelled = true;
      };
    },
    newId: (() => {
      let n = 0;
      return () => `op-${++n}`;
    })(),
    deviceId: 'dev-1',
    batchSize: opts.batchSize ?? 50,
  };
  const v = view();
  if (opts.settings) v.settings = { ...v.settings, ...opts.settings };
  return {
    clock,
    server,
    store,
    scheduled,
    deps,
    v,
    /** Runs the most recent live timer. */
    async runTimer() {
      const next = [...scheduled].reverse().find((s) => !s.cancelled);
      if (next) {
        next.cancelled = true;
        next.fn();
      }
      await new Promise((r) => setTimeout(r, 0));
    },
  };
}

describe('AttemptEngine: nothing is lost', () => {
  it('writes an answer to local storage before any network call', async () => {
    const t = setup();
    t.server.online = false;
    const engine = await AttemptEngine.open(t.v, t.deps);
    await engine.setAnswer('q1', { selected: ['a'] });

    expect(t.server.requests).toHaveLength(0);
    const stored = await t.store.load('att-1');
    expect(stored?.answers.q1.response).toEqual({ selected: ['a'] });
    expect(stored?.pendingOps).toHaveLength(1);
    expect(engine.state.pending).toBe(1);
    expect(engine.state.save).toBe('saving');
  });

  it('sends changes and clears the queue when online', async () => {
    const t = setup();
    const engine = await AttemptEngine.open(t.v, t.deps);
    await engine.setAnswer('q1', { selected: ['b'] });
    await engine.flush();

    expect(t.server.answers.get('q1')?.response).toEqual({ selected: ['b'] });
    expect(engine.state.pending).toBe(0);
    expect(engine.state.save).toBe('synced');
    expect((await t.store.load('att-1'))?.pendingOps).toHaveLength(0);
  });

  it('coalesces a typing burst into one change with the latest revision', async () => {
    const t = setup();
    const engine = await AttemptEngine.open(t.v, t.deps);
    for (const text of ['H', 'He', 'Hea', 'Heat', 'Heat flux']) {
      await engine.setAnswer('q1', { text });
    }
    const stored = await t.store.load('att-1');
    expect(stored?.pendingOps).toHaveLength(1);
    expect(stored?.pendingOps[0].revision).toBe(5);
    expect(stored?.pendingOps[0].response).toEqual({ text: 'Heat flux' });
  });

  it('keeps work while offline, then delivers it when the connection returns', async () => {
    const t = setup();
    const engine = await AttemptEngine.open(t.v, t.deps);
    t.server.online = false;
    await engine.setAnswer('q1', { selected: ['a'] });
    await engine.setAnswer('q2', { value: true });
    await engine.flush();

    expect(engine.state.online).toBe(false);
    expect(engine.state.save).toBe('offline');
    expect(engine.state.pending).toBe(2);

    t.server.online = true;
    engine.setOnline(true);
    await engine.flush();
    expect(t.server.answers.size).toBe(2);
    expect(engine.state.save).toBe('synced');
    expect(engine.state.online).toBe(true);
  });

  it('is safe when the server got the change but the response was lost', async () => {
    const t = setup();
    const engine = await AttemptEngine.open(t.v, t.deps);
    await engine.setAnswer('q1', { selected: ['a'] });

    t.server.dropResponses = true;
    await engine.flush(); // applied on the server, but the client saw a network error
    expect(t.server.applied.size).toBe(1);
    expect(engine.state.pending).toBe(1);

    t.server.dropResponses = false;
    await engine.flush(); // the same op id is replayed
    const last = t.server.requests.at(-1)!;
    expect(last.ops).toHaveLength(1);
    expect(t.server.applied.size).toBe(1); // applied exactly once
    expect(engine.state.pending).toBe(0);
  });

  it('retries with growing delays while offline', async () => {
    const t = setup();
    const engine = await AttemptEngine.open(t.v, t.deps);
    t.server.online = false;
    await engine.setAnswer('q1', { selected: ['a'] });
    await engine.flush();
    const first = t.scheduled.filter((s) => !s.cancelled).at(-1)!.ms;
    await t.runTimer();
    const second = t.scheduled.filter((s) => !s.cancelled).at(-1)!.ms;
    await t.runTimer();
    const third = t.scheduled.filter((s) => !s.cancelled).at(-1)!.ms;
    expect(first).toBe(1000);
    expect(second).toBe(2000);
    expect(third).toBe(4000);
  });

  it('splits a large backlog into batches', async () => {
    const t = setup({ batchSize: 50 });
    const engine = await AttemptEngine.open(t.v, t.deps);
    for (let i = 0; i < 120; i++)
      await engine.setAnswer(`q${i}`, { selected: ['a'] });
    await engine.flush();
    expect(t.server.requests.map((r) => r.ops?.length)).toEqual([50, 50, 20]);
    expect(t.server.applied.size).toBe(120);
  });

  it('survives a reload: unsent work is merged back in and still delivered', async () => {
    const t = setup();
    const first = await AttemptEngine.open(t.v, t.deps);
    t.server.online = false;
    await first.setAnswer('q1', { selected: ['c'] });
    first.dispose(); // the tab closes

    // The server view is stale (it has not seen the answer); the page reloads.
    const second = await AttemptEngine.open(view(), t.deps);
    expect(second.answerFor('q1')?.response).toEqual({ selected: ['c'] });
    expect(second.state.pending).toBe(1);

    t.server.online = true;
    await second.flush();
    expect(t.server.answers.get('q1')?.response).toEqual({ selected: ['c'] });
    expect(second.state.pending).toBe(0);
  });

  it('can be restored from local storage alone (reload with no network)', async () => {
    const t = setup();
    const first = await AttemptEngine.open(t.v, t.deps);
    t.server.online = false;
    await first.setAnswer('q1', { value: false });
    first.dispose();

    const restored = await AttemptEngine.restore('att-1', t.deps);
    expect(restored).toBeDefined();
    expect(restored!.view.questions).toEqual([]);
    expect(restored!.answerFor('q1')?.response).toEqual({ value: false });
    expect(await AttemptEngine.restore('missing', t.deps)).toBeUndefined();
  });
});

describe('AttemptEngine: submitting', () => {
  it('submits online and issues a receipt', async () => {
    const t = setup();
    const engine = await AttemptEngine.open(t.v, t.deps);
    await engine.setAnswer('q1', { selected: ['a'] });
    await engine.submit();
    expect(engine.state.phase).toBe('submitted');
    expect(engine.state.receipt).toBe('ABCD-1234-EF56');
    // The final answers went in the same request as the submit.
    expect(t.server.requests.at(-1)?.submit).toBeDefined();
    expect(t.server.answers.has('q1')).toBe(true);
  });

  it('queues a submit made offline with the measured elapsed time, then delivers it', async () => {
    const t = setup();
    const engine = await AttemptEngine.open(t.v, t.deps);
    t.server.online = false;
    engine.setOnline(false);
    await engine.setAnswer('q1', { selected: ['a'] });
    t.clock.advance(500_000); // 500 seconds of answering offline
    await engine.submit();

    expect(engine.state.phase).toBe('submitted_local');
    const queued = (await t.store.load('att-1'))?.queuedSubmit;
    expect(queued?.offline).toBe(true);
    expect(queued?.elapsed_ms).toBe(500_000);

    t.clock.advance(300_000); // the connection comes back after the deadline
    t.server.online = true;
    engine.setOnline(true);
    await engine.flush();
    expect(engine.state.phase).toBe('submitted');
    expect(t.server.requests.at(-1)?.submit?.elapsed_ms).toBe(500_000);
    expect(t.server.requests.at(-1)?.submit?.offline).toBe(true);
  });

  it('keeps a queued submit across a reload', async () => {
    const t = setup();
    const engine = await AttemptEngine.open(t.v, t.deps);
    t.server.online = false;
    await engine.setAnswer('q1', { selected: ['a'] });
    await engine.submit();
    engine.dispose();

    t.server.online = true;
    const reopened = await AttemptEngine.open(view(), t.deps);
    expect(reopened.state.phase).toBe('submitted_local');
    await reopened.flush();
    expect(reopened.state.phase).toBe('submitted');
  });

  it('does not loop on a submit the server does not confirm', async () => {
    const t = setup();
    t.server.refuseToConfirm = true;
    const engine = await AttemptEngine.open(t.v, t.deps);
    await engine.submit();
    expect(t.server.requests).toHaveLength(1);
    expect(engine.state.phase).toBe('submitted_local');
    expect(engine.state.lastError).toMatch(/did not confirm/);
    expect(engine.state.save).toBe('error');
  });

  it('ignores edits after submitting', async () => {
    const t = setup();
    const engine = await AttemptEngine.open(t.v, t.deps);
    await engine.submit();
    const before = t.server.requests.length;
    await engine.setAnswer('q1', { selected: ['a'] });
    await engine.flush();
    expect(t.server.requests.length).toBe(before);
  });

  it('closes when the server says the attempt is already finished', async () => {
    const t = setup();
    const engine = await AttemptEngine.open(t.v, t.deps);
    await engine.setAnswer('q1', { selected: ['a'] });
    t.server.status = 'submitted';
    await engine.flush();
    expect(engine.state.phase).toBe('submitted');
    expect(engine.state.pending).toBe(0);
  });

  it('submits automatically when the time is up', async () => {
    const t = setup();
    const engine = await AttemptEngine.open(t.v, t.deps);
    expect(await engine.autoSubmitIfExpired()).toBe(false);
    t.clock.advance(601_000);
    expect(await engine.autoSubmitIfExpired()).toBe(true);
    expect(engine.state.phase).toBe('submitted');
  });
});

describe('time cannot be gamed from the device clock', () => {
  it('uses the monotonic clock inside a session', async () => {
    const t = setup();
    const engine = await AttemptEngine.open(t.v, t.deps);
    t.clock.monoMs += 10_000;
    t.clock.wallMs -= 3_600_000; // the student winds the clock back an hour
    expect(Math.round(engine.elapsedMs() / 1000)).toBe(10);
    expect(Math.round((engine.remainingMs() ?? 0) / 1000)).toBe(590);
  });

  it('never gives time back after a reload', async () => {
    const t = setup();
    const first = await AttemptEngine.open(t.v, t.deps);
    t.clock.advance(100_000);
    await first.setAnswer('q1', { selected: ['a'] }); // persists the elapsed anchor
    first.dispose();

    t.clock.wallMs -= 3_600_000; // wind the clock back, then reload
    const sameView = view({
      server_now: new Date(START + 100_000).toISOString(),
    });
    const second = await AttemptEngine.open(sameView, t.deps);
    expect(second.elapsedMs()).toBeGreaterThanOrEqual(100_000);
  });

  it('measures the offset to the server clock', async () => {
    const t = setup();
    const skewed = view({ server_now: new Date(START + 5000).toISOString() });
    const engine = await AttemptEngine.open(skewed, t.deps);
    expect(engine.serverNowMs()).toBe(t.clock.wall() + 5000);
  });
});

describe('AttemptTimer', () => {
  it('after a reload resumes from the last elapsed time plus forward wall time only', () => {
    const clock = new FakeClock();
    const savedAt = clock.wall();
    clock.wallMs += 4000;
    const forward = new AttemptTimer(clock, 1000, savedAt, true);
    expect(forward.elapsedMs()).toBe(5000);

    const wound = new FakeClock();
    wound.wallMs -= 60_000; // the clock was wound back before the reload
    const refunded = new AttemptTimer(wound, 1000, START, true);
    expect(refunded.elapsedMs()).toBe(1000); // no refund
  });

  it('then follows the monotonic clock regardless of the device clock', () => {
    const clock = new FakeClock();
    const timer = new AttemptTimer(clock, 1000, clock.wall(), true);
    clock.monoMs += 3000;
    clock.wallMs -= 3_600_000;
    expect(timer.elapsedMs()).toBe(4000);
  });
});

describe('integrity signals', () => {
  it('ignores events unless the assessment monitors them', async () => {
    const t = setup();
    const engine = await AttemptEngine.open(t.v, t.deps);
    await engine.recordEvent('paste');
    expect(engine.state.pending).toBe(0);
  });

  it('records events and offline gaps when monitored', async () => {
    const t = setup({ settings: { records_integrity: true } });
    const engine = await AttemptEngine.open(t.v, t.deps);
    await engine.recordEvent('focus_lost', { reason: 'blur' });
    engine.setOnline(false);
    t.clock.advance(42_000);
    t.server.online = true;
    engine.setOnline(true);
    await engine.flush();

    const events = t.server.requests.flatMap((r) => r.events ?? []);
    expect(events.map((e) => e.kind)).toEqual([
      'focus_lost',
      'offline_start',
      'offline_end',
    ]);
    expect(events[2].meta).toEqual({ gap_secs: 42 });
  });
});
