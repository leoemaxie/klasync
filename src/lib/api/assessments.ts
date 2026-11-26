/**
 * Typed client for assessments (backend `assessments` module): lecture-linked
 * quizzes and homework.
 *
 * Lecturer routes live under `/assessments/{id}/...`; students use
 * `/students/...`. Types mirror `backend/src/assessments/model.rs`. Student
 * types deliberately have no answer-key fields, and grader types carry an
 * `alias` and never a student identity.
 */
import { apiRequest, getAccessToken, API_BASE } from './http';

// ---- shared vocabulary ----------------------------------------------------

export type QuestionType =
  'mcq_single' | 'mcq_multi' | 'true_false' | 'short_answer' | 'long_answer';
export type AssessmentKind = 'quiz' | 'homework' | 'practice';
export type AssessmentStatus =
  'draft' | 'published' | 'closed' | 'finalized' | 'archived';
export type Difficulty = 'easy' | 'medium' | 'hard';
export type LatePolicy = 'none' | 'flag' | 'penalty' | 'reject';
export type KeepPolicy = 'best' | 'last' | 'average';
export type IntegrityLevel = 'open' | 'monitored' | 'strict';
export type ScoringMode = 'manual' | 'ai_confirm' | 'ai_auto';
export type ReleasePolicy =
  'immediate' | 'after_due' | 'after_finalize' | 'manual';
export type Visibility = 'score' | 'correctness' | 'answers' | 'feedback';

export const QUESTION_TYPE_LABEL: Record<QuestionType, string> = {
  mcq_single: 'Single choice',
  mcq_multi: 'Multiple choice',
  true_false: 'True / false',
  short_answer: 'Short answer',
  long_answer: 'Long answer',
};

/** Per-assessment controls. Mirrors `AssessmentSettings`; every field has a default. */
export interface AssessmentSettings {
  late_policy: LatePolicy;
  late_penalty_pct_per_day: number;
  grace_secs: number;
  max_attempts: number;
  keep: KeepPolicy;
  cooldown_secs: number;
  time_limit_secs: number;
  shuffle_questions: boolean;
  shuffle_options: boolean;
  one_per_page: boolean;
  allow_backtracking: boolean;
  integrity: IntegrityLevel;
  require_fullscreen: boolean;
  block_paste: boolean;
  auto_submit_after_violations: number;
  allow_offline_continue: boolean;
  allow_offline_start: boolean;
  max_offline_gap_secs: number;
  scoring_mode: ScoringMode;
  ai_short_answer: boolean;
  negative_marking: boolean;
  release: ReleasePolicy;
  visibility: Visibility;
}

export const DEFAULT_SETTINGS: AssessmentSettings = {
  late_policy: 'flag',
  late_penalty_pct_per_day: 10,
  grace_secs: 30,
  max_attempts: 1,
  keep: 'best',
  cooldown_secs: 0,
  time_limit_secs: 0,
  shuffle_questions: false,
  shuffle_options: false,
  one_per_page: false,
  allow_backtracking: true,
  integrity: 'open',
  require_fullscreen: false,
  block_paste: false,
  auto_submit_after_violations: 0,
  allow_offline_continue: true,
  allow_offline_start: false,
  max_offline_gap_secs: 0,
  scoring_mode: 'ai_confirm',
  ai_short_answer: false,
  negative_marking: false,
  release: 'after_finalize',
  visibility: 'correctness',
};

export interface SourceRange {
  start_sec: number;
  end_sec: number;
}

// ---- lecturer: assessments and questions ----------------------------------------

export interface Assessment {
  id: string;
  course_id: string;
  session_id: string;
  lecturer_id: string;
  title: string;
  instructions_md: string;
  kind: AssessmentKind;
  status: AssessmentStatus;
  settings: AssessmentSettings;
  draft_rev: number;
  published_version_id: string | null;
  opens_at: string | null;
  due_at: string | null;
  closes_at: string | null;
  results_released_at: string | null;
  identity_revealed_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface McqOption {
  id: string;
  text: string;
}

export interface RubricCriterion {
  id: string;
  criterion: string;
  points: number;
  description?: string;
}

/** A draft question as the lecturer sees it (keys included). */
export interface Question {
  id: string;
  assessment_id: string;
  position: number;
  question_type: QuestionType;
  prompt_md: string;
  points: number;
  difficulty: Difficulty;
  topic_tag: string | null;
  chapter_index: number | null;
  public_payload: {
    options?: McqOption[];
    max_chars?: number;
    max_words?: number;
  };
  key_payload: {
    correct?: string[] | boolean;
    partial?: 'all_or_nothing' | 'proportional';
    accepted?: string[];
    case_sensitive?: boolean;
    ai_fallback?: boolean;
    rubric?: RubricCriterion[];
    reference?: string | null;
  };
  explanation_md: string;
  origin: 'lecturer' | 'ai' | 'ai_edited';
  source_ranges: SourceRange[];
  low_confidence: boolean;
  locked: boolean;
}

export interface QuestionInput {
  question_type: QuestionType;
  prompt_md: string;
  points?: number;
  difficulty?: Difficulty;
  topic_tag?: string | null;
  chapter_index?: number | null;
  public_payload?: Question['public_payload'];
  key_payload?: Question['key_payload'];
  explanation_md?: string;
  locked?: boolean;
}

export type QuestionPatch = Partial<Omit<QuestionInput, 'question_type'>>;

export interface AssessmentDetail {
  assessment: Assessment;
  settings: AssessmentSettings;
  questions: Question[];
  version: { id: string; version_no: number } | null;
  /** Edits made since the last publish. */
  unpublished_changes: boolean;
  attempts: number;
  submitted: number;
  pending_grades: number;
}

export interface AssessmentPatch {
  title?: string;
  instructions_md?: string;
  kind?: AssessmentKind;
  settings?: AssessmentSettings;
  /** `null` clears the date. */
  opens_at?: string | null;
  due_at?: string | null;
  closes_at?: string | null;
}

export interface LectureOption {
  id: string;
  title: string;
  short_code: string;
  status: string;
  started_at: string | null;
  has_transcript: boolean;
  assessments: number;
}

export interface GenerateSpec {
  count?: number;
  types?: QuestionType[];
  /** Share of easy / medium / hard. */
  difficulty_mix?: [number, number, number];
  topics?: string[];
  chapters?: number[];
  points_per_question?: number;
}

export interface AssessmentJob {
  id: string;
  job_type: 'assessment_generate' | 'assessment_grade';
  status: 'queued' | 'running' | 'completed' | 'failed' | 'cancelled';
  error: string | null;
  result: {
    added?: number;
    dropped_invalid?: number;
    duplicates?: number;
    solve_back_disagreements?: number;
  } | null;
  created_at: string;
}

export interface RosterEntry {
  student_account_id: string;
  display_name: string;
  matric_number: string;
  state: 'not_started' | 'in_progress' | 'submitted';
  attempts: number;
}

export interface Accommodation {
  student_account_id: string;
  display_name: string;
  matric_number: string;
  extra_time_pct: number;
  extra_attempts: number;
  due_override: string | null;
  reason: string;
}

const base = (id: string) => `/assessments/${id}`;
const json = (body: unknown) => JSON.stringify(body);

export const createAssessment = (
  shortCode: string,
  input: {
    title: string;
    kind?: AssessmentKind;
    instructions_md?: string;
  }
) =>
  apiRequest<Assessment>(`/sessions/code/${shortCode}/assessments`, {
    method: 'POST',
    body: json(input),
  });

export const listCourseAssessments = (courseId: string) =>
  apiRequest<Assessment[]>(`/courses/${courseId}/assessments`);

export const listCourseLectures = (courseId: string) =>
  apiRequest<LectureOption[]>(`/courses/${courseId}/assessment-lectures`);

export const getAssessment = (id: string) =>
  apiRequest<AssessmentDetail>(base(id));

export const patchAssessment = (
  id: string,
  baseRev: number,
  patch: AssessmentPatch
) =>
  apiRequest<{ draft_rev: number }>(base(id), {
    method: 'PATCH',
    body: json({ base_rev: baseRev, ...patch }),
  });

export const addQuestion = (id: string, input: QuestionInput) =>
  apiRequest<Question>(`${base(id)}/questions`, {
    method: 'POST',
    body: json(input),
  });

export const patchQuestion = (
  id: string,
  questionId: string,
  patch: QuestionPatch
) =>
  apiRequest<Question>(`${base(id)}/questions/${questionId}`, {
    method: 'PATCH',
    body: json(patch),
  });

export const deleteQuestion = (id: string, questionId: string) =>
  apiRequest<void>(`${base(id)}/questions/${questionId}`, {
    method: 'DELETE',
  });

export const reorderQuestions = (id: string, ids: string[]) =>
  apiRequest<void>(`${base(id)}/questions/reorder`, {
    method: 'POST',
    body: json({ ids }),
  });

export const generateQuestions = (id: string, spec: GenerateSpec) =>
  apiRequest<{ job_id: string }>(`${base(id)}/generate`, {
    method: 'POST',
    body: json(spec),
  });

export const listAssessmentJobs = (id: string) =>
  apiRequest<AssessmentJob[]>(`${base(id)}/jobs`);

export const publishAssessment = (id: string) =>
  apiRequest<{ version_id: string; version_no: number; new_version: boolean }>(
    `${base(id)}/publish`,
    { method: 'POST' }
  );

export const closeAssessment = (id: string) =>
  apiRequest<{ submitted_in_progress: number }>(`${base(id)}/close`, {
    method: 'POST',
  });

export const finalizeAssessment = (id: string) =>
  apiRequest<void>(`${base(id)}/finalize`, { method: 'POST' });

export const releaseResults = (id: string, release = true) =>
  apiRequest<{ results_released_at: string | null }>(`${base(id)}/release`, {
    method: 'POST',
    body: json({ release }),
  });

export const archiveAssessment = (id: string) =>
  apiRequest<void>(`${base(id)}/archive`, { method: 'POST' });

export const getRosterStatus = (id: string) =>
  apiRequest<RosterEntry[]>(`${base(id)}/roster-status`);

export const listAccommodations = (id: string) =>
  apiRequest<Accommodation[]>(`${base(id)}/accommodations`);

export const putAccommodation = (
  id: string,
  input: {
    student_account_id: string;
    extra_time_pct: number;
    extra_attempts: number;
    due_override?: string | null;
    reason: string;
  }
) =>
  apiRequest<void>(`${base(id)}/accommodations`, {
    method: 'PUT',
    body: json(input),
  });

export const deleteAccommodation = (id: string, studentId: string) =>
  apiRequest<void>(`${base(id)}/accommodations/${studentId}`, {
    method: 'DELETE',
  });

// ---- lecturer: blind grading -----------------------------------------------------------------

/** One answer as the grader sees it: an alias, never a student. */
export interface GraderAnswer {
  answer_id: string;
  alias: string;
  question_id: string;
  response: { text?: string; selected?: string[]; value?: boolean };
  score: number | null;
  max_score: number;
  grade_source: 'auto' | 'ai' | 'lecturer' | null;
  grade_status: 'proposed' | 'confirmed' | 'overridden' | 'rejected' | null;
  feedback_md: string | null;
  rubric_scores: { id: string; points: number; max: number; comment: string }[];
  confidence: number | null;
  proposal: {
    score: number;
    feedback_md: string;
    confidence: number | null;
    rubric_scores: GraderAnswer['rubric_scores'];
    evidence: { quotes?: string[]; flag?: string | null } | null;
    source: string;
  } | null;
}

export interface QueueQuestion {
  id: string;
  position: number;
  question_type: QuestionType;
  prompt_md: string;
  points: number;
  rubric: RubricCriterion[];
  reference: unknown;
  pending: number;
}

export interface GradingQueue {
  answers: GraderAnswer[];
  pending: number;
  total: number;
  questions: QueueQuestion[];
}

export const getGradingQueue = (
  id: string,
  opts: { question?: string; pendingOnly?: boolean } = {}
) => {
  const params = new URLSearchParams();
  if (opts.question) params.set('question', opts.question);
  if (opts.pendingOnly) params.set('pending_only', 'true');
  const qs = params.toString();
  return apiRequest<GradingQueue>(
    `${base(id)}/grading/queue${qs ? `?${qs}` : ''}`
  );
};

export const putGrade = (
  id: string,
  answerId: string,
  grade: { score: number; feedback_md?: string; rubric_scores?: unknown }
) =>
  apiRequest<void>(`${base(id)}/grading/answers/${answerId}`, {
    method: 'PUT',
    body: json(grade),
  });

export const confirmGrade = (id: string, answerId: string) =>
  apiRequest<void>(`${base(id)}/grading/answers/${answerId}/confirm`, {
    method: 'POST',
  });

export const confirmAllGrades = (id: string, minConfidence = 0.8) =>
  apiRequest<{ confirmed: number }>(`${base(id)}/grading/confirm-all`, {
    method: 'POST',
    body: json({ min_confidence: minConfidence }),
  });

export const runAiGrading = (id: string, questionId?: string) =>
  apiRequest<{ job_id: string | null; already_running: boolean }>(
    `${base(id)}/grading/ai-run`,
    { method: 'POST', body: json({ question_id: questionId ?? null }) }
  );

// ---- lecturer: reports ------------------------------------------------------------------------------

export interface ItemStat {
  question_id: string;
  position: number;
  prompt: string;
  topic_tag: string | null;
  attempts: number;
  difficulty: number;
  discrimination: number | null;
  skip_rate: number;
  avg_time_secs: number;
  options: {
    option: string;
    picked: number;
    share: number;
    correct: boolean;
    top_share: number;
  }[];
  flags: (
    'too_hard' | 'too_easy' | 'non_discriminating' | 'possibly_miskeyed'
  )[];
}

export interface AssessmentReport {
  assessment_id: string;
  provisional: boolean;
  pending_grades: number;
  funnel: {
    assigned: number;
    started: number;
    submitted: number;
    graded: number;
  };
  summary: {
    n: number;
    mean: number;
    median: number;
    std_dev: number;
    min: number;
    max: number;
    pass_rate: number;
  } | null;
  histogram: { from: number; to: number; count: number }[];
  items: ItemStat[];
  topics: {
    topic: string;
    chapter_index: number | null;
    questions: number;
    mastery: number;
  }[];
  integrity: {
    attempts_with_flags: number;
    offline_submits: number;
    late_submits: number;
  };
}

export const getAssessmentReport = (id: string) =>
  apiRequest<AssessmentReport>(`${base(id)}/report`);

export interface StudentScoreRow {
  student_account_id: string;
  display_name: string;
  matric_number: string;
  attempts: number;
  percent: number | null;
  late: boolean;
}

/** Named results; refused until the assessment is finalised. */
export const getNamedResults = (id: string) =>
  apiRequest<StudentScoreRow[]>(`${base(id)}/results`);

export interface IntegrityRow {
  alias: string;
  status: string;
  late: boolean;
  submitted_offline: boolean;
  violations: number;
  events: Record<string, number>;
}

export const getIntegrity = (id: string) =>
  apiRequest<IntegrityRow[]>(`${base(id)}/integrity`);

/** Authenticated download of the named results CSV. */
export async function downloadResultsCsv(id: string): Promise<Blob> {
  const token = getAccessToken();
  const response = await fetch(`${API_BASE}${base(id)}/export.csv`, {
    credentials: 'include',
    headers: token ? { authorization: `Bearer ${token}` } : {},
  });
  if (!response.ok) {
    const payload = (await response.json().catch(() => ({}))) as {
      error?: string;
    };
    throw new Error(payload.error ?? 'Could not export the results');
  }
  return response.blob();
}

// ---- student ----------------------------------------------------------------------------------------------

export type StudentAssessmentState =
  'upcoming' | 'open' | 'in_progress' | 'submitted' | 'closed';

export interface StudentAssessment {
  id: string;
  title: string;
  kind: AssessmentKind;
  status: AssessmentStatus;
  course_id: string;
  course_code: string;
  course_title: string;
  session_id: string;
  lecture_title: string;
  opens_at: string | null;
  due_at: string | null;
  closes_at: string | null;
  time_limit_secs: number;
  max_attempts: number;
  attempts_used: number;
  in_progress_attempt_id: string | null;
  latest_attempt_id: string | null;
  state: StudentAssessmentState;
  can_start: boolean;
  results_available: boolean;
}

export interface StudentOption {
  id: string;
  text: string;
}

/** A question as a student receives it: there is no answer key here. */
export interface StudentQuestion {
  id: string;
  position: number;
  question_type: QuestionType;
  prompt_md: string;
  points: number;
  options: StudentOption[];
  max_chars: number | null;
  max_words: number | null;
}

export interface StudentSettings {
  time_limit_secs: number;
  grace_secs: number;
  one_per_page: boolean;
  allow_backtracking: boolean;
  require_fullscreen: boolean;
  block_paste: boolean;
  records_integrity: boolean;
  allow_offline_continue: boolean;
  allow_offline_start: boolean;
  max_offline_gap_secs: number;
  auto_submit_after_violations: number;
  max_attempts: number;
}

export type AnswerResponse = {
  selected?: string[];
  value?: boolean;
  text?: string;
};

export interface SavedAnswer {
  question_id: string;
  response: AnswerResponse;
  revision: number;
  flagged_for_review: boolean;
}

export interface AttemptView {
  attempt_id: string;
  assessment_id: string;
  title: string;
  instructions_md: string;
  kind: AssessmentKind;
  attempt_no: number;
  status: 'in_progress' | 'submitted' | 'auto_submitted' | 'void' | 'graded';
  started_at: string;
  deadline_at: string | null;
  /** Server time when this was built; used to measure the clock offset. */
  server_now: string;
  settings: StudentSettings;
  questions: StudentQuestion[];
  answers: SavedAnswer[];
  receipt: string | null;
  submitted_at: string | null;
  auto_submitted: boolean;
}

export interface AnswerOp {
  op_id: string;
  question_id: string;
  response: AnswerResponse;
  revision: number;
  time_spent_ms?: number;
  flagged?: boolean;
}

export interface ClientEvent {
  kind: string;
  client_at?: string;
  meta?: Record<string, unknown>;
}

export interface SubmitClaim {
  /** Monotonic milliseconds from the server-issued start. */
  elapsed_ms?: number;
  claimed_at?: string;
  offline?: boolean;
}

export interface SyncRequest {
  device_id?: string;
  ops?: AnswerOp[];
  events?: ClientEvent[];
  submit?: SubmitClaim;
}

export interface SyncResponse {
  applied: number;
  duplicates: number;
  rejected: number;
  status: AttemptView['status'];
  deadline_at: string | null;
  server_now: string;
  receipt: string | null;
  flagged: boolean;
  auto_submitted: boolean;
}

export const getStudentAssessments = () =>
  apiRequest<StudentAssessment[]>('/students/assessments');

export const startAttempt = (assessmentId: string, deviceId?: string) =>
  apiRequest<AttemptView>(`/students/assessments/${assessmentId}/attempts`, {
    method: 'POST',
    body: json({ device_id: deviceId ?? null }),
  });

export const getAttempt = (attemptId: string) =>
  apiRequest<AttemptView>(`/students/attempts/${attemptId}`);

export const syncAttempt = (attemptId: string, request: SyncRequest) =>
  apiRequest<SyncResponse>(`/students/attempts/${attemptId}/sync`, {
    method: 'POST',
    body: json(request),
  });

export interface ResultQuestion {
  id: string;
  position: number;
  question_type: QuestionType;
  prompt_md: string;
  points: number;
  options: StudentOption[];
  response: AnswerResponse;
  score?: number | null;
  correct?: boolean | null;
  correct_answer?: unknown;
  feedback_md?: string;
  explanation_md?: string;
  source_ranges: SourceRange[];
  ai_assisted: boolean;
}

export interface StudentResult {
  attempt_id: string;
  assessment_id: string;
  title: string;
  attempt_no: number;
  status: string;
  submitted_at: string | null;
  late: boolean;
  penalty_pct: number;
  score: number | null;
  max_score: number;
  percent: number | null;
  complete: boolean;
  visibility: Visibility;
  ai_assisted: boolean;
  questions: ResultQuestion[];
}

export const getAttemptResult = (attemptId: string) =>
  apiRequest<StudentResult>(`/students/attempts/${attemptId}/result`);

export interface StudentProgress {
  assessments: {
    assessment_id: string;
    title: string;
    kind: AssessmentKind;
    submitted_at: string | null;
    percent: number | null;
    attempts: number;
    late: boolean;
  }[];
  topics: {
    topic: string;
    chapter_index: number | null;
    questions: number;
    mastery: number;
  }[];
}

export const getAssessmentProgress = (courseId: string) =>
  apiRequest<StudentProgress>(
    `/students/courses/${courseId}/assessment-progress`
  );
