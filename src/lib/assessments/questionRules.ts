/**
 * Client-side mirror of the backend's question validation
 * (`validate_question` in `backend/src/assessments/model.rs`), so a lecturer
 * sees what blocks publishing while editing. The server stays the authority.
 */
import type {
  McqOption,
  Question,
  RubricCriterion,
} from '$lib/api/assessments';

/** The next unused option id: a, b, ... z, then a1, b1, ... */
export function nextOptionId(options: McqOption[]): string {
  const used = new Set(options.map((o) => o.id));
  for (let i = 0; i < 260; i++) {
    const letter = String.fromCharCode(97 + (i % 26));
    const id = i < 26 ? letter : `${letter}${Math.floor(i / 26)}`;
    if (!used.has(id)) return id;
  }
  return `o${options.length + 1}`;
}

export function rubricTotal(rubric: RubricCriterion[] | undefined): number {
  return (rubric ?? []).reduce((sum, c) => sum + (Number(c.points) || 0), 0);
}

/** A plain-language reason the question cannot be published yet, or `null`. */
export function questionProblem(q: Question): string | null {
  if (!q.prompt_md.trim()) return 'Add the question text.';
  if (!(q.points >= 0 && q.points <= 1000))
    return 'Points must be between 0 and 1000.';
  const key = q.key_payload ?? {};
  switch (q.question_type) {
    case 'mcq_single':
    case 'mcq_multi': {
      const options = q.public_payload?.options ?? [];
      if (options.length < 2) return 'Add at least two options.';
      if (options.length > 10) return 'Use at most ten options.';
      if (options.some((o) => !o.text.trim())) return 'Fill in every option.';
      const correct = Array.isArray(key.correct) ? key.correct : [];
      if (correct.length === 0) return 'Mark the correct option.';
      if (q.question_type === 'mcq_single' && correct.length !== 1) {
        return 'A single-choice question has exactly one correct option.';
      }
      if (correct.some((id) => !options.some((o) => o.id === id))) {
        return 'The correct answer must be one of the options.';
      }
      return null;
    }
    case 'true_false':
      return typeof key.correct === 'boolean' ? null : 'Choose true or false.';
    case 'short_answer':
      return (key.accepted ?? []).some((a) => !a.trim())
        ? 'Accepted answers cannot be empty.'
        : null;
    case 'long_answer': {
      const rubric = key.rubric ?? [];
      if (
        rubric.length > 0 &&
        Math.abs(rubricTotal(rubric) - q.points) > 0.01
      ) {
        return `The rubric adds up to ${rubricTotal(rubric)} points but the question is worth ${q.points}.`;
      }
      return null;
    }
  }
}

/** What stops the whole assessment from publishing, in order. */
export function publishProblems(questions: Question[]): string[] {
  if (questions.length === 0) return ['Add at least one question.'];
  const problems: string[] = [];
  questions.forEach((q, i) => {
    const p = questionProblem(q);
    if (p) problems.push(`Question ${i + 1}: ${p}`);
  });
  if (questions.every((q) => q.points <= 0)) {
    problems.push('The questions must be worth some points.');
  }
  return problems;
}

/** `datetime-local` input value for an ISO instant (in the user's time zone). */
export function toLocalInput(iso: string | null): string {
  if (!iso) return '';
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '';
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** The inverse: an ISO instant from a `datetime-local` value, or `null` when empty. */
export function fromLocalInput(value: string): string | null {
  if (!value) return null;
  const d = new Date(value);
  return Number.isNaN(d.getTime()) ? null : d.toISOString();
}
