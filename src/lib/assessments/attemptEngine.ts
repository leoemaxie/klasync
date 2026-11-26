/**
 * The offline-safe engine behind taking an assessment.
 *
 * Principle: once an attempt has started, a student's work cannot be lost to a
 * dropped connection. Every change is written to local storage *before* any
 * network call, sent in idempotent batches (each change has a unique id the
 * server remembers), retried with backoff, and replayed safely after a reload.
 * Submitting offline queues the submit with the device's measured elapsed time
 * so the server can accept it, flagged for the lecturer, if it fits the deadline.
 *
 * The engine knows nothing about Svelte, `fetch` or IndexedDB: those are
 * injected, which is what makes every rule here testable.
 */
import type {
  AnswerOp,
  AnswerResponse,
  AttemptView,
  ClientEvent,
  SubmitClaim,
  SyncRequest,
  SyncResponse,
} from '$lib/api/assessments';

// ---- storage -----------------------------------------------------------------------

export interface LocalAnswer {
  response: AnswerResponse;
  revision: number;
  flagged: boolean;
  timeSpentMs: number;
}

/** Everything needed to resume an attempt with no network. */
export interface StoredAttempt {
  /** The attempt id; the storage key. */
  id: string;
  /** The cached question set (it never contains an answer key). */
  view: AttemptView;
  answers: Record<string, LocalAnswer>;
  /** Changes not yet confirmed by the server, oldest first. */
  pendingOps: AnswerOp[];
  pendingEvents: ClientEvent[];
  /** A submit waiting to be delivered. */
  queuedSubmit: SubmitClaim | null;
  phase: Phase;
  receipt: string | null;
  deviceId: string;
  /** Milliseconds the attempt had run at `anchorWall`, for clock recovery after a reload. */
  anchorElapsedMs: number;
  anchorWall: number;
  /** Server time minus device time when the attempt was last seen by the server. */
  serverOffsetMs: number;
  deadlineAt: string | null;
  flagged: boolean;
  autoSubmitted: boolean;
  updatedAt: number;
}

export interface AttemptStore {
  load(id: string): Promise<StoredAttempt | undefined>;
  save(attempt: StoredAttempt): Promise<void>;
  remove(id: string): Promise<void>;
}

/** A store that keeps everything in memory (tests, or when IndexedDB is unavailable). */
export class MemoryAttemptStore implements AttemptStore {
  private items = new Map<string, StoredAttempt>();
  async load(id: string) {
    const found = this.items.get(id);
    return found ? structuredClone(found) : undefined;
  }
  async save(attempt: StoredAttempt) {
    this.items.set(attempt.id, structuredClone(attempt));
  }
  async remove(id: string) {
    this.items.delete(id);
  }
}

// ---- time ----------------------------------------------------------------------------

export interface Clock {
  /** Wall-clock milliseconds (Date.now). Can be changed by the user. */
  wall(): number;
  /** Monotonic milliseconds (performance.now). Cannot be wound back. */
  mono(): number;
}

export const systemClock: Clock = {
  wall: () => Date.now(),
  mono: () =>
    typeof performance !== 'undefined' ? performance.now() : Date.now(),
};

/**
 * Measures how long an attempt has run. The starting point is fixed once, when
 * the timer is created: from the server's view of elapsed time, or after a
 * reload from the last recorded elapsed time plus the wall time since (never
 * going backwards, so winding the device clock back refunds nothing). From
 * then on it uses the monotonic clock, which the device clock cannot affect.
 */
export class AttemptTimer {
  private readonly baseElapsedMs: number;
  private readonly monoAtStart: number;
  constructor(
    private clock: Clock,
    anchorElapsedMs: number,
    anchorWall: number,
    /** True when the anchor was saved by an earlier page session (a reload). */
    resumed: boolean
  ) {
    this.baseElapsedMs = resumed
      ? anchorElapsedMs + Math.max(0, clock.wall() - anchorWall)
      : anchorElapsedMs;
    this.monoAtStart = clock.mono();
  }

  elapsedMs(): number {
    return this.baseElapsedMs + (this.clock.mono() - this.monoAtStart);
  }
}

// ---- engine ----------------------------------------------------------------------------

export type Phase =
  /** Answering. */
  | 'in_progress'
  /** Submitted on this device; waiting to reach the server. */
  | 'submitted_local'
  /** The server has the submission and issued a receipt. */
  | 'submitted';

export type SaveState =
  /** Every change is on this device and on the server. */
  | 'synced'
  /** On this device, on its way to the server. */
  | 'saving'
  /** On this device; the network is unavailable. */
  | 'offline'
  /** On this device; the server refused or something is wrong. */
  | 'error';

export interface EngineState {
  phase: Phase;
  save: SaveState;
  online: boolean;
  pending: number;
  receipt: string | null;
  /** The server closed the attempt (time ran out or a limit was reached). */
  autoSubmitted: boolean;
  /** Accepted after the deadline on the strength of an offline claim. */
  flagged: boolean;
  lastError: string | null;
  lastSyncedAt: number | null;
}

export interface EngineDeps {
  store: AttemptStore;
  sync: (attemptId: string, request: SyncRequest) => Promise<SyncResponse>;
  clock?: Clock;
  /** Schedules a retry; returns a cancel function. Defaults to setTimeout. */
  schedule?: (fn: () => void, ms: number) => () => void;
  newId?: () => string;
  deviceId?: string;
  /** Changes sent per request. */
  batchSize?: number;
  /** Delay before a change is sent, so typing is batched. */
  debounceMs?: number;
}

const defaultSchedule = (fn: () => void, ms: number) => {
  const handle = setTimeout(fn, ms);
  return () => clearTimeout(handle);
};

const defaultId = () =>
  typeof crypto !== 'undefined' && 'randomUUID' in crypto
    ? crypto.randomUUID()
    : 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
        const r = (Math.random() * 16) | 0;
        return (c === 'x' ? r : (r & 0x3) | 0x8).toString(16);
      });

/** A failed `fetch` (no network) is a TypeError; HTTP errors are plain Errors. */
export const isNetworkError = (error: unknown) =>
  error instanceof TypeError ||
  (error instanceof Error &&
    /network|failed to fetch|offline/i.test(error.message));

const BACKOFF_MS = [1000, 2000, 4000, 8000, 15000, 30000];

export class AttemptEngine {
  readonly attemptId: string;
  private stored: StoredAttempt;
  private deps: Required<EngineDeps>;
  private timer: AttemptTimer;
  private listeners = new Set<(state: EngineState) => void>();
  private cancelRetry: (() => void) | null = null;
  private cancelDebounce: (() => void) | null = null;
  private syncing = false;
  private dirtyWhileSyncing = false;
  private failures = 0;
  private online = true;
  private offlineSince: number | null = null;
  private lastError: string | null = null;
  private lastSyncedAt: number | null = null;
  private disposed = false;

  private constructor(
    stored: StoredAttempt,
    deps: EngineDeps,
    resumed: boolean
  ) {
    this.stored = stored;
    this.attemptId = stored.id;
    this.deps = {
      clock: systemClock,
      schedule: defaultSchedule,
      newId: defaultId,
      deviceId: stored.deviceId,
      batchSize: 50,
      debounceMs: 1500,
      ...deps,
    };
    this.timer = new AttemptTimer(
      this.deps.clock,
      stored.anchorElapsedMs,
      stored.anchorWall,
      resumed
    );
  }

  /**
   * Opens an attempt from a view the server just sent. Anything saved locally
   * for this attempt is merged in: local changes the server has not seen win
   * (they are still pending), so a reload never loses work.
   */
  static async open(
    view: AttemptView,
    deps: EngineDeps
  ): Promise<AttemptEngine> {
    const clock = deps.clock ?? systemClock;
    const deviceId = deps.deviceId ?? (deps.newId ?? defaultId)();
    const existing = await deps.store.load(view.attempt_id);
    const serverElapsed = Math.max(
      0,
      Date.parse(view.server_now) - Date.parse(view.started_at)
    );
    const offset = Date.parse(view.server_now) - clock.wall();

    const answers: Record<string, LocalAnswer> = {};
    for (const saved of view.answers) {
      answers[saved.question_id] = {
        response: saved.response,
        revision: saved.revision,
        flagged: saved.flagged_for_review,
        timeSpentMs: 0,
      };
    }
    let base: StoredAttempt = {
      id: view.attempt_id,
      view,
      answers,
      pendingOps: [],
      pendingEvents: [],
      queuedSubmit: null,
      phase: view.status === 'in_progress' ? 'in_progress' : 'submitted',
      receipt: view.receipt,
      deviceId,
      anchorElapsedMs: serverElapsed,
      anchorWall: clock.wall(),
      serverOffsetMs: offset,
      deadlineAt: view.deadline_at,
      flagged: false,
      autoSubmitted: view.auto_submitted,
      updatedAt: clock.wall(),
    };
    if (existing && existing.phase !== 'submitted') {
      // Keep the work the server has not confirmed: pending ops, queued submit, newer revisions.
      for (const [qid, local] of Object.entries(existing.answers)) {
        const remote = base.answers[qid];
        if (!remote || local.revision >= remote.revision)
          base.answers[qid] = local;
      }
      base = {
        ...base,
        pendingOps: existing.pendingOps,
        pendingEvents: existing.pendingEvents,
        queuedSubmit: existing.queuedSubmit,
        phase: existing.phase,
        deviceId: existing.deviceId,
        flagged: existing.flagged,
        // Never let elapsed time shrink across a reload.
        anchorElapsedMs: Math.max(
          serverElapsed,
          existing.anchorElapsedMs +
            Math.max(0, clock.wall() - existing.anchorWall)
        ),
      };
      // The server says it is finished: nothing local can change that.
      if (view.status !== 'in_progress') {
        base.phase = 'submitted';
        base.pendingOps = [];
        base.queuedSubmit = null;
      }
    }
    const engine = new AttemptEngine({ ...base }, deps, false);
    await engine.persist();
    if (engine.hasUnsent()) engine.scheduleSync(0);
    return engine;
  }

  /** Restores an attempt purely from local storage (no network, e.g. after a reload offline). */
  static async restore(
    attemptId: string,
    deps: EngineDeps
  ): Promise<AttemptEngine | undefined> {
    const stored = await deps.store.load(attemptId);
    if (!stored) return undefined;
    const engine = new AttemptEngine(stored, deps, true);
    if (engine.hasUnsent()) engine.scheduleSync(0);
    return engine;
  }

  // ---- state ----

  get view(): AttemptView {
    return this.stored.view;
  }

  get state(): EngineState {
    const pending =
      this.stored.pendingOps.length +
      this.stored.pendingEvents.length +
      (this.stored.queuedSubmit ? 1 : 0);
    let save: SaveState = 'synced';
    if (pending > 0) {
      if (this.lastError && this.online) save = 'error';
      else if (!this.online) save = 'offline';
      else save = 'saving';
    }
    return {
      phase: this.stored.phase,
      save,
      online: this.online,
      pending,
      receipt: this.stored.receipt,
      autoSubmitted: this.stored.autoSubmitted,
      flagged: this.stored.flagged,
      lastError: this.lastError,
      lastSyncedAt: this.lastSyncedAt,
    };
  }

  subscribe(listener: (state: EngineState) => void): () => void {
    this.listeners.add(listener);
    listener(this.state);
    return () => this.listeners.delete(listener);
  }

  answerFor(questionId: string): LocalAnswer | undefined {
    return this.stored.answers[questionId];
  }

  /** The device's best estimate of server time, from the last measured offset. */
  serverNowMs(): number {
    return this.deps.clock.wall() + this.stored.serverOffsetMs;
  }

  elapsedMs(): number {
    return this.timer.elapsedMs();
  }

  /** Milliseconds left, from elapsed time and the server's deadline. `null` = no deadline. */
  remainingMs(): number | null {
    if (!this.stored.deadlineAt) return null;
    const startedAt = Date.parse(this.stored.view.started_at);
    const total = Date.parse(this.stored.deadlineAt) - startedAt;
    return total - this.timer.elapsedMs();
  }

  private emit() {
    const state = this.state;
    for (const listener of this.listeners) listener(state);
  }

  private hasUnsent(): boolean {
    return (
      this.stored.pendingOps.length > 0 ||
      this.stored.pendingEvents.length > 0 ||
      this.stored.queuedSubmit !== null
    );
  }

  private async persist() {
    this.stored.anchorElapsedMs = this.timer.elapsedMs();
    this.stored.anchorWall = this.deps.clock.wall();
    this.stored.updatedAt = this.deps.clock.wall();
    try {
      await this.deps.store.save(this.stored);
    } catch {
      // Storage full or blocked: the in-memory copy still syncs; surface a warning.
      this.lastError = 'Could not save on this device';
    }
  }

  // ---- changes ----

  /** Records an answer. It is on this device before this promise resolves. */
  async setAnswer(
    questionId: string,
    response: AnswerResponse,
    timeSpentMs = 0
  ): Promise<void> {
    if (this.stored.phase !== 'in_progress') return;
    const previous = this.stored.answers[questionId];
    const revision = (previous?.revision ?? 0) + 1;
    const spent = (previous?.timeSpentMs ?? 0) + timeSpentMs;
    const flagged = previous?.flagged ?? false;
    this.stored.answers[questionId] = {
      response,
      revision,
      flagged,
      timeSpentMs: spent,
    };
    this.pushOp({
      op_id: this.deps.newId(),
      question_id: questionId,
      response,
      revision,
      time_spent_ms: spent,
      flagged,
    });
    await this.persist();
    this.emit();
    this.scheduleSync(this.deps.debounceMs);
  }

  async setFlag(questionId: string, flagged: boolean): Promise<void> {
    if (this.stored.phase !== 'in_progress') return;
    const previous = this.stored.answers[questionId];
    const revision = (previous?.revision ?? 0) + 1;
    this.stored.answers[questionId] = {
      response: previous?.response ?? {},
      revision,
      flagged,
      timeSpentMs: previous?.timeSpentMs ?? 0,
    };
    this.pushOp({
      op_id: this.deps.newId(),
      question_id: questionId,
      response: previous?.response ?? {},
      revision,
      time_spent_ms: previous?.timeSpentMs ?? 0,
      flagged,
    });
    await this.persist();
    this.emit();
    this.scheduleSync(this.deps.debounceMs);
  }

  /** A newer change to the same question replaces an unsent older one, so a typing burst is one op. */
  private pushOp(op: AnswerOp) {
    const ops = this.stored.pendingOps;
    const index = ops.findIndex((o) => o.question_id === op.question_id);
    if (index >= 0 && !this.inFlight.has(ops[index].op_id))
      ops.splice(index, 1);
    ops.push(op);
  }

  /** Records an integrity signal (focus lost, paste, ...) when the assessment monitors them. */
  async recordEvent(kind: string, meta: Record<string, unknown> = {}) {
    if (this.stored.phase !== 'in_progress') return;
    if (!this.stored.view.settings.records_integrity) return;
    this.stored.pendingEvents.push({
      kind,
      client_at: new Date(
        this.deps.clock.wall() + this.stored.serverOffsetMs
      ).toISOString(),
      meta,
    });
    await this.persist();
    this.emit();
    this.scheduleSync(this.deps.debounceMs);
  }

  /** Tells the engine the device's connectivity changed (browser online/offline events). */
  setOnline(online: boolean) {
    if (online === this.online) return;
    this.markOnline(online);
    if (online && this.hasUnsent()) this.scheduleSync(0);
    this.emit();
  }

  private markOnline(online: boolean) {
    if (online === this.online) return;
    const now = this.deps.clock.wall();
    if (!online) {
      this.offlineSince = now;
      this.online = false;
      if (this.stored.view.settings.records_integrity) {
        this.stored.pendingEvents.push({
          kind: 'offline_start',
          client_at: new Date(now + this.stored.serverOffsetMs).toISOString(),
        });
      }
    } else {
      const gap = this.offlineSince
        ? Math.round((now - this.offlineSince) / 1000)
        : 0;
      this.online = true;
      this.offlineSince = null;
      if (this.stored.view.settings.records_integrity) {
        this.stored.pendingEvents.push({
          kind: 'offline_end',
          client_at: new Date(now + this.stored.serverOffsetMs).toISOString(),
          meta: { gap_secs: gap },
        });
      }
    }
  }

  // ---- submit ----

  /**
   * Submits. With no network the submit is queued with the elapsed time this
   * device measured; it is delivered when the connection returns.
   */
  async submit(): Promise<void> {
    if (this.stored.phase !== 'in_progress') return;
    const wall = this.deps.clock.wall() + this.stored.serverOffsetMs;
    this.stored.queuedSubmit = {
      elapsed_ms: Math.round(this.timer.elapsedMs()),
      claimed_at: new Date(wall).toISOString(),
      offline: !this.online,
    };
    this.stored.phase = 'submitted_local';
    await this.persist();
    this.emit();
    this.cancelDebounce?.();
    await this.flush();
  }

  /** Submits automatically once the time is up. Returns whether it submitted. */
  async autoSubmitIfExpired(): Promise<boolean> {
    const left = this.remainingMs();
    if (left !== null && left <= 0 && this.stored.phase === 'in_progress') {
      await this.submit();
      return true;
    }
    return false;
  }

  // ---- syncing ----

  private scheduleSync(delay: number) {
    if (this.disposed) return;
    this.cancelDebounce?.();
    this.cancelDebounce = this.deps.schedule(() => void this.flush(), delay);
  }

  private inFlight = new Set<string>();

  /** Sends everything unsent. Resolves when the queue is empty or a send fails. */
  async flush(): Promise<void> {
    if (this.disposed) return;
    if (this.syncing) {
      this.dirtyWhileSyncing = true;
      return;
    }
    this.syncing = true;
    try {
      do {
        this.dirtyWhileSyncing = false;
        while (this.hasUnsent()) {
          const ok = await this.sendOnce();
          if (!ok) return;
        }
      } while (this.dirtyWhileSyncing);
    } finally {
      this.syncing = false;
    }
  }

  private async sendOnce(): Promise<boolean> {
    const ops = this.stored.pendingOps.slice(0, this.deps.batchSize);
    const events = this.stored.pendingEvents.slice(0, this.deps.batchSize);
    // The submit rides along only once every change has been sent.
    const lastBatch =
      ops.length === this.stored.pendingOps.length &&
      events.length === this.stored.pendingEvents.length;
    const submit = lastBatch
      ? (this.stored.queuedSubmit ?? undefined)
      : undefined;
    ops.forEach((o) => this.inFlight.add(o.op_id));
    const request: SyncRequest = {
      device_id: this.deps.deviceId,
      ops,
      events,
      ...(submit ? { submit } : {}),
    };
    try {
      const response = await this.deps.sync(this.attemptId, request);
      this.onSuccess(ops, events, submit, response);
      await this.persist();
      this.emit();
      return true;
    } catch (error) {
      ops.forEach((o) => this.inFlight.delete(o.op_id));
      if (isNetworkError(error)) {
        this.markOnline(false);
        this.lastError = null;
        this.scheduleRetry();
      } else {
        // The server answered with an error. Keep the data and retry slowly: it may be transient.
        this.lastError =
          error instanceof Error ? error.message : 'Could not sync';
        this.scheduleRetry();
      }
      await this.persist();
      this.emit();
      return false;
    }
  }

  private onSuccess(
    ops: AnswerOp[],
    events: ClientEvent[],
    submit: SubmitClaim | undefined,
    response: SyncResponse
  ) {
    const sent = new Set(ops.map((o) => o.op_id));
    this.stored.pendingOps = this.stored.pendingOps.filter(
      (o) => !sent.has(o.op_id)
    );
    ops.forEach((o) => this.inFlight.delete(o.op_id));
    this.stored.pendingEvents = this.stored.pendingEvents.slice(events.length);
    this.failures = 0;
    this.lastError = null;
    this.markOnline(true);
    this.lastSyncedAt = this.deps.clock.wall();
    this.stored.serverOffsetMs =
      Date.parse(response.server_now) - this.deps.clock.wall();
    if (response.deadline_at) this.stored.deadlineAt = response.deadline_at;
    if (response.flagged) this.stored.flagged = true;
    if (response.auto_submitted) this.stored.autoSubmitted = true;
    // The server closed the attempt: nothing more can be sent for it.
    const finished =
      response.status !== 'in_progress' || response.receipt !== null;
    if (submit && !finished) {
      // Never loop on a submit the server did not confirm; surface it and retry with backoff.
      throw new Error('The server did not confirm the submission');
    }
    if (finished) {
      this.stored.queuedSubmit = null;
      this.stored.pendingOps = [];
      this.stored.pendingEvents = [];
      this.stored.receipt = response.receipt;
      this.stored.phase = 'submitted';
    }
  }

  private scheduleRetry() {
    if (this.disposed) return;
    this.cancelRetry?.();
    const delay = BACKOFF_MS[Math.min(this.failures, BACKOFF_MS.length - 1)];
    this.failures += 1;
    this.cancelRetry = this.deps.schedule(() => void this.flush(), delay);
  }

  dispose() {
    this.disposed = true;
    this.cancelRetry?.();
    this.cancelDebounce?.();
    this.listeners.clear();
  }
}
